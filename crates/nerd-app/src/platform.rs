//! System assertions for show time.
//!
//! While a song or the background is on air the app must not let the Mac sleep
//! or start the screensaver. `caffeinate` is part of macOS and creates the same
//! power assertions as a native IOKit call, without adding unsafe FFI to the UI
//! crate.

use std::process::{Child, Command, Stdio};

pub struct ShowAssertion {
    child: Option<Child>,
}

impl ShowAssertion {
    pub fn begin() -> Self {
        let child = Command::new("/usr/bin/caffeinate")
            .args(["-dims", "-w", &std::process::id().to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();
        if child.is_none() {
            tracing::warn!("could not create caffeinate assertion; screen may sleep");
        }
        Self { child }
    }

    pub fn end(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ShowAssertion {
    fn drop(&mut self) {
        self.end();
    }
}
