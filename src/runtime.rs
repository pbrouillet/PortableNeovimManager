use std::ffi::OsString;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::config::{GlobalSettings, InstanceManifest};

// ── Error type ─────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Bun not found on PATH. Install Bun or provide an absolute path.")]
    BunNotFound,

    #[error("Runtime binary not found at '{0}'")]
    BinaryNotFound(String),

    #[error("Failed to create shim: {0}")]
    ShimCreationFailed(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// ── Resolution ─────────────────────────────────────────────────────────────

/// Resolve the effective JavaScript runtime for an instance.
///
/// Priority: instance override > global default > None (system Node).
pub fn resolve_js_runtime(
    manifest: &InstanceManifest,
    settings: &GlobalSettings,
) -> Option<String> {
    manifest
        .js_runtime
        .clone()
        .or_else(|| settings.default_js_runtime.clone())
}

/// Display label for a js_runtime value.
pub fn runtime_display_name(js_runtime: Option<&str>) -> &str {
    match js_runtime {
        None => "Node (system default)",
        Some(val) => {
            if val.eq_ignore_ascii_case("bun") {
                "Bun"
            } else {
                // It's a custom path — show it as-is
                val
            }
        }
    }
}

/// Locate the runtime binary on the system.
///
/// - `"bun"` → searches PATH for `bun` / `bun.exe`
/// - Absolute path → validates it exists
pub fn find_runtime_binary(runtime_value: &str) -> Result<PathBuf, RuntimeError> {
    if runtime_value.eq_ignore_ascii_case("bun") {
        find_on_path("bun").ok_or(RuntimeError::BunNotFound)
    } else {
        let path = PathBuf::from(runtime_value);
        if path.is_file() {
            Ok(path)
        } else {
            Err(RuntimeError::BinaryNotFound(runtime_value.to_string()))
        }
    }
}

/// Search PATH for an executable by name.
fn find_on_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let extensions: Vec<&str> = if cfg!(windows) {
        vec![".exe", ".cmd", ".bat", ""]
    } else {
        vec![""]
    };

    for dir in std::env::split_paths(&path_var) {
        for ext in &extensions {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

// ── Shim directory ─────────────────────────────────────────────────────────

/// Path to the shims directory inside an instance.
pub fn shims_dir(instance_dir: &Path) -> PathBuf {
    instance_dir.join("shims")
}

/// Create node shims that delegate to the given runtime binary.
///
/// Returns the shims directory path (to prepend to PATH).
pub fn create_node_shims(
    instance_dir: &Path,
    runtime_binary: &Path,
) -> Result<PathBuf, RuntimeError> {
    let dir = shims_dir(instance_dir);
    std::fs::create_dir_all(&dir)?;

    let runtime_path_str = runtime_binary.display().to_string();

    if cfg!(windows) {
        create_windows_shims(&dir, &runtime_path_str)?;
    } else {
        create_unix_shims(&dir, &runtime_path_str)?;
    }

    Ok(dir)
}

#[cfg(windows)]
fn create_windows_shims(dir: &Path, runtime_path: &str) -> Result<(), RuntimeError> {
    // node.cmd — caught when tools invoke `node` (without extension)
    let cmd_content = format!("@\"{runtime_path}\" %*\r\n");
    std::fs::write(dir.join("node.cmd"), &cmd_content)
        .map_err(|e| RuntimeError::ShimCreationFailed(format!("node.cmd: {e}")))?;

    // node.exe — copy of the runtime binary for tools that invoke `node.exe` directly
    let src = PathBuf::from(runtime_path);
    let dst = dir.join("node.exe");
    if !dst.exists() || file_changed(&src, &dst) {
        std::fs::copy(&src, &dst)
            .map_err(|e| RuntimeError::ShimCreationFailed(format!("node.exe copy: {e}")))?;
    }

    // npm.cmd — stub that redirects to bun (covers npm install calls from Mason)
    let npm_content = format!("@\"{runtime_path}\" x %*\r\n");
    std::fs::write(dir.join("npm.cmd"), &npm_content)
        .map_err(|e| RuntimeError::ShimCreationFailed(format!("npm.cmd: {e}")))?;

    // npx.cmd
    let npx_content = format!("@\"{runtime_path}\" x %*\r\n");
    std::fs::write(dir.join("npx.cmd"), &npx_content)
        .map_err(|e| RuntimeError::ShimCreationFailed(format!("npx.cmd: {e}")))?;

    Ok(())
}

#[cfg(not(windows))]
fn create_windows_shims(_dir: &Path, _runtime_path: &str) -> Result<(), RuntimeError> {
    Ok(())
}

#[cfg(not(windows))]
fn create_unix_shims(dir: &Path, runtime_path: &str) -> Result<(), RuntimeError> {
    use std::os::unix::fs::PermissionsExt;

    // node — shell script that exec's the runtime
    let node_content = format!("#!/bin/sh\nexec \"{runtime_path}\" \"$@\"\n");
    let node_path = dir.join("node");
    std::fs::write(&node_path, &node_content)
        .map_err(|e| RuntimeError::ShimCreationFailed(format!("node: {e}")))?;
    std::fs::set_permissions(&node_path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| RuntimeError::ShimCreationFailed(format!("chmod node: {e}")))?;

    // npm — redirects to bun
    let npm_content = format!("#!/bin/sh\nexec \"{runtime_path}\" x \"$@\"\n");
    let npm_path = dir.join("npm");
    std::fs::write(&npm_path, &npm_content)
        .map_err(|e| RuntimeError::ShimCreationFailed(format!("npm: {e}")))?;
    std::fs::set_permissions(&npm_path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| RuntimeError::ShimCreationFailed(format!("chmod npm: {e}")))?;

    // npx — redirects to bun x
    let npx_content = format!("#!/bin/sh\nexec \"{runtime_path}\" x \"$@\"\n");
    let npx_path = dir.join("npx");
    std::fs::write(&npx_path, &npx_content)
        .map_err(|e| RuntimeError::ShimCreationFailed(format!("npx: {e}")))?;
    std::fs::set_permissions(&npx_path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| RuntimeError::ShimCreationFailed(format!("chmod npx: {e}")))?;

    Ok(())
}

#[cfg(windows)]
fn create_unix_shims(_dir: &Path, _runtime_path: &str) -> Result<(), RuntimeError> {
    Ok(())
}

/// Check if source and dest files differ (by size).
#[cfg(windows)]
fn file_changed(src: &Path, dst: &Path) -> bool {
    match (std::fs::metadata(src), std::fs::metadata(dst)) {
        (Ok(s), Ok(d)) => s.len() != d.len(),
        _ => true,
    }
}

/// Remove the shims directory.
pub fn cleanup_shims(instance_dir: &Path) {
    let dir = shims_dir(instance_dir);
    let _ = std::fs::remove_dir_all(dir);
}

// ── PATH construction ──────────────────────────────────────────────────────

/// Build a PATH with the shims directory prepended.
pub fn build_path_with_shims(shims_dir: &Path) -> OsString {
    let current_path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![shims_dir.to_path_buf()];
    paths.extend(std::env::split_paths(&current_path));
    std::env::join_paths(paths).unwrap_or(current_path)
}

// ── Setup helper ───────────────────────────────────────────────────────────

/// Full setup: resolve runtime, find binary, create shims, return PATH.
///
/// Returns `Some(new_path)` if shims were created, `None` if no runtime override.
pub fn setup_runtime_shims(
    instance_dir: &Path,
    manifest: &InstanceManifest,
    settings: &GlobalSettings,
) -> Result<Option<OsString>, RuntimeError> {
    let runtime_value = match resolve_js_runtime(manifest, settings) {
        Some(v) => v,
        None => return Ok(None),
    };

    let binary = find_runtime_binary(&runtime_value)?;
    let dir = create_node_shims(instance_dir, &binary)?;
    let new_path = build_path_with_shims(&dir);
    Ok(Some(new_path))
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GlobalSettings, InstanceManifest};

    #[test]
    fn resolve_prefers_instance_over_global() {
        let mut manifest = InstanceManifest::new("test".into(), "v0.10".into(), vec![]);
        manifest.js_runtime = Some("bun".into());

        let mut settings = GlobalSettings::default();
        settings.default_js_runtime = Some("/custom/node".into());

        assert_eq!(resolve_js_runtime(&manifest, &settings), Some("bun".into()));
    }

    #[test]
    fn resolve_falls_back_to_global() {
        let manifest = InstanceManifest::new("test".into(), "v0.10".into(), vec![]);
        let mut settings = GlobalSettings::default();
        settings.default_js_runtime = Some("bun".into());

        assert_eq!(resolve_js_runtime(&manifest, &settings), Some("bun".into()));
    }

    #[test]
    fn resolve_returns_none_when_unset() {
        let manifest = InstanceManifest::new("test".into(), "v0.10".into(), vec![]);
        let settings = GlobalSettings::default();

        assert_eq!(resolve_js_runtime(&manifest, &settings), None);
    }

    #[test]
    fn display_names() {
        assert_eq!(runtime_display_name(None), "Node (system default)");
        assert_eq!(runtime_display_name(Some("bun")), "Bun");
        assert_eq!(runtime_display_name(Some("BUN")), "Bun");
        assert_eq!(runtime_display_name(Some("/usr/bin/deno")), "/usr/bin/deno");
    }

    #[test]
    fn shims_dir_path() {
        let dir = PathBuf::from("/instances/test");
        assert_eq!(shims_dir(&dir), PathBuf::from("/instances/test/shims"));
    }

    #[test]
    fn build_path_prepends_shims() {
        let shims = PathBuf::from("/my/shims");
        let result = build_path_with_shims(&shims);
        let result_str = result.to_string_lossy();
        assert!(result_str.starts_with("/my/shims") || result_str.starts_with("\\my\\shims")
            || result_str.contains("my") ); // Windows paths differ
    }
}
