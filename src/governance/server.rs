//! Managed child process for `specsmith serve`.
//!
//! `GovernanceServer` spawns `specsmith serve` as a subprocess and owns its
//! lifecycle. On drop the child is terminated cleanly.
//!
//! # REQ-002 — Kairos Spawns specsmith serve as Managed Child Process

use anyhow::{Context, Result};
use std::process::{Child, Command};
use std::time::Duration;

/// Handle for the `specsmith serve` child process (REQ-002).
///
/// Drop this to cleanly terminate the governance backend.
pub struct GovernanceServer {
    child: Child,
    /// Port on which the server is listening.
    pub port: u16,
}

impl GovernanceServer {
    /// Spawn `specsmith serve` and wait for it to become healthy.
    ///
    /// `specsmith_cmd` is the command used to invoke specsmith (e.g. `"py -m specsmith"`
    /// on Windows or `"specsmith"` on Unix if installed globally).
    ///
    /// H11 — includes a startup timeout: returns an error if the server does not
    /// respond to `/health` within `startup_timeout`.
    pub fn spawn(
        specsmith_cmd: &str,
        port: u16,
        startup_timeout: Duration,
    ) -> Result<Self> {
        // Split the command into program + args (e.g. ["py", "-m", "specsmith"]).
        let mut parts = specsmith_cmd.split_whitespace();
        let program = parts.next().context("specsmith_cmd must not be empty")?;
        let args: Vec<&str> = parts.collect();

        let child = Command::new(program)
            .args(&args)
            .arg("serve")
            .arg("--port")
            .arg(port.to_string())
            .spawn()
            .with_context(|| format!("Failed to spawn `{specsmith_cmd} serve --port {port}`"))?;

        let server = Self { child, port };
        server.wait_healthy(startup_timeout)?;
        Ok(server)
    }

    /// Poll `GET /health` until the server responds or the timeout expires.
    ///
    /// H11 — blocking wait with explicit timeout and diagnostic error.
    fn wait_healthy(&self, timeout: Duration) -> Result<()> {
        use std::thread;
        use std::time::Instant;

        let url = format!("http://127.0.0.1:{}/health", self.port);
        let deadline = Instant::now() + timeout;
        let poll_interval = Duration::from_millis(200);

        while Instant::now() < deadline {
            // Use a synchronous check here since we're in startup (not inside async).
            if let Ok(resp) = ureq::get(&url).call() {
                if resp.status() == 200 {
                    return Ok(());
                }
            }
            thread::sleep(poll_interval);
        }

        Err(anyhow::anyhow!(
            "specsmith serve did not become healthy within {:.1}s at {url}. \
             Ensure specsmith is installed: `pip install specsmith`",
            timeout.as_secs_f64()
        ))
    }

    /// Terminate the governance server child process.
    pub fn terminate(mut self) -> Result<()> {
        self.child.kill().context("Failed to terminate specsmith serve")?;
        self.child.wait().context("Failed to wait for specsmith serve to exit")?;
        Ok(())
    }
}

impl Drop for GovernanceServer {
    /// Attempt a graceful kill on drop. Errors are suppressed (best-effort cleanup).
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
