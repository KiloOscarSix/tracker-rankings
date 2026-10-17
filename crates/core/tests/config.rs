use rankings_core::config::ScoringConfig;
use rankings_core::error::ConfigError;

#[test]
fn default_config_is_valid() {
    assert_eq!(ScoringConfig::default().validate(), Ok(()));
}

#[test]
fn invalid_configs_are_rejected() {
    let cfg = ScoringConfig {
        tier_shares: [0.05, 0.15, 0.25, 0.25, 0.20, 0.0],
        ..ScoringConfig::default()
    };
    assert!(matches!(cfg.validate(), Err(ConfigError::TierShares(_))));

    let cfg = ScoringConfig { confidence: 0.0, ..ScoringConfig::default() };
    assert!(matches!(
        cfg.validate(),
        Err(ConfigError::NotPositive { name: "confidence", .. })
    ));

    let mut cfg = ScoringConfig::default();
    cfg.bradley_terry.max_iterations = 0;
    assert!(matches!(cfg.validate(), Err(ConfigError::ZeroIterations)));
}
