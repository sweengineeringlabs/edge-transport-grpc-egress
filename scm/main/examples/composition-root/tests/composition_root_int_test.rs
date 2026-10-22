//! Runs every scenario of the composition root against the in-process peer and asserts the effects the
//! scenarios observed -- not their output text.

use std::error::Error;

use grpc_egress_example_composition_root::{
    AllOptionsScenario, CancellationScenario, DecoratorScenario, GenericClientScenario,
    InterceptorScenario, InvalidCombinationScenario, LoadbalancerScenario, LoadedConfigScenario,
    ScenarioReport, SecurityContextScenario,
};

fn missing(report: &ScenarioReport) -> Vec<&'static str> {
    report
        .effects
        .iter()
        .filter(|effect| !effect.observed)
        .map(|effect| effect.description)
        .collect()
}

fn assert_complete(report: &ScenarioReport, effects: usize) {
    assert_eq!(
        report.effects.len(),
        effects,
        "scenario '{}' must look for {effects} effects",
        report.name
    );
    assert!(
        report.is_complete(),
        "scenario '{}' did not observe: {:?}",
        report.name,
        missing(report)
    );
}

#[tokio::test]
async fn test_run_every_transport_option_scenario_observes_every_effect(
) -> Result<(), Box<dyn Error>> {
    assert_complete(&AllOptionsScenario.run().await?, 8);
    Ok(())
}

#[tokio::test]
async fn test_run_loaded_config_scenario_observes_the_file_and_the_override(
) -> Result<(), Box<dyn Error>> {
    assert_complete(&LoadedConfigScenario.run().await?, 2);
    Ok(())
}

#[tokio::test]
async fn test_run_interceptor_scenario_observes_order_trace_context_and_refusal(
) -> Result<(), Box<dyn Error>> {
    assert_complete(&InterceptorScenario.run().await?, 3);
    Ok(())
}

#[tokio::test]
async fn test_run_loadbalancer_scenario_reaches_both_backends_and_avoids_the_dead_one(
) -> Result<(), Box<dyn Error>> {
    assert_complete(&LoadbalancerScenario.run().await?, 2);
    Ok(())
}

#[tokio::test]
async fn test_run_decorator_scenario_retries_then_opens_the_breaker() -> Result<(), Box<dyn Error>>
{
    assert_complete(&DecoratorScenario.run().await?, 3);
    Ok(())
}

#[tokio::test]
async fn test_run_invalid_combination_scenario_names_the_options_of_every_refusal(
) -> Result<(), Box<dyn Error>> {
    assert_complete(&InvalidCombinationScenario.run().await?, 5);
    Ok(())
}

#[tokio::test]
async fn test_run_generic_client_scenario_passes_every_layer_to_one_generic_function(
) -> Result<(), Box<dyn Error>> {
    assert_complete(&GenericClientScenario.run().await?, 4);
    Ok(())
}

#[tokio::test]
async fn test_run_cancellation_scenario_ends_the_call_on_the_peer_and_on_a_fired_token(
) -> Result<(), Box<dyn Error>> {
    assert_complete(&CancellationScenario.run().await?, 3);
    Ok(())
}

#[tokio::test]
async fn test_run_security_context_scenario_carries_the_context_as_headers(
) -> Result<(), Box<dyn Error>> {
    assert_complete(&SecurityContextScenario.run().await?, 4);
    Ok(())
}

#[test]
fn test_report_with_a_missing_effect_is_not_complete() {
    let mut report = ScenarioReport::new("probe");
    report.record("seen", true);
    report.record("not seen", false);

    assert!(!report.is_complete());
    assert_eq!(missing(&report), vec!["not seen"]);
}

#[test]
fn test_report_without_effects_is_not_complete() {
    assert!(!ScenarioReport::new("empty").is_complete());
}
