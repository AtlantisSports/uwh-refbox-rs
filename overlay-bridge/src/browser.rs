//! Opening the operator's own browser at the bridge's status page when the bridge starts.
//!
//! The bridge has no window of its own -- its only user interface is the status page it serves
//! (see `status`). Before this, reaching that page meant reading an address off a console window
//! and typing it into a browser, which is a poor first minute for a volunteer who is setting up
//! minutes before a game.
//!
//! **Nothing here can change what the bridge serves.** It launches a browser and prints a line;
//! the feed, the tables and the status page are untouched.
//!
//! Split in two on purpose: [`status_page_url`] and [`open_command`] are values a test can
//! inspect, and only [`open_status_page`] actually launches anything. That is what lets the test
//! suite cover the platform wiring without opening a browser window on a developer's machine or
//! in CI.

use std::process::{Command, Stdio};

/// The address to send the operator's browser to.
///
/// **The numeric loopback, deliberately, not `localhost`.** The bridge binds the IPv4 wildcard
/// (`0.0.0.0`) and does not listen on IPv6 at all. On modern Windows `localhost` resolves to the
/// IPv6 loopback `::1` first, so a browser sent there is refused and has to fall back to IPv4 on
/// its own. Browsers generally do, but the numeric address removes the question.
///
/// Also not the `0.0.0.0` the server binds: that is "listen on every interface", not a
/// destination, and typing it into a browser on Windows generally fails.
pub fn status_page_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/")
}

/// Builds -- without running -- the command that hands `url` to whatever this operating system
/// has registered as the default handler for a web address.
fn open_command(url: &str) -> Command {
    let mut command = platform_command(url);
    // Nothing this produces belongs in the operator's console: it is the browser's own startup
    // noise, and it would land in the middle of the bridge's status lines. Mirrors the way
    // `refbox` spawns its own children (`refbox/src/main.rs`).
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Windows: `start` is a shell builtin rather than a program, so it has to be reached through
/// `cmd`.
///
/// The empty `""` is not decoration -- `start` reads a lone quoted argument as the *title* of the
/// window it opens, so without a placeholder it would swallow the address and open nothing.
///
/// `CREATE_NO_WINDOW` covers the one case where this could put a stray console on screen. A child
/// normally inherits the console the bridge already owns, so no second window appears; but if the
/// bridge is ever started without a console of its own -- auto-started at boot, say -- `cmd`
/// would create one. The flag costs nothing and is in the standard library, so it needs neither a
/// Windows bindings crate nor any `unsafe`.
#[cfg(target_os = "windows")]
fn platform_command(url: &str) -> Command {
    use std::os::windows::process::CommandExt;

    /// `CREATE_NO_WINDOW`, from the Windows process-creation flags.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut command = Command::new("cmd");
    command.args(["/C", "start", "", url]);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

/// macOS: `open` is the system's own "use the default application for this" tool.
#[cfg(target_os = "macos")]
fn platform_command(url: &str) -> Command {
    let mut command = Command::new("open");
    command.arg(url);
    command
}

/// Everything else: `xdg-open` is the freedesktop.org convention. Not a shipping target -- the
/// bridge is for a Windows or macOS streaming PC -- but it is what developers run the bridge on.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn platform_command(url: &str) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(url);
    command
}

/// Opens the status page in the operator's browser, and says what happened either way.
///
/// **Never fatal.** Serving vMix is the bridge's job; opening a browser is a convenience, so a
/// failure here is reported and otherwise ignored -- there is no desktop session, no handler is
/// registered, the helper is missing. The address is printed either way, so the operator can
/// still reach the page by hand: that fallback is the whole reason the startup line has to name
/// an address a browser can actually open.
///
/// Saying which of the two happened is for whoever is supporting a venue over the phone. "It
/// didn't open" covers both a browser that never launched and a browser that launched into some
/// other application, and those are diagnosed differently.
pub fn open_status_page(port: u16) {
    let url = status_page_url(port);
    // The child is deliberately not waited on -- it hands the address to the browser and exits,
    // and the bridge has a server to get on with. This runs exactly once per run, so the
    // unreaped child cannot accumulate.
    match open_command(&url).spawn() {
        Ok(_) => println!("Opened {url} in your browser."),
        Err(e) => {
            eprintln!("could not open a browser: {e}. Open {url} yourself to reach this page.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_page_url_is_the_numeric_loopback() {
        // Not `localhost`: the bridge binds the IPv4 wildcard and does not listen on IPv6, while
        // `localhost` resolves to the IPv6 loopback first on modern Windows. A browser sent to
        // `localhost` would be refused there and would depend on its own fallback to retry IPv4.
        let url = status_page_url(8099);

        assert_eq!(url, "http://127.0.0.1:8099/");
        assert!(
            !url.contains("localhost"),
            "the operator's browser must not be sent to a name that can resolve to IPv6"
        );
    }

    #[test]
    fn the_status_page_url_is_never_the_wildcard_the_server_binds() {
        // `0.0.0.0` is "listen on every interface", not a destination -- typing it into a browser
        // on Windows generally fails. This is the trap the printed startup line used to carry.
        assert!(!status_page_url(8099).contains("0.0.0.0"));
    }

    #[test]
    fn the_status_page_url_carries_the_configured_port() {
        // The port is an operator setting (`--port`), so a hard-coded 8099 anywhere in this path
        // would send the browser to the wrong place for anyone who changed it.
        assert_eq!(status_page_url(9001), "http://127.0.0.1:9001/");
    }

    #[test]
    fn the_open_command_hands_the_url_to_this_platform_s_own_handler() {
        // Asserting on the command as a *value* is what keeps this test honest: building it
        // proves the platform wiring without launching anything, so the suite never opens a
        // browser window on a developer's machine or in CI.
        let url = status_page_url(8099);
        let command = open_command(&url);

        let program = command.get_program().to_string_lossy().into_owned();
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();

        let expected_program = if cfg!(target_os = "windows") {
            "cmd"
        } else if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };

        assert_eq!(program, expected_program);
        assert!(
            args.contains(&url),
            "the address must reach the handler as its own argument, got {args:?}"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn the_windows_command_keeps_the_title_placeholder_before_the_url() {
        // `start` treats a lone quoted argument as the new window's title, so without the empty
        // placeholder it would swallow the address and open nothing. Order matters, not just
        // presence -- hence indexes rather than `contains`.
        let url = status_page_url(8099);
        let command = open_command(&url);
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();

        assert_eq!(
            args,
            vec!["/C".to_string(), "start".to_string(), String::new(), url]
        );
    }
}
