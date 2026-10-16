//! `ScenarioSuite` -- runs the six scenarios.

use std::error::Error;
use std::io::Write;
use std::process::ExitCode;

use crate::{
    AllOptionsScenario, DecoratorScenario, InterceptorScenario, InvalidCombinationScenario,
    LoadbalancerScenario, LoadedConfigScenario, ScenarioReport,
};

/// The six scenarios of the composition root, run in order.
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
            DecoratorScenario.run().await?,
            InvalidCombinationScenario.run().await?,
        ])
    }

    /// Run every scenario, write one line each to standard output, and exit with success only when
    /// every effect of every scenario was observed.
    pub async fn run(&self) -> ExitCode {
        self.run_to(&mut std::io::stdout()).await
    }

    /// Run every scenario, write one line each to `out`, and exit with success only when every effect
    /// of every scenario was observed.
    pub async fn run_to(&self, out: &mut impl Write) -> ExitCode {
        match self.run_all().await {
            Ok(reports) => match Self::write_reports(out, &reports) {
                Ok(true) => ExitCode::SUCCESS,
                Ok(false) | Err(_) => ExitCode::FAILURE,
            },
            Err(error) => {
                let _ = writeln!(out, "a scenario could not run: {error}");
                ExitCode::FAILURE
            }
        }
    }

    /// Write the line of each report; whether every effect of every report was observed.
    fn write_reports(out: &mut impl Write, reports: &[ScenarioReport]) -> std::io::Result<bool> {
        let mut all_complete = true;
        for report in reports {
            let observed = report
                .effects
                .iter()
                .filter(|effect| effect.observed)
                .count();
            writeln!(
                out,
                "{}: {observed}/{} effects observed",
                report.name,
                report.effects.len()
            )?;
            for effect in report.effects.iter().filter(|effect| !effect.observed) {
                writeln!(out, "  missing: {}", effect.description)?;
            }
            all_complete &= report.is_complete();
        }
        Ok(all_complete)
    }
}

impl Default for ScenarioSuite {
    fn default() -> Self {
        Self::new()
    }
}
