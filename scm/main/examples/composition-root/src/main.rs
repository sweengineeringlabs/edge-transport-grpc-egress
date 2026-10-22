//! Runs every scenario of the composition root and prints one line per scenario.

use grpc_egress_example_composition_root::ScenarioSuite;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    ScenarioSuite::new().run().await
}
