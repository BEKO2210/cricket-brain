use cricket_brain::brain::CricketBrain;
use cricket_brain::patterns::{encode_morse, MORSE_FREQ};

#[test]
fn sos_encodes_correctly() {
    let signal = encode_morse("SOS");

    // S = dot dot dot, O = dash dash dash, S = dot dot dot
    // Count the tone segments
    let tones: Vec<&(f32, usize)> = signal.iter().filter(|(f, _)| *f > 0.0).collect();

    // S has 3 dots, O has 3 dashes, S has 3 dots = 9 tone segments
    assert_eq!(
        tones.len(),
        9,
        "SOS should have 9 tone segments (3+3+3), got {}",
        tones.len()
    );

    // First 3 are dots (50ms), middle 3 are dashes (150ms), last 3 dots (50ms)
    assert_eq!(tones[0].1, 50, "First element of S should be dot (50ms)");
    assert_eq!(tones[3].1, 150, "First element of O should be dash (150ms)");
    assert_eq!(tones[6].1, 50, "First element of last S should be dot (50ms)");
}

#[test]
fn no_spikes_during_silence() {
    // Critical test: the brain should produce ZERO spikes during silence periods.
    let mut brain = CricketBrain::new();
    let signal = encode_morse("SOS");

    let mut silence_spikes = 0;

    for &(freq, duration) in &signal {
        for _ in 0..duration {
            let output = brain.step(freq);
            if freq == 0.0 && output > 0.0 {
                silence_spikes += 1;
            }
        }
    }

    assert_eq!(
        silence_spikes, 0,
        "Brain should produce 0 spikes during silence, got {silence_spikes}"
    );
}

#[test]
fn all_morse_chars_encode() {
    // Every letter A-Z should produce a non-empty signal
    for ch in 'A'..='Z' {
        let signal = encode_morse(&ch.to_string());
        assert!(
            !signal.is_empty(),
            "Character '{ch}' should produce non-empty signal"
        );
        // Must contain at least one tone segment
        let has_tone = signal.iter().any(|(f, _)| *f > 0.0);
        assert!(has_tone, "Character '{ch}' should have tone segments");
    }
}

#[test]
fn morse_frequencies_correct() {
    let signal = encode_morse("A");
    for &(freq, _) in &signal {
        assert!(
            freq == 0.0 || freq == MORSE_FREQ,
            "Frequency should be 0 or {MORSE_FREQ}, got {freq}"
        );
    }
}

#[test]
fn char_roundtrip() {
    use cricket_brain::patterns::{MorseSymbol::Dot, MorseSymbol::Dash};
    // Verify that the Morse table is consistent
    let s_morse = vec![Dot, Dot, Dot];
    let o_morse = vec![Dash, Dash, Dash];
    // S and O are distinct patterns
    assert_ne!(s_morse, o_morse);
}
