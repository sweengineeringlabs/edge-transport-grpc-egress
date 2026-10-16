//! `ScenarioReport` -- what one scenario observed.

use crate::ObservedEffect;

/// The effects one scenario observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioReport {
    /// The name of the scenario.
    pub name: &'static str,
    /// The effects it looked for, in order.
    pub effects: Vec<ObservedEffect>,
}

impl ScenarioReport {
    /// An empty report for the scenario `name`.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            effects: Vec::new(),
        }
    }

    /// Record that `description` was looked for and `observed` says whether it happened.
    pub fn record(&mut self, description: &'static str, observed: bool) {
        self.effects.push(ObservedEffect {
            description,
            observed,
        });
    }

    /// Whether every effect that was looked for happened.
    pub fn is_complete(&self) -> bool {
        !self.effects.is_empty() && self.effects.iter().all(|effect| effect.observed)
    }
}
