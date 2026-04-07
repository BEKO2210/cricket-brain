// SPDX-License-Identifier: AGPL-3.0-only
use cricket_brain::brain::BrainConfig;
use cricket_brain::token::TokenVocabulary;
use proptest::prelude::*;

proptest! {
    #[test]
    fn token_vocab_never_panics_on_generated_inputs(
        labels in prop::collection::vec("[A-Z]{1,4}", 1..16),
        freq_min in 0.0f32..20000.0f32,
        freq_span in 0.0f32..20000.0f32,
    ) {
        let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        let freq_max = freq_min + freq_span;
        let vocab = TokenVocabulary::new(&refs, freq_min, freq_max);
        prop_assert_eq!(vocab.len(), refs.len());
    }

    #[test]
    fn brain_config_validation_is_panic_free(
        n_neurons in 0usize..4096usize,
        min_freq in -1000.0f32..10000.0f32,
        max_freq in -1000.0f32..10000.0f32,
        sample_rate_hz in 0u32..48000u32,
        min_activation_threshold in -1.0f32..2.0f32,
        agc_rate in -1.0f32..2.0f32,
        adaptive in any::<bool>(),
        seed in any::<u64>(),
        privacy_mode in any::<bool>(),
    ) {
        let cfg = BrainConfig {
            n_neurons,
            min_freq,
            max_freq,
            k_connections: None,
            sample_rate_hz,
            min_activation_threshold,
            adaptive_sensitivity: adaptive,
            agc_rate,
            seed,
            privacy_mode,
        };
        let _ = cfg.validate();
    }
}
