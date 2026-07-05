//! A dev **card gallery / text audit**: renders every card in `sample_table()` at all three render sizes
//! and reports any whose text overflows its fixed footprint (see [`cardtable::run_card_gallery`]). Full
//! coverage the in-app view can't give — it walks the model, not the current screen.
//!
//! Run with: `cargo run -p cardtable --example card_gallery`
//!
//! A window opens showing a scrollable grid (each row = one card at Small / Medium / Large); the terminal
//! prints an overflow report and overflowing cards are framed in red. Close the window when done.

fn main() {
    cardtable::run_card_gallery();
}
