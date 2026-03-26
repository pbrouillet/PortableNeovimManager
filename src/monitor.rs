use std::path::{Path, PathBuf};
use std::time::Duration;

use sysinfo::{Pid, ProcessesToUpdate, System};
use thiserror::Error;

// ── Error type ─────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum MonitorError {
    #[error("Instance is not running (no PID file found)")]
    InstanceNotRunning,

    #[error("Failed to read PID file: {0}")]
    PidFileError(String),

    #[error("Process {0} not found — Neovim may have exited")]
    ProcessNotFound(u32),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("RPC error: {0}")]
    RpcError(String),
}

// ── Data types ─────────────────────────────────────────────────────────────

/// Memory and CPU stats for a single process.
#[derive(Clone, Debug)]
pub struct ProcessMemory {
    pub pid: u32,
    pub name: String,
    pub working_set_bytes: u64,
    pub virtual_memory_bytes: u64,
    pub cpu_percent: f32,
}

/// Complete memory snapshot for a Neovim instance and its children.
#[derive(Clone, Debug)]
pub struct InstanceMemorySnapshot {
    pub nvim_process: ProcessMemory,
    pub child_processes: Vec<ProcessMemory>,
    pub lua_memory_bytes: Option<u64>,
    pub total_working_set_bytes: u64,
    pub total_virtual_memory_bytes: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ── PID file helpers ───────────────────────────────────────────────────────

/// Path to the PID file inside an instance directory.
pub fn pid_file_path(instance_dir: &Path) -> PathBuf {
    instance_dir.join("nvim.pid")
}

/// Path to the RPC address file inside an instance directory.
pub fn rpc_addr_file_path(instance_dir: &Path) -> PathBuf {
    instance_dir.join("nvim-rpc-addr.txt")
}

/// Read the Neovim PID from the instance's `nvim.pid` file.
pub fn read_pid_file(instance_dir: &Path) -> Result<u32, MonitorError> {
    let path = pid_file_path(instance_dir);
    if !path.exists() {
        return Err(MonitorError::InstanceNotRunning);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| MonitorError::PidFileError(format!("{}: {e}", path.display())))?;
    content
        .trim()
        .parse::<u32>()
        .map_err(|e| MonitorError::PidFileError(format!("invalid PID '{}': {e}", content.trim())))
}

/// Write a PID to the instance's `nvim.pid` file.
pub fn write_pid_file(instance_dir: &Path, pid: u32) -> Result<(), MonitorError> {
    let path = pid_file_path(instance_dir);
    std::fs::write(&path, pid.to_string())?;
    Ok(())
}

/// Remove the PID file (cleanup on exit).
pub fn remove_pid_file(instance_dir: &Path) {
    let _ = std::fs::remove_file(pid_file_path(instance_dir));
}

/// Read the RPC address from the instance's `nvim-rpc-addr.txt` file.
pub fn read_rpc_addr(instance_dir: &Path) -> Option<String> {
    let path = rpc_addr_file_path(instance_dir);
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// Write the RPC address to the instance's `nvim-rpc-addr.txt` file.
pub fn write_rpc_addr_file(instance_dir: &Path, addr: &str) -> Result<(), MonitorError> {
    let path = rpc_addr_file_path(instance_dir);
    std::fs::write(&path, addr)?;
    Ok(())
}

/// Remove the RPC address file (cleanup on exit).
pub fn remove_rpc_addr_file(instance_dir: &Path) {
    let _ = std::fs::remove_file(rpc_addr_file_path(instance_dir));
}

// ── RPC address construction ───────────────────────────────────────────────

/// Build the RPC listen address for a given instance name.
///
/// On Windows this is a named pipe; on Unix it's a socket file in the instance dir.
pub fn rpc_listen_addr(instance_dir: &Path, instance_name: &str) -> String {
    if cfg!(windows) {
        format!(r"\\.\pipe\pnm-nvim-{instance_name}")
    } else {
        instance_dir
            .join("nvim-rpc.sock")
            .display()
            .to_string()
    }
}

// ── Process queries ────────────────────────────────────────────────────────

/// Check whether a process with the given PID is still alive.
pub fn is_process_alive(pid: u32) -> bool {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), true);
    sys.process(Pid::from_u32(pid)).is_some()
}

