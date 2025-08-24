use std::sync::Arc;
use std::{
    io::{self, Write},
    process::{Child, Command as StdCommand, Stdio},
};
use tokio::sync::Mutex;

// Type alias for clarity
pub type ChildHandle = Arc<Mutex<Option<Child>>>;

/// Kill any process in `slot` if it exists.
pub async fn kill_existing(slot: &mut Option<Child>) {
    if let Some(mut child) = slot.take() {
        let pid = child.id();
        log::debug!("Interrupting previous command (PID: {})", pid);
        if let Err(e) = child.kill() {
            log::error!("Warning: failed to kill PID {}: {}", pid, e);
        }
    }
}

/// Spawn `sh -c <cmd>`, pipe in `message` on stdin, and return the new Child.
pub fn spawn_and_pipe(cmd: &str, message: Vec<u8>) -> io::Result<Child> {
    let mut child = StdCommand::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    let pid = child.id();
    log::debug!("Spawned new command (PID: {})", pid);

    if let Some(mut stdin) = child.stdin.take() {
        // Fire-and-forget task that writes the decrypted message into the child's stdin.
        tokio::spawn(async move {
            if let Err(e) = stdin.write_all(&message) {
                log::error!("Error writing to stdin of PID {}: {}", pid, e);
            }
            // flush and drop to close the pipe
            if let Err(e) = stdin.flush() {
                log::warn!("flush failed on PID {}: {}", pid, e);
            }
            log::debug!("Wrote {} bytes to PID {}", message.len(), pid);
        });
    } else {
        log::error!("Failed to open stdin for PID {}", pid);
        let _ = child.kill();
    }

    Ok(child)
}
