use cricket_brain::sequence::SequencePredictor;
use cricket_brain::token::TokenVocabulary;

/// Well-spaced vocabulary: 5 tokens across 2000–8000 Hz (1500 Hz spacing).
/// This ensures clean Gaussian separation (10% bandwidth ≈ 200–800 Hz).
fn wide_vocab() -> TokenVocabulary {
    TokenVocabulary::new(&["A", "B", "C", "D", "E"], 2000.0, 8000.0)
}

fn feed_token(pred: &mut SequencePredictor, label: &str, hold_ms: usize, gap_ms: usize) {
    let freq = pred.vocab.get(label).unwrap().freq;
    for _ in 0..hold_ms {
        pred.step(freq);
    }
    for _ in 0..gap_ms {
        pred.step(0.0);
    }
}

#[test]
fn predict_next_in_sequence() {
    let vocab = wide_vocab();
    let mut pred = SequencePredictor::with_params(vocab, 8, 300);
    pred.register_pattern("abc", &["A", "B", "C"]);

    // Feed "A" → should predict "B"
    feed_token(&mut pred, "A", 50, 40);
    let p = pred.predict();
    assert!(p.is_some(), "Should predict after first token");
    assert_eq!(p.unwrap().label, "B");

    // Feed "B" → should predict "C"
    feed_token(&mut pred, "B", 50, 40);
    let p = pred.predict();
    assert!(p.is_some(), "Should predict after second token");
    assert_eq!(p.unwrap().label, "C");
}

#[test]
fn confidence_increases_with_match_length() {
    let vocab = wide_vocab();
    let mut pred = SequencePredictor::with_params(vocab, 8, 300);
    pred.register_pattern("long", &["A", "B", "C", "D", "E"]);

    feed_token(&mut pred, "A", 50, 40);
    let c1 = pred.predict().map(|p| p.confidence).unwrap_or(0.0);

    feed_token(&mut pred, "B", 50, 40);
    let c2 = pred.predict().map(|p| p.confidence).unwrap_or(0.0);

    feed_token(&mut pred, "C", 50, 40);
    let c3 = pred.predict().map(|p| p.confidence).unwrap_or(0.0);

    assert!(c2 > c1, "Confidence should increase: {c1} → {c2}");
    assert!(c3 > c2, "Confidence should increase: {c2} → {c3}");
}

#[test]
fn competing_patterns_both_tracked() {
    let vocab = wide_vocab();
    let mut pred = SequencePredictor::with_params(vocab, 8, 300);
    pred.register_pattern("abcd", &["A", "B", "C", "D"]);
    pred.register_pattern("abce", &["A", "B", "C", "E"]);

    // Feed "A", "B" — both patterns should be active
    feed_token(&mut pred, "A", 50, 40);
    feed_token(&mut pred, "B", 50, 40);

    let all = pred.predict_all();
    assert!(
        all.len() >= 2,
        "Both patterns should be active after A,B (got {})",
        all.len()
    );

    // Both should predict "C" as next
    assert!(
        all.iter().all(|p| p.label == "C"),
        "Both patterns expect 'C' after A,B"
    );
}

#[test]
fn patterns_diverge_after_shared_prefix() {
    let vocab = wide_vocab();
    let mut pred = SequencePredictor::with_params(vocab, 8, 300);
    pred.register_pattern("abcd", &["A", "B", "C", "D"]);
    pred.register_pattern("abce", &["A", "B", "C", "E"]);

    feed_token(&mut pred, "A", 50, 40);
    feed_token(&mut pred, "B", 50, 40);
    feed_token(&mut pred, "C", 50, 40);

    // After A,B,C: "abcd" predicts "D", "abce" predicts "E"
    let all = pred.predict_all();
    let labels: Vec<&str> = all.iter().map(|p| p.label.as_str()).collect();

    assert!(
        labels.contains(&"D") || labels.contains(&"E"),
        "Should predict 'D' (abcd) or 'E' (abce), got: {labels:?}"
    );
}

#[test]
fn no_prediction_for_unknown_sequence() {
    let vocab = wide_vocab();
    let mut pred = SequencePredictor::with_params(vocab, 8, 300);
    pred.register_pattern("abc", &["A", "B", "C"]);

    // Feed "D", "E" — no pattern starts with D
    feed_token(&mut pred, "D", 50, 40);
    feed_token(&mut pred, "E", 50, 40);

    assert!(
        pred.predict().is_none(),
        "Should have no prediction for unknown sequence"
    );
}

#[test]
fn reset_clears_all_state() {
    let vocab = wide_vocab();
    let mut pred = SequencePredictor::with_params(vocab, 8, 300);
    pred.register_pattern("abc", &["A", "B", "C"]);

    feed_token(&mut pred, "A", 50, 40);
    assert!(pred.predict().is_some());

    pred.reset();
    assert!(pred.predict().is_none(), "Reset should clear predictions");
    assert!(pred.token_history.is_empty(), "Reset should clear history");
}

#[test]
fn history_tracks_detected_tokens() {
    let vocab = wide_vocab();
    let mut pred = SequencePredictor::with_params(vocab, 8, 300);
    pred.register_pattern("abc", &["A", "B", "C"]);

    feed_token(&mut pred, "A", 50, 40);
    feed_token(&mut pred, "B", 50, 40);

    let history = pred.history_labels();
    assert!(
        history.contains(&"A".to_string()),
        "History should contain A"
    );
    assert!(
        history.contains(&"B".to_string()),
        "History should contain B"
    );
}

#[test]
fn weighted_patterns() {
    let vocab = wide_vocab();
    let mut pred = SequencePredictor::with_params(vocab, 8, 300);
    pred.register_weighted_pattern("important", &["A", "B", "C"], 2.0);
    pred.register_weighted_pattern("normal", &["A", "B", "D"], 1.0);

    feed_token(&mut pred, "A", 50, 40);
    feed_token(&mut pred, "B", 50, 40);

    // "important" (weight=2.0) should beat "normal" (weight=1.0)
    let p = pred.predict().unwrap();
    assert_eq!(
        p.pattern_name, "important",
        "Higher-weighted pattern should win"
    );
}
