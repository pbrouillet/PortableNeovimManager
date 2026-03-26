use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

pub mod terminal;
pub mod wt;
pub mod alacritty;
#[cfg(not(target_os = "windows"))]
pub mod kitty;
#[cfg(target_os = "linux")]
pub mod gnome_terminal;
#[cfg(target_os = "macos")]
pub mod iterm2;
#[cfg(target_os = "linux")]
pub mod konsole;

// Re-exports for backward compatibility and convenience
pub use terminal::{
    TerminalInstallation, TerminalKind, TerminalProfile, find_terminals,
    apply_font_to_defaults as apply_terminal_font_to_defaults,
    apply_font_to_profiles as apply_terminal_font_to_profiles,
    is_any_terminal_configured,
};
pub use wt::{
    WtInstallation, WtProfile, apply_font_to_wt_defaults, apply_font_to_wt_profiles,
    find_wt_installations, is_wt_configured,
};

pub const NERD_FONT_NAME: &str = "JetBrainsMono";
pub const NERD_FONT_FACE: &str = "JetBrainsMono Nerd Font";
const NERD_FONT_VERSION: &str = "v3.3.0";
const NERD_FONT_ZIP_URL: &str =
    "https://github.com/ryanoasis/nerd-fonts/releases/download/v3.3.0/JetBrainsMono.zip";

// ── Platform font directory ─────────────────────────────────────────────────

/// Returns the platform-specific user fonts directory.
fn fonts_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA").ok().map(|appdata| {
            PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts")
        })
    }

    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| h.join("Library").join("Fonts"))
    }

    #[cfg(target_os = "linux")]
    {
        dirs::home_dir().map(|h| h.join(".local").join("share").join("fonts"))
    }
}

// ── Font installation ───────────────────────────────────────────────────────

/// Result of a font install operation.
pub struct FontInstallResult {
    pub installed_count: u32,
    pub already_installed: bool,
    pub dest_dir: PathBuf,
    pub message: String,
}

/// Downloads and installs JetBrainsMono Nerd Font.
pub async fn install_nerd_font() -> FontInstallResult {
    let error = |msg: String| FontInstallResult {
        installed_count: 0,
        already_installed: false,
        dest_dir: PathBuf::new(),
        message: msg,
    };

    let Some(dest_dir) = fonts_dir() else {
        return error("Could not determine fonts directory for this platform.".to_string());
    };

    if let Err(e) = fs::create_dir_all(&dest_dir) {
        return error(format!("Failed to create fonts directory: {e}"));
    }

    // Check if already installed
    if font_files_exist(&dest_dir) {
        return FontInstallResult {
            installed_count: 0,
            already_installed: true,
            dest_dir: dest_dir.clone(),
            message: format!(
                "{NERD_FONT_NAME} Nerd Font is already installed in {}.",
                dest_dir.display()
            ),
        };
    }

    println!("Downloading {NERD_FONT_NAME} Nerd Font {NERD_FONT_VERSION}...");

    let response = match reqwest::get(NERD_FONT_ZIP_URL).await {
        Ok(r) => r,
        Err(e) => return error(format!("Download failed: {e}")),
    };

    if !response.status().is_success() {
        return error(format!("Download failed: HTTP {}", response.status()));
    }

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => return error(format!("Failed to read download: {e}")),
    };

    println!("Extracting font files...");

    let cursor = Cursor::new(&bytes);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(e) => return error(format!("Failed to open zip: {e}")),
    };

    let mut installed_count = 0u32;
    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let Some(name) = file.enclosed_name().map(|n| n.to_owned()) else {
            continue;
        };

        let name_str = name.to_string_lossy();

        // Only extract .ttf files, skip Windows Compatible variants to reduce clutter
        if !name_str.ends_with(".ttf") || name_str.contains("WindowsCompatible") {
            continue;
        }

        let file_name = name.file_name().unwrap_or(name.as_os_str());
        let dest_path = dest_dir.join(file_name);

        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).is_ok() {
            if fs::write(&dest_path, &buf).is_ok() {
                installed_count += 1;
            }
        }
    }

    if installed_count == 0 {
        return error("No font files found in the downloaded archive.".to_string());
    }

    // Platform-specific post-install
    #[cfg(target_os = "windows")]
    {
        println!("Registering fonts...");
        if let Err(e) = register_fonts_windows(&dest_dir) {
            eprintln!("Warning: font registration failed: {e}");
        }
        if let Err(e) = activate_fonts_windows(&dest_dir) {
            eprintln!("Warning: font activation failed: {e}");
        }
    }

    #[cfg(target_os = "linux")]
    {
        println!("Refreshing font cache...");
        let _ = std::process::Command::new("fc-cache").arg("-f").status();
    }

    FontInstallResult {
        installed_count,
        already_installed: false,
        dest_dir: dest_dir.clone(),
        message: format!(
            "✓ Installed {installed_count} font files to {}.",
            dest_dir.display()
        ),
    }
}

/// Check if any JetBrainsMono Nerd Font .ttf files exist in the directory.
fn font_files_exist(dir: &Path) -> bool {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("JetBrainsMonoNerd") && name_str.ends_with(".ttf") {
                return true;
            }
        }
    }
    false
}

