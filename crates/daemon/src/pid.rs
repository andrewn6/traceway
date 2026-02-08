use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

pub struct PidFile {
    path: PathBuf,
}

impl PidFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Acquire the PID file. Returns error if another instance is running.
    pub fn acquire(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        if let Some(existing_pid) = self.read_pid() {
            if is_process_alive(existing_pid) {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!(
                        "daemon already running (pid {}). Stop it first or remove {}",
                        existing_pid,
                        self.path.display()
                    ),
                ));
            }
            // Stale PID file — clean it up
            tracing::info!(pid = existing_pid, "removing stale PID file");
            let _ = fs::remove_file(&self.path);
        }

        let pid = std::process::id();
        fs::write(&self.path, pid.to_string())?;
        tracing::info!(pid, path = %self.path.display(), "wrote PID file");
        Ok(())
    }

    /// Remove the PID file.
    pub fn release(&self) {
        if self.path.exists() {
            if let Err(e) = fs::remove_file(&self.path) {
                tracing::warn!(error = %e, "failed to remove PID file");
            } else {
                tracing::info!(path = %self.path.display(), "removed PID file");
            }
        }
    }

    /// Check if a daemon is running by reading the PID file and checking the process.
    pub fn is_running(&self) -> Option<u32> {
        let pid = self.read_pid()?;
        if is_process_alive(pid) {
            Some(pid)
        } else {
            None
        }
    }

    fn read_pid(&self) -> Option<u32> {
        fs::read_to_string(&self.path)
            .ok()?
            .trim()
            .parse::<u32>()
            .ok()
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        self.release();
    }
}

/// Check if a process is alive using kill(pid, 0).
pub fn is_process_alive(pid: u32) -> bool {
    kill(Pid::from_raw(pid as i32), Signal::SIGCONT)
        .map(|_| true)
        .unwrap_or_else(|e| {
            // EPERM means process exists but we can't signal it
            e == nix::errno::Errno::EPERM
        })
}

/// Read PID from a file path and check if alive.
pub fn check_running(pid_path: &Path) -> Option<u32> {
    let pid_file = PidFile::new(pid_path.to_path_buf());
    pid_file.is_running()
}
