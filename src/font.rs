use std::fs;
use std::io::{Cursor, Read};
use std::path::PathBuf;

const NERD_FONT_NAME: &str = "JetBrainsMono";
const NERD_FONT_VERSION: &str = "v3.3.0";
const NERD_FONT_ZIP_URL: &str =
    "https://github.com/ryanoasis/nerd-fonts/releases/download/v3.3.0/JetBrainsMono.zip";

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

/// Downloads and installs JetBrainsMono Nerd Font.
/// Returns a user-facing message describing the result.
pub async fn install_nerd_font() -> String {
    let Some(dest_dir) = fonts_dir() else {
        return "Could not determine fonts directory for this platform.".to_string();
    };

    if let Err(e) = fs::create_dir_all(&dest_dir) {
        return format!("Failed to create fonts directory: {e}");
    }

    // Check if already installed
    if font_files_exist(&dest_dir) {
        return format!(
            "{NERD_FONT_NAME} Nerd Font is already installed in {}.\n\n\
             Set your terminal font to \"{NERD_FONT_NAME} Nerd Font\" (or \"JetBrainsMono NF\") and restart the terminal.",
            dest_dir.display()
        );
    }

    println!("Downloading {NERD_FONT_NAME} Nerd Font {NERD_FONT_VERSION}...");

    // Download the zip
    let response = match reqwest::get(NERD_FONT_ZIP_URL).await {
        Ok(r) => r,
        Err(e) => return format!("Download failed: {e}"),
    };

    if !response.status().is_success() {
        return format!("Download failed: HTTP {}", response.status());
    }

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => return format!("Failed to read download: {e}"),
    };

    println!("Extracting font files...");

    // Extract .ttf files from the zip
    let cursor = Cursor::new(&bytes);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(e) => return format!("Failed to open zip: {e}"),
    };

    let mut installed_count = 0;
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
        return "No font files found in the downloaded archive.".to_string();
    }

    // On Linux, refresh font cache
    #[cfg(target_os = "linux")]
    {
        println!("Refreshing font cache...");
        let _ = std::process::Command::new("fc-cache").arg("-f").status();
    }

    format!(
        "✓ Installed {installed_count} font files to {}.\n\n\
         Now set your terminal font to \"{NERD_FONT_NAME} Nerd Font\" (or \"JetBrainsMono NF\"):\n\
         • Windows Terminal: Settings → Profiles → Appearance → Font face\n\
         • iTerm2: Preferences → Profiles → Text → Font\n\
         • GNOME Terminal: Preferences → Profile → Custom font\n\n\
         Then restart your terminal.",
        dest_dir.display()
    )
}

/// Check if any JetBrainsMono Nerd Font .ttf files exist in the directory.
fn font_files_exist(dir: &PathBuf) -> bool {
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
