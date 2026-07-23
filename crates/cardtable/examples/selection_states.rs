//! Dev demo of the four **selection states** a card can be in during a multi-part (source -> action ->
//! target) gesture - background / in-the-selection / completing / selectable - each with its own visual
//! treatment, so the interaction design can be seen and compared in one window.
//!
//! Run with: `cargo run -p cardtable --example selection_states`

fn main() {
    cardtable::run_selection_states();
}
