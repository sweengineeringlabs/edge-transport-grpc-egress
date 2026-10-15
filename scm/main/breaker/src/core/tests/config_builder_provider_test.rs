#[cfg(test)]
mod tests {
    use crate::api::{ConfigBuilderProvider, ConfigBuilderRequest, GrpcBreakerSvc};

    #[derive(serde::Deserialize, Default, PartialEq, Debug)]
    struct AbsentSectionProbe {
        marker: bool,
    }

    #[test]
    fn test_build_loader_produces_a_working_loader() {
        let resp = GrpcBreakerSvc
            .create_config_builder(ConfigBuilderRequest)
            .expect("infallible");
        let loader = resp
            .builder
            .build_loader()
            .expect("pre-seeded builder must build a valid loader");
        let err = loader
            .load_section::<AbsentSectionProbe>("breaker_core_tests_probe_section_absent")
            .expect_err("no config directory exists in the test environment");
        assert!(err
            .to_string()
            .contains("breaker_core_tests_probe_section_absent"));
    }
}
