#[cfg(feature = "moonshine")]
mod moonshine_tests {
    use coldvox_stt::plugin::SttPluginFactory;
    use coldvox_stt::plugins::moonshine::MoonshinePluginFactory;
    use std::env;

    #[tokio::test]
    #[ignore = "Requires Python environment and model download"]
    async fn test_moonshine_initialization() {
        use coldvox_stt::types::TranscriptionConfig;

        let factory = MoonshinePluginFactory::new();

        // Skip if requirements not met
        if factory.check_requirements().is_err() {
            println!("Skipping test_moonshine_initialization: requirements not met");
            return;
        }

        let mut plugin = factory.create().expect("Should create plugin");

        let config = TranscriptionConfig {
            enabled: true,
            model_path: "base".to_string(),
            ..Default::default()
        };

        plugin.initialize(config).await.expect("Should initialize successfully");

        assert_eq!(plugin.info().id, "moonshine");
        assert!(plugin.is_available().await.unwrap());
    }

    #[test]
    fn test_moonshine_factory_env_vars() {
        // Safe to run without Python
        env::set_var("MOONSHINE_MODEL", "tiny");
        let factory = MoonshinePluginFactory::new();
        let plugin = factory.create().unwrap();
        assert!(plugin.info().name.contains("Tiny"));
        env::remove_var("MOONSHINE_MODEL");

        env::set_var("MOONSHINE_MODEL", "base");
        let factory = MoonshinePluginFactory::new();
        let plugin = factory.create().unwrap();
        assert!(plugin.info().name.contains("Base"));
        env::remove_var("MOONSHINE_MODEL");
    }
}
