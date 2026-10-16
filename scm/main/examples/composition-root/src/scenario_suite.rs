//! `ScenarioSuite` -- runs the seven scenarios.

use std::error::Error;
use std::process::ExitCode;

use crate::{
    AllOptionsScenario, DecoratorScenario, InterceptorScenario, InvalidCombinationScenario,
    LoadbalancerScenario, LoadedConfigScenario, ProstScenario, ScenarioReport,
};

/// The seven scenarios of the composition root, run in order.
pub struct ScenarioSuite;

impl ScenarioSuite {
    /// The suite.
    pub fn new() -> Self {
        Self
    }

    /// Run every scenario and return its report.
    ///
    /// # Errors
    ///
    /// The first error of a scenario that could not run at all.
    pub async fn run_all(&self) -> Result<Vec<ScenarioReport>, Box<dyn Error>> {
        Ok(vec![
            AllOptionsScenario.run().await?,
            LoadedConfigScenario.run().await?,
            InterceptorScenario.run().await?,
            LoadbalancerScenario.run().await?,
            ProstScenario.run().await?,
            DecoratorScenario.run().await?,
            InvalidCombinationScenario.run().await?,
        ])
    }

    /// Run every scenario, print one line each, and exit with success only when every effect of every
    /// scenario was observed.
    pub async fn run(&self) -> ExitCode {
        match self.run_all().await {
            Ok(reports) => {
                let mut all_complete = true;
                for report in &reports {
                    let observed = report
                        .effects
                        .iter()
                        .filter(|effect| effect.observed)
                        .count();
                    println!(
                        "{}: {observed}/{} effects observed",
                        report.name,
                        report.effects.len()
                    );
                    for effect in report.effects.iter().filter(|effect| !effect.observed) {
                        println!("  missing: {}", effect.description);
                    }
                    all_complete &= report.is_complete();
                }
                if all_complete {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(error) => {
                eprintln!("a scenario could not run: {error}");
                ExitCode::FAILURE
            }
        }
    }
}

impl Default for ScenarioSuite {
    fn default() -> Self {
        Self::new()
    }
}