/// Check whether the Nerd Font is installed (files present in fonts directory).
pub fn is_font_installed() -> bool {
    fonts_dir().is_some_and(|dir| font_files_exist(&dir))
}

// ── Font reset ──────────────────────────────────────────────────────────────

/// Result of a font reset operation.
pub struct FontResetResult {
    pub removed_count: u32,
    pub message: String,
}

/// Remove all JetBrainsMono Nerd Font .ttf files from the given directory.
/// Returns the number of files successfully deleted.
fn remove_font_files(dir: &Path) -> u32 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0u32;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("JetBrainsMonoNerd") && name_str.ends_with(".ttf") {
            if fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

/// Reset the Nerd Font installation: delete font files and clean up
/// platform-specific registrations (Windows registry entries, font activation).
pub fn reset_nerd_font() -> FontResetResult {
    let Some(dest_dir) = fonts_dir() else {
        return FontResetResult {
            removed_count: 0,
            message: "Could not determine fonts directory for this platform.".to_string(),
        };
    };

    if !font_files_exist(&dest_dir) {
        return FontResetResult {
            removed_count: 0,
            message: "No Nerd Font files found — nothing to reset.".to_string(),
        };
    }

    // Platform-specific pre-removal (deactivate before deleting files)
    #[cfg(target_os = "windows")]
    {
        if let Err(e) = deactivate_fonts_windows(&dest_dir) {
            eprintln!("Warning: font deactivation failed: {e}");
        }
        if let Err(e) = unregister_fonts_windows() {
            eprintln!("Warning: registry cleanup failed: {e}");
        }
    }

    let removed = remove_font_files(&dest_dir);

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("fc-cache").arg("-f").status();
    }

    FontResetResult {
        removed_count: removed,
        message: format!(
            "✓ Removed {removed} font file{} from {}.\n\
             Run `pnm font install` to reinstall.",
            if removed == 1 { "" } else { "s" },
            dest_dir.display()
        ),
    }
}

// ── Windows font registration ───────────────────────────────────────────────

/// Derive a human-readable registry font name from a Nerd Font filename.
///
/// Example: `JetBrainsMonoNerdFont-Regular.ttf` → `JetBrainsMono Nerd Font Regular (TrueType)`
fn registry_font_name(filename: &str) -> String {
    let stem = filename.strip_suffix(".ttf").unwrap_or(filename);

    // Split on hyphen: "JetBrainsMonoNerdFont" + "Regular"
    let (family_part, style_part) = stem.split_once('-').unwrap_or((stem, ""));

    // Insert spaces into CamelCase family name
    let mut family = String::with_capacity(family_part.len() + 8);
    for (i, ch) in family_part.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            // Don't insert space between consecutive uppercase (e.g., "NF")
            let prev = family_part.as_bytes()[i - 1];
            if prev.is_ascii_lowercase() {
                family.push(' ');
            }
        }
        family.push(ch);
    }

    if style_part.is_empty() {
        format!("{family} (TrueType)")
    } else {
        format!("{family} {style_part} (TrueType)")
    }
}

