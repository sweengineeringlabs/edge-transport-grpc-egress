//! `grpc_egress_example_composition_root` -- what a composition root does with the gRPC egress crates:
//! it loads a channel configuration, assembles a client from it and from options, and wraps the client
//! in the retry and breaker decorators. It uses only the public API of the crates.

#![warn(missing_docs)]
#![deny(unsafe_code)]

mod all_options_scenario;
mod composition_root;
mod decorator_scenario;
mod echo_server;
mod interceptor_scenario;
mod invalid_combination_scenario;
mod loadbalancer_scenario;
mod loaded_config_scenario;
mod observed_effect;
mod order_interceptor;
mod prost_scenario;
mod refusing_interceptor;
mod scenario_report;
mod scenario_suite;

pub use all_options_scenario::AllOptionsScenario;
pub use composition_root::CompositionRoot;
pub use decorator_scenario::DecoratorScenario;
pub use echo_server::EchoServer;
pub use interceptor_scenario::InterceptorScenario;
pub use invalid_combination_scenario::InvalidCombinationScenario;
pub use loadbalancer_scenario::LoadbalancerScenario;
pub use loaded_config_scenario::LoadedConfigScenario;
pub use observed_effect::ObservedEffect;
pub use order_interceptor::OrderInterceptor;
pub use prost_scenario::ProstScenario;
pub use refusing_interceptor::RefusingInterceptor;
pub use scenario_report::ScenarioReport;
pub use scenario_suite::ScenarioSuite;
