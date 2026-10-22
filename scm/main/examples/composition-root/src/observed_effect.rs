//! `ObservedEffect` -- one thing a scenario looked for, and whether it was there.

/// One effect a scenario checked: what was looked for, and whether it happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedEffect {
    /// What was looked for.
    pub description: &'static str,
    /// Whether it happened.
    pub observed: bool,
}
