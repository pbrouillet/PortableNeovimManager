use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

use crate::monitor;
use crate::runtime;

#[derive(Debug, Error)]
pub enum NeovimError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Neovim binary not found in instance '{0}'")]
    BinaryNotFound(String),

    #[error("Failed to launch Neovim: {0}")]
    LaunchFailed(String),
}

/// Search `instance_dir/bin/` (and nested subdirectories) for an `nvim` or
/// `nvim.exe` executable. Extracted archives sometimes nest the binary under
/// an extra directory (e.g. `bin/nvim-win64/bin/nvim.exe`).
pub fn find_nvim_binary(instance_dir: &Path) -> Result<PathBuf, NeovimError> {
    let bin_dir = instance_dir.join("bin");

    let candidates: &[&str] = if cfg!(windows) {
        &["nvim.exe", "nvim"]
    } else {
        &["nvim"]
    };

    // Direct check in bin/
    for name in candidates {
        let path = bin_dir.join(name);
        if path.is_file() {
            return Ok(path);
        }
    }

    // Recursive search in subdirectories of bin/
    if bin_dir.is_dir() {
        if let Some(found) = search_dir_recursive(&bin_dir, candidates) {
            return Ok(found);
        }
    }

    Err(NeovimError::BinaryNotFound(
        instance_dir.display().to_string(),
    ))
}

fn search_dir_recursive(dir: &Path, candidates: &[&str]) -> Option<PathBuf> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return None,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Check direct children of this subdirectory first
            for name in candidates {
                let candidate = path.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            // Then recurse deeper
            if let Some(found) = search_dir_recursive(&path, candidates) {
                return Some(found);
            }
        }
    }

    None
}

/// Launch Neovim from the given instance directory with isolated XDG paths.
///
/// All configuration, data, cache, and state directories are scoped to the
/// instance so that multiple portable installations stay independent.
///
/// Writes a PID file (`nvim.pid`) and RPC address file (`nvim-rpc-addr.txt`)
/// to the instance directory for monitoring. Both are cleaned up on exit.
///
/// If `js_runtime_path` is `Some`, a shims directory is created with a `node`
/// shim that delegates to the specified runtime, and the shims dir is
/// prepended to PATH.
pub fn launch(
    instance_dir: &Path,
    instance_name: &str,
    extra_args: &[String],
    js_runtime_path: Option<std::ffi::OsString>,
) -> Result<std::process::ExitStatus, NeovimError> {
    let nvim = find_nvim_binary(instance_dir)?;

    let rpc_addr = monitor::rpc_listen_addr(instance_dir, instance_name);

    let mut cmd = Command::new(&nvim);
    cmd.arg("--listen")
        .arg(&rpc_addr)
        .args(extra_args)
        .env("XDG_CONFIG_HOME", instance_dir.join("config"))
        .env("XDG_DATA_HOME", instance_dir.join("data"))
        .env("XDG_CACHE_HOME", instance_dir.join("cache"))
        .env("XDG_STATE_HOME", instance_dir.join("state"))
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    // Apply runtime shim PATH if configured
    if let Some(ref new_path) = js_runtime_path {
        cmd.env("PATH", new_path);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| NeovimError::LaunchFailed(e.to_string()))?;

    // Write PID and RPC address files for monitoring
    let pid = child.id();
    let _ = monitor::write_pid_file(instance_dir, pid);
    let _ = monitor::write_rpc_addr_file(instance_dir, &rpc_addr);

    // Wait for Neovim to exit, then clean up
    let status = child
        .wait()
        .map_err(|e| NeovimError::LaunchFailed(e.to_string()))?;

    monitor::remove_pid_file(instance_dir);
    monitor::remove_rpc_addr_file(instance_dir);

    // Clean up shims directory if we created one
    if js_runtime_path.is_some() {
        runtime::cleanup_shims(instance_dir);
    }

    Ok(status)
}

/// Run `nvim --version` and return the first line (e.g. "NVIM v0.10.4").
pub fn get_version(instance_dir: &Path) -> Result<String, NeovimError> {
    let nvim = find_nvim_binary(instance_dir)?;

    let output = Command::new(&nvim)
        .arg("--version")
        .output()
        .map_err(|e| NeovimError::LaunchFailed(format!("could not run nvim --version: {e}")))?;

    if !output.status.success() {
        return Err(NeovimError::LaunchFailed(format!(
            "nvim --version exited with status {}",
            output.status,
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or("").trim().to_string();

    if first_line.is_empty() {
        return Err(NeovimError::LaunchFailed(
            "nvim --version produced no output".to_string(),
        ));
    }

    Ok(first_line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn binary_not_found_in_empty_dir() {
        let tmp = std::env::temp_dir().join("pnm_test_empty_instance");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("bin")).unwrap();

        let result = find_nvim_binary(&tmp);
        assert!(result.is_err());
        assert!(matches!(result, Err(NeovimError::BinaryNotFound(_))));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn finds_binary_in_nested_subdir() {
        let tmp = std::env::temp_dir().join("pnm_test_nested_instance");
        let _ = fs::remove_dir_all(&tmp);

        let nested_bin = if cfg!(windows) {
            tmp.join("bin").join("nvim-win64").join("bin")
        } else {
            tmp.join("bin").join("nvim-linux64").join("bin")
        };
        fs::create_dir_all(&nested_bin).unwrap();

        let binary_name = if cfg!(windows) { "nvim.exe" } else { "nvim" };
        let binary_path = nested_bin.join(binary_name);
        fs::write(&binary_path, b"fake").unwrap();

        let found = find_nvim_binary(&tmp).unwrap();
        assert_eq!(found, binary_path);

        let _ = fs::remove_dir_all(&tmp);
    }
}
