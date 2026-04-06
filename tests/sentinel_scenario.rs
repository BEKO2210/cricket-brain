use cricket_brain::brain::{BrainConfig, CricketBrain};
use cricket_brain::sequence::{PredictorConfig, SequencePredictor};
use cricket_brain::token::TokenVocabulary;

#[test]
fn detects_arrhythmia_pattern_with_noise_and_jitter() {
    // Sentinel setup: adaptive sensitivity enabled for noisy environments.
    let mut brain = CricketBrain::new(
        BrainConfig::default()
            .with_adaptive_sensitivity(true)
            .with_min_activation_threshold(0.05),
    )
    .expect("valid brain config");

    let vocab = TokenVocabulary::from_labels(&["N", "A"]);
    let mut predictor = SequencePredictor::new(
        vocab.clone(),
        PredictorConfig::default()
            .with_debounce(6)
            .with_max_pattern_gap(120)
            .with_temporal_tolerance(12),
    )
    .expect("valid predictor config");
    predictor
        .register_pattern("arrhythmia_cycle", &["N", "A", "N", "A", "N"])
        .expect("pattern should register");

    let n_freq = vocab.get("N").expect("N token").freq;
    let a_freq = vocab.get("A").expect("A token").freq;
    let noise_freq = 7_300.0; // out-of-band noise for the 2-token sentinel vocabulary

    // Baseline heartbeat duration = 40ms; here we inject ±10% timing drift.
    let scenario: &[(f32, usize, bool)] = &[
        (noise_freq, 30, false), // ambient background
        (n_freq, 44, true),      // +10% jitter
        (noise_freq, 28, false), // gap with background noise
        (a_freq, 36, true),      // -10% jitter
        (noise_freq, 24, false), // gap with background noise
        (n_freq, 41, true),      // +2.5% jitter
        (noise_freq, 24, false), // gap with background noise
        (a_freq, 39, true),      // -2.5% jitter
        (noise_freq, 20, false), // gap with background noise
    ];

    let mut signal_power = 0.0f32;
    let mut noise_power = 0.0f32;
    let mut signal_samples = 0usize;
    let mut noise_samples = 0usize;

    for (freq, duration, is_signal) in scenario.iter().copied() {
        for _ in 0..duration {
            let brain_freq = if is_signal { 4_500.0 } else { noise_freq };
            let out = brain.step(brain_freq);
            let _ = predictor.step(freq);

            if is_signal {
                signal_power += out * out;
                signal_samples += 1;
            } else {
                noise_power += out * out;
                noise_samples += 1;
            }
        }
    }

    let prediction = predictor
        .predict()
        .expect("expected prediction after N,A,N,A");
    assert_eq!(
        prediction.label, "N",
        "predictor should robustly infer the next beat despite jitter/noise"
    );
    assert!(
        prediction.confidence >= 0.799,
        "expected confidence >= 0.8, got {:.3}",
        prediction.confidence
    );

    let signal_avg = signal_power / signal_samples.max(1) as f32;
    let noise_avg = noise_power / noise_samples.max(1) as f32;
    let snr_db = 10.0 * ((signal_avg + 1e-6) / (noise_avg + 1e-6)).log10();

    // Research log output for sentinel robustness evidence.
    println!(
        "[sentinel] SNR(dB)={snr_db:.3}, signal_avg={signal_avg:.6}, noise_avg={noise_avg:.6}, global_sensitivity={:.3}",
        brain.global_sensitivity
    );
    assert!(
        snr_db > 0.0,
        "SNR should be positive in the sentinel scenario"
    );
}
