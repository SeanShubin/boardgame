//! Capture the git commit at build time and expose it as the `BUILD_GIT_HASH`, `BUILD_GIT_DATE`, and
//! `BUILD_GIT_TIMESTAMP` env vars (read via `option_env!` in the binary), so the deployed build can show
//! which commit it came from and how long ago it was built. Each falls back to empty / unset when git
//! isn't available (e.g. a source tarball with no `.git`).

use std::process::Command;

/// Run `git` with `args` and return its trimmed stdout, or `None` when git is unavailable or fails.
fn git(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn main() {
    let hash = git(&["describe", "--always", "--dirty", "--abbrev=8"])
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_GIT_HASH={hash}");

    // The commit's date (YYYY-MM-DD) and its unix timestamp (seconds), for the Version card's
    // "Updated {date}" and relative "{n} {unit} ago" lines. Emitted empty when git is unavailable.
    let date = git(&["log", "-1", "--format=%cs"]).unwrap_or_default();
    println!("cargo:rustc-env=BUILD_GIT_DATE={date}");
    let timestamp = git(&["log", "-1", "--format=%ct"]).unwrap_or_default();
    println!("cargo:rustc-env=BUILD_GIT_TIMESTAMP={timestamp}");

    // Re-run when the checked-out commit moves, so the stamp stays current.
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    if let Ok(head) = std::fs::read_to_string("../../.git/HEAD")
        && let Some(reference) = head.strip_prefix("ref: ").map(str::trim)
    {
        println!("cargo:rerun-if-changed=../../.git/{reference}");
    }
}
