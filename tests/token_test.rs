// SPDX-License-Identifier: AGPL-3.0-only
use cricket_brain::resonator_bank::ResonatorBank;
use cricket_brain::token::TokenVocabulary;

#[test]
fn alphabet_vocab_covers_all_letters() {
    let vocab = TokenVocabulary::from_alphabet();
    for ch in 'A'..='Z' {
        let label = ch.to_string();
        assert!(
            vocab.get(&label).is_some(),
            "Missing letter '{ch}' in alphabet vocab"
        );
    }
    assert!(vocab.get(" ").is_some(), "Missing space in alphabet vocab");
}

#[test]
fn tokens_have_unique_frequencies() {
    let vocab = TokenVocabulary::from_alphabet();
    for i in 0..vocab.len() {
        for j in (i + 1)..vocab.len() {
            let fi = vocab.tokens[i].freq;
            let fj = vocab.tokens[j].freq;
            assert!(
                (fi - fj).abs() > 1.0,
                "Tokens {} and {} have near-identical frequencies: {fi} vs {fj}",
                vocab.tokens[i].label,
                vocab.tokens[j].label
            );
        }
    }
}

#[test]
fn bank_detects_correct_token() {
    // Use widely-spaced tokens to guarantee clean discrimination
    let vocab = TokenVocabulary::new(&["alpha", "beta", "gamma"], 2000.0, 8000.0);
    let mut bank = ResonatorBank::new(&vocab);

    let beta_freq = vocab.get("beta").unwrap().freq;

    // Feed beta frequency for 30 steps
    let mut beta_fired = false;
    let mut others_fired = false;

    for _ in 0..30 {
        let act = bank.step(beta_freq);
        if act[1] > 0.0 {
            beta_fired = true;
        }
        if act[0] > 0.0 || act[2] > 0.0 {
            others_fired = true;
        }
    }

    assert!(beta_fired, "Beta channel should fire for beta frequency");
    assert!(
        !others_fired,
        "Other channels should NOT fire for beta frequency"
    );
}

#[test]
fn bank_silence_produces_no_spikes() {
    let vocab = TokenVocabulary::from_labels(&["X", "Y", "Z"]);
    let mut bank = ResonatorBank::new(&vocab);

    for _ in 0..50 {
        let act = bank.step(0.0);
        for &a in &act {
            assert_eq!(a, 0.0, "No spikes during silence");
        }
    }
}

#[test]
fn bank_switching_between_tokens() {
    let vocab = TokenVocabulary::new(&["lo", "hi"], 2000.0, 7000.0);
    let mut bank = ResonatorBank::new(&vocab);

    // Feed "lo" then silence then "hi"
    let lo_freq = vocab.get("lo").unwrap().freq;
    let hi_freq = vocab.get("hi").unwrap().freq;

    // Phase 1: lo
    for _ in 0..30 {
        bank.step(lo_freq);
    }
    let act_lo = bank.step(lo_freq);
    assert!(act_lo[0] > 0.0, "Lo should be active");

    // Silence gap
    for _ in 0..30 {
        bank.step(0.0);
    }

    // Phase 2: hi
    for _ in 0..30 {
        bank.step(hi_freq);
    }
    let act_hi = bank.step(hi_freq);
    assert!(act_hi[1] > 0.0, "Hi should be active after switching");
}

#[test]
fn encode_text_roundtrip() {
    let vocab = TokenVocabulary::new(&["A", "B", "C"], 2000.0, 8000.0);
    let mut bank = ResonatorBank::new(&vocab);

    let signal = vocab.encode_text("ABC", 50, 40);

    let mut detected_sequence = Vec::new();
    let mut current_token: Option<usize> = None;
    let mut hold_count = 0;

    for &(freq, duration) in &signal {
        for _ in 0..duration {
            let det = bank.step_detect(freq);
            if det == current_token && det.is_some() {
                hold_count += 1;
            } else if det != current_token {
                if hold_count >= 8 {
                    if let Some(id) = current_token {
                        detected_sequence.push(id);
                    }
                }
                current_token = det;
                hold_count = if det.is_some() { 1 } else { 0 };
            }
        }
    }
    // Flush last token
    if hold_count >= 8 {
        if let Some(id) = current_token {
            detected_sequence.push(id);
        }
    }

    assert_eq!(
        detected_sequence,
        vec![0, 1, 2],
        "Should detect A(0), B(1), C(2) in sequence"
    );
}
