//! Driver: load a roster and write the report directory.
//!
//! Usage: `cargo run -p combat-lab --bin combat_report [roster.ron] [out-dir]`
//! Defaults: `roster.ron` and `combat-reports/` in the current directory.

use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let roster_path = args.next().unwrap_or_else(|| "roster.ron".to_string());
    let out_dir = args.next().unwrap_or_else(|| "combat-reports".to_string());

    let chars = match combat_lab::roster::load(&roster_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    match combat_lab::report::generate(&chars, Path::new(&out_dir)) {
        Ok(files) => {
            println!(
                "wrote {files} reports for {} characters to {out_dir}/",
                chars.len()
            );
        }
        Err(e) => {
            eprintln!("error writing reports: {e}");
            std::process::exit(1);
        }
    }
}
