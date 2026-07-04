//! Capture the git commit at build time and expose it as the `BUILD_GIT_HASH` env var (read via
//! `env!` in the binary), so the deployed build can show which commit it came from. Falls back to
//! `"unknown"` when git isn't available (e.g. a source tarball with no `.git`).

use std::process::Command;

fn main() {
    let hash = Command::new("git")
        .args(["describe", "--always", "--dirty", "--abbrev=8"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_GIT_HASH={hash}");

    // Re-run when the checked-out commit moves, so the stamp stays current.
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    if let Ok(head) = std::fs::read_to_string("../../.git/HEAD")
        && let Some(reference) = head.strip_prefix("ref: ").map(str::trim)
    {
        println!("cargo:rerun-if-changed=../../.git/{reference}");
    }
}