#[cfg(target_os = "windows")]
fn register_fonts_windows(fonts_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (fonts_key, _) = hkcu.create_subkey(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
    )?;

    for entry in fs::read_dir(fonts_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if !name_str.starts_with("JetBrainsMonoNerd") || !name_str.ends_with(".ttf") {
            continue;
        }

        let reg_name = registry_font_name(&name_str);
        // Per-user fonts require the full absolute path as the value
        let full_path = entry.path().to_string_lossy().to_string();
        fonts_key.set_value(&reg_name, &full_path)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn activate_fonts_windows(fonts_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use windows::Win32::Graphics::Gdi::AddFontResourceW;
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SendMessageW, WM_FONTCHANGE,
    };
    use windows::core::HSTRING;

    for entry in fs::read_dir(fonts_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if !name_str.starts_with("JetBrainsMonoNerd") || !name_str.ends_with(".ttf") {
            continue;
        }

        let path_str = entry.path().to_string_lossy().to_string();
        let hstring = HSTRING::from(&path_str);
        unsafe {
            AddFontResourceW(&hstring);
        }
    }

    // Notify all top-level windows that fonts have changed
    unsafe {
        let _ = SendMessageW(HWND_BROADCAST, WM_FONTCHANGE, None, None);
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn unregister_fonts_windows() -> Result<(), Box<dyn std::error::Error>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (fonts_key, _) = hkcu.create_subkey(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
    )?;

    // Collect matching value names first, then delete
    let to_delete: Vec<String> = fonts_key
        .enum_values()
        .filter_map(|v| v.ok())
        .map(|(name, _)| name)
        .filter(|name| name.contains("Jet Brains Mono Nerd"))
        .collect();

    for name in &to_delete {
        let _ = fonts_key.delete_value(name);
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn deactivate_fonts_windows(fonts_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use windows::Win32::Graphics::Gdi::RemoveFontResourceW;
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SendMessageW, WM_FONTCHANGE,
    };
    use windows::core::HSTRING;

    for entry in fs::read_dir(fonts_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if !name_str.starts_with("JetBrainsMonoNerd") || !name_str.ends_with(".ttf") {
            continue;
        }

        let path_str = entry.path().to_string_lossy().to_string();
        let hstring = HSTRING::from(&path_str);
        unsafe {
            let _ = RemoveFontResourceW(&hstring);
        }
    }

    unsafe {
        let _ = SendMessageW(HWND_BROADCAST, WM_FONTCHANGE, None, None);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_font_name_regular() {
        assert_eq!(
            registry_font_name("JetBrainsMonoNerdFont-Regular.ttf"),
            "Jet Brains Mono Nerd Font Regular (TrueType)"
        );
    }

    #[test]
    fn test_registry_font_name_bold_italic() {
        assert_eq!(
            registry_font_name("JetBrainsMonoNerdFont-BoldItalic.ttf"),
            "Jet Brains Mono Nerd Font BoldItalic (TrueType)"
        );
    }

    #[test]
    fn test_registry_font_name_mono_variant() {
        assert_eq!(
            registry_font_name("JetBrainsMonoNerdFontMono-Regular.ttf"),
            "Jet Brains Mono Nerd Font Mono Regular (TrueType)"
        );
    }

    #[test]
    fn test_registry_font_name_no_style() {
        assert_eq!(
            registry_font_name("SomeFont.ttf"),
            "Some Font (TrueType)"
        );
    }

    #[test]
    fn test_font_face_constant() {
        assert_eq!(NERD_FONT_FACE, "JetBrainsMono Nerd Font");
    }

    // ── font_files_exist tests ──────────────────────────────────────────────

    #[test]
    fn test_font_files_exist_finds_matching() {
        let dir = std::env::temp_dir().join("pnm_test_font_exist_match");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("JetBrainsMonoNerdFont-Regular.ttf"), b"fake").unwrap();
        assert!(font_files_exist(&dir));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_font_files_exist_no_match_empty_dir() {
        let dir = std::env::temp_dir().join("pnm_test_font_exist_empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        assert!(!font_files_exist(&dir));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_font_files_exist_wrong_prefix() {
        let dir = std::env::temp_dir().join("pnm_test_font_exist_wrong");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("FiraCode-Regular.ttf"), b"fake").unwrap();
        fs::write(dir.join("JetBrainsMono-Regular.ttf"), b"fake").unwrap(); // missing "Nerd"
        assert!(!font_files_exist(&dir));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_font_files_exist_wrong_extension() {
        let dir = std::env::temp_dir().join("pnm_test_font_exist_ext");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("JetBrainsMonoNerdFont-Regular.otf"), b"fake").unwrap();
        assert!(!font_files_exist(&dir));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_font_files_exist_nonexistent_dir() {
        let dir = std::env::temp_dir().join("pnm_test_font_exist_missing_dir");
        let _ = fs::remove_dir_all(&dir);
        assert!(!font_files_exist(&dir));
    }

    // ── remove_font_files tests ─────────────────────────────────────────────

    #[test]
    fn test_remove_font_files_deletes_matching() {
        let dir = std::env::temp_dir().join("pnm_test_remove_match");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("JetBrainsMonoNerdFont-Regular.ttf"), b"fake").unwrap();
        fs::write(dir.join("JetBrainsMonoNerdFont-Bold.ttf"), b"fake").unwrap();
        fs::write(dir.join("JetBrainsMonoNerdFontMono-Regular.ttf"), b"fake").unwrap();

        let removed = remove_font_files(&dir);
        assert_eq!(removed, 3);
        assert!(!font_files_exist(&dir));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_remove_font_files_ignores_non_font_files() {
        let dir = std::env::temp_dir().join("pnm_test_remove_ignore");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Non-matching files that should survive
        fs::write(dir.join("FiraCode-Regular.ttf"), b"keep").unwrap();
        fs::write(dir.join("README.txt"), b"keep").unwrap();
        fs::write(dir.join("JetBrainsMono-Regular.ttf"), b"keep").unwrap();

        // Matching file that should be removed
        fs::write(dir.join("JetBrainsMonoNerdFont-Regular.ttf"), b"remove").unwrap();

        let removed = remove_font_files(&dir);
        assert_eq!(removed, 1);

        // Non-matching files still exist
        assert!(dir.join("FiraCode-Regular.ttf").exists());
        assert!(dir.join("README.txt").exists());
        assert!(dir.join("JetBrainsMono-Regular.ttf").exists());
        // Matching file is gone
        assert!(!dir.join("JetBrainsMonoNerdFont-Regular.ttf").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_remove_font_files_empty_dir() {
        let dir = std::env::temp_dir().join("pnm_test_remove_empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let removed = remove_font_files(&dir);
        assert_eq!(removed, 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_remove_font_files_nonexistent_dir() {
        let dir = std::env::temp_dir().join("pnm_test_remove_missing_dir");
        let _ = fs::remove_dir_all(&dir);

        let removed = remove_font_files(&dir);
        assert_eq!(removed, 0);
    }
}
