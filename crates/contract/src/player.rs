use std::fmt;

/// Identifies a player by seat index, counting from zero.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct PlayerId(pub usize);

impl PlayerId {
    /// The zero-based seat index.
    pub fn index(self) -> usize {
        self.0
    }
}

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Player {}", self.0)
    }
}