/// Take a memory snapshot of the Neovim process and all its children.
///
/// The returned snapshot includes the Neovim process itself plus every
/// descendant in the process tree (LSP servers, formatters, DAP adapters, etc.).
pub fn snapshot_memory(pid: u32) -> Result<InstanceMemorySnapshot, MonitorError> {
    let mut sys = System::new();
    // Refresh all processes so we can walk the tree
    sys.refresh_processes(ProcessesToUpdate::All, true);
    // Brief pause then refresh again so CPU percentages are meaningful
    std::thread::sleep(Duration::from_millis(200));
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let nvim_pid = Pid::from_u32(pid);
    let nvim_proc = sys
        .process(nvim_pid)
        .ok_or(MonitorError::ProcessNotFound(pid))?;

    let nvim_mem = ProcessMemory {
        pid,
        name: nvim_proc.name().to_string_lossy().to_string(),
        working_set_bytes: nvim_proc.memory(),
        virtual_memory_bytes: nvim_proc.virtual_memory(),
        cpu_percent: nvim_proc.cpu_usage(),
    };

    // Walk process tree to find all children (direct and transitive)
    let child_pids = collect_descendants(&sys, nvim_pid);
    let mut children: Vec<ProcessMemory> = Vec::new();
    for cpid in &child_pids {
        if let Some(proc) = sys.process(*cpid) {
            children.push(ProcessMemory {
                pid: cpid.as_u32(),
                name: proc.name().to_string_lossy().to_string(),
                working_set_bytes: proc.memory(),
                virtual_memory_bytes: proc.virtual_memory(),
                cpu_percent: proc.cpu_usage(),
            });
        }
    }

    // Sort children by working set descending
    children.sort_by(|a, b| b.working_set_bytes.cmp(&a.working_set_bytes));

    let total_ws = nvim_mem.working_set_bytes + children.iter().map(|c| c.working_set_bytes).sum::<u64>();
    let total_vm = nvim_mem.virtual_memory_bytes + children.iter().map(|c| c.virtual_memory_bytes).sum::<u64>();

    Ok(InstanceMemorySnapshot {
        nvim_process: nvim_mem,
        child_processes: children,
        lua_memory_bytes: None, // filled in separately via RPC
        total_working_set_bytes: total_ws,
        total_virtual_memory_bytes: total_vm,
        timestamp: chrono::Utc::now(),
    })
}

/// Collect all descendant PIDs of a given parent, recursively.
fn collect_descendants(sys: &System, parent: Pid) -> Vec<Pid> {
    let mut result = Vec::new();
    let mut stack = vec![parent];

    while let Some(current) = stack.pop() {
        for (pid, proc) in sys.processes() {
            if let Some(ppid) = proc.parent() {
                if ppid == current && *pid != parent {
                    result.push(*pid);
                    stack.push(*pid);
                }
            }
        }
    }

    result
}

// ── Lua memory via RPC ─────────────────────────────────────────────────────

/// Query Neovim's Lua heap memory via `nvim --server <addr> --remote-expr`.
///
/// This shells out to the Neovim binary to avoid implementing msgpack-rpc.
/// Returns the Lua heap size in bytes.
pub fn query_lua_memory(nvim_binary: &Path, rpc_addr: &str) -> Result<u64, MonitorError> {
    let output = std::process::Command::new(nvim_binary)
        .arg("--server")
        .arg(rpc_addr)
        .arg("--remote-expr")
        .arg("luaeval('collectgarbage(\"count\") * 1024')")
        .output()
        .map_err(|e| MonitorError::RpcError(format!("failed to run nvim --server: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MonitorError::RpcError(format!(
            "nvim --server exited with {}: {}",
            output.status,
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value_str = stdout.trim();
    // The result may be a float like "1234567.0"
    value_str
        .parse::<f64>()
        .map(|v| v as u64)
        .map_err(|e| MonitorError::RpcError(format!("unexpected response '{value_str}': {e}")))
}

// ── Formatting helpers ─────────────────────────────────────────────────────

/// Format a byte count as a human-readable string (e.g. "128.4 MB").
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Build a full memory snapshot including optional Lua heap stats.
///
/// This is the main entry point for both CLI and TUI consumers.
pub fn full_snapshot(
    instance_dir: &Path,
    nvim_binary: Option<&Path>,
) -> Result<InstanceMemorySnapshot, MonitorError> {
    let pid = read_pid_file(instance_dir)?;

    // Validate PID is actually alive; clean up stale PID file if not
    if !is_process_alive(pid) {
        remove_pid_file(instance_dir);
        remove_rpc_addr_file(instance_dir);
        return Err(MonitorError::ProcessNotFound(pid));
    }

    let mut snap = snapshot_memory(pid)?;

    // Try to get Lua memory via RPC (best-effort)
    if let Some(nvim_bin) = nvim_binary {
        if let Some(addr) = read_rpc_addr(instance_dir) {
            match query_lua_memory(nvim_bin, &addr) {
                Ok(lua_bytes) => snap.lua_memory_bytes = Some(lua_bytes),
                Err(_) => {} // RPC not available — that's fine
            }
        }
    }

    Ok(snap)
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_display() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(128 * 1024 * 1024 + 400 * 1024), "128.4 MB");
        assert_eq!(format_bytes(2 * 1024 * 1024 * 1024), "2.0 GB");
    }

    #[test]
    fn pid_file_roundtrip() {
        let tmp = std::env::temp_dir().join("pnm_test_pid_roundtrip");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        write_pid_file(&tmp, 12345).unwrap();
        assert_eq!(read_pid_file(&tmp).unwrap(), 12345);

        remove_pid_file(&tmp);
        assert!(matches!(read_pid_file(&tmp), Err(MonitorError::InstanceNotRunning)));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rpc_addr_windows() {
        let dir = PathBuf::from(r"C:\instances\test");
        let addr = rpc_listen_addr(&dir, "test");
        if cfg!(windows) {
            assert_eq!(addr, r"\\.\pipe\pnm-nvim-test");
        }
    }

    #[test]
    fn missing_pid_file_returns_not_running() {
        let tmp = std::env::temp_dir().join("pnm_test_no_pid");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let result = read_pid_file(&tmp);
        assert!(matches!(result, Err(MonitorError::InstanceNotRunning)));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
