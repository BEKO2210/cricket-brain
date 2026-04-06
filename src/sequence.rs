//! Sequence prediction via delay-line pattern memory (v0.3).
//!
//! The [`SequencePredictor`] uses temporal coincidence detection to predict
//! the next token in a sequence. Patterns (N-grams) are registered as chains
//! of delay-line coincidence detectors — no training, no gradients, no weights.
//!
//! # How it works
//!
//! 1. Each registered pattern creates a "detector chain":
//!    token₁ must fire `d₁` ms before token₂, which fires `d₂` ms before token₃, etc.
//!
//! 2. When the beginning of a known pattern is detected in the input stream,
//!    the predictor identifies which pattern is active and returns the expected
//!    next token.
//!
//! 3. Multiple patterns can be active simultaneously — the one with the highest
//!    match confidence wins.
//!
//! This is a train-free associative memory: patterns are stored as topology,
//! not as learned weights.

use crate::resonator_bank::ResonatorBank;
use crate::token::TokenVocabulary;

/// A registered pattern (N-gram) stored as a sequence of token IDs.
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Human-readable name for this pattern.
    pub name: String,
    /// Sequence of token IDs that form the pattern.
    pub token_ids: Vec<usize>,
    /// Confidence weight (higher = preferred when patterns compete).
    pub weight: f32,
}

/// Tracks how far along a pattern has been matched in real-time.
#[derive(Debug, Clone)]
struct PatternMatcher {
    /// Index of the pattern in the predictor's pattern list.
    pattern_idx: usize,
    /// How many tokens of this pattern have been matched so far.
    matched_count: usize,
    /// Timestep when the last token was matched.
    last_match_step: usize,
    /// Maximum allowed gap between consecutive token matches (ms).
    #[allow(dead_code)]
    max_gap: usize,
}

/// A prediction result with the predicted next token and confidence.
#[derive(Debug, Clone, PartialEq)]
pub struct Prediction {
    /// Predicted next token ID.
    pub token_id: usize,
    /// Predicted next token label.
    pub label: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Name of the pattern that generated this prediction.
    pub pattern_name: String,
    /// How many tokens of the pattern have been matched.
    pub matched_length: usize,
}

/// Sequence predictor using delay-line pattern memory.
///
/// Registers patterns (N-grams) and predicts the next token based on
/// temporal coincidence between the input stream and stored patterns.
///
/// # Example
/// ```
/// use cricket_brain::token::TokenVocabulary;
/// use cricket_brain::sequence::SequencePredictor;
///
/// let vocab = TokenVocabulary::from_alphabet();
/// let mut pred = SequencePredictor::new(vocab.clone());
///
/// // Register "HELLO" as a known pattern
/// pred.register_pattern("greeting", &["H", "E", "L", "L", "O"]);
///
/// // Feed "H", "E", "L" → predictor should predict "L" (4th char)
/// let h_freq = vocab.get("H").unwrap().freq;
/// for _ in 0..50 { pred.step(h_freq); }
/// for _ in 0..30 { pred.step(0.0); }  // gap
///
/// let e_freq = vocab.get("E").unwrap().freq;
/// for _ in 0..50 { pred.step(e_freq); }
/// for _ in 0..30 { pred.step(0.0); }
///
/// let l_freq = vocab.get("L").unwrap().freq;
/// for _ in 0..50 { pred.step(l_freq); }
///
/// if let Some(p) = pred.predict() {
///     println!("Predicted: {} (confidence: {:.2})", p.label, p.confidence);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SequencePredictor {
    /// The token vocabulary.
    pub vocab: TokenVocabulary,
    /// Parallel resonator bank for token detection.
    pub bank: ResonatorBank,
    /// Registered patterns.
    pub patterns: Vec<Pattern>,
    /// Active pattern matchers tracking partial matches.
    matchers: Vec<PatternMatcher>,
    /// History of detected token IDs (ring buffer).
    pub token_history: Vec<Option<usize>>,
    /// Maximum history length.
    history_capacity: usize,
    /// Current timestep.
    pub time_step: usize,
    /// Last detected token ID (debounced).
    last_detected: Option<usize>,
    /// How long the current token has been continuously detected.
    detection_hold: usize,
    /// Minimum hold time (ms) before a token detection is confirmed.
    min_hold: usize,
    /// Sliding window of recent detections for majority-vote debounce.
    detection_window: Vec<Option<usize>>,
    /// Size of the majority-vote window.
    window_size: usize,
    /// Maximum gap (ms) between consecutive tokens in a pattern.
    max_pattern_gap: usize,
}

impl SequencePredictor {
    /// Creates a new sequence predictor with the given vocabulary.
    pub fn new(vocab: TokenVocabulary) -> Self {
        let bank = ResonatorBank::new(&vocab);
        SequencePredictor {
            vocab,
            bank,
            patterns: Vec::new(),
            matchers: Vec::new(),
            token_history: Vec::new(),
            history_capacity: 256,
            time_step: 0,
            last_detected: None,
            detection_hold: 0,
            min_hold: 8,
            detection_window: Vec::new(),
            window_size: 12,
            max_pattern_gap: 200,
        }
    }

    /// Creates a predictor with custom timing parameters.
    ///
    /// # Arguments
    /// * `vocab` - Token vocabulary
    /// * `min_hold` - Minimum detections in the sliding window to confirm a token
    /// * `max_pattern_gap` - Maximum ms between consecutive tokens in a pattern
    pub fn with_params(vocab: TokenVocabulary, min_hold: usize, max_pattern_gap: usize) -> Self {
        let bank = ResonatorBank::new(&vocab);
        let window_size = min_hold + 4; // window slightly larger than threshold
        SequencePredictor {
            vocab,
            bank,
            patterns: Vec::new(),
            matchers: Vec::new(),
            token_history: Vec::new(),
            history_capacity: 256,
            time_step: 0,
            last_detected: None,
            detection_hold: 0,
            min_hold,
            detection_window: Vec::new(),
            window_size,
            max_pattern_gap,
        }
    }

    /// Registers a named pattern (N-gram) for prediction.
    ///
    /// # Arguments
    /// * `name` - Human-readable name (e.g., "greeting")
    /// * `labels` - Token labels forming the pattern (e.g., `["H", "E", "L", "L", "O"]`)
    ///
    /// # Panics
    /// Panics if any label is not in the vocabulary.
    pub fn register_pattern(&mut self, name: &str, labels: &[&str]) {
        let token_ids: Vec<usize> = labels
            .iter()
            .map(|&label| {
                self.vocab
                    .get(label)
                    .unwrap_or_else(|| panic!("Token '{label}' not in vocabulary"))
                    .id
            })
            .collect();

        self.patterns.push(Pattern {
            name: name.to_string(),
            token_ids,
            weight: 1.0,
        });
    }

    /// Registers a pattern with a custom weight.
    pub fn register_weighted_pattern(&mut self, name: &str, labels: &[&str], weight: f32) {
        self.register_pattern(name, labels);
        if let Some(p) = self.patterns.last_mut() {
            p.weight = weight;
        }
    }

    /// Processes one timestep with the given input frequency.
    ///
    /// Internally:
    /// 1. Feeds the frequency through the resonator bank
    /// 2. Debounces token detection (requires `min_hold` consecutive ms)
    /// 3. On confirmed detection, updates pattern matchers
    /// 4. Updates token history
    ///
    /// # Returns
    /// The raw activation vector from the resonator bank.
    pub fn step(&mut self, input_freq: f32) -> Vec<f32> {
        let activations = self.bank.step(input_freq);
        self.time_step += 1;

        // Find the currently active token (if any)
        let detected = activations
            .iter()
            .enumerate()
            .filter(|(_, &v)| v > 0.0)
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(id, _)| id);

        // Sliding-window majority-vote debounce:
        // Collect recent detections, confirm when a token reaches min_hold votes.
        self.detection_window.push(detected);
        if self.detection_window.len() > self.window_size {
            self.detection_window.remove(0);
        }

        // Count votes for each token in the window
        let majority = self.majority_token();

        if let Some(token_id) = majority {
            if self.last_detected == Some(token_id) {
                // Same token still dominant — already confirmed
            } else {
                // New token reached majority — confirm it
                self.last_detected = Some(token_id);
                self.on_token_confirmed(token_id);
            }
        } else if detected.is_none() {
            // Silence: clear detection state after window drains
            let any_active = self.detection_window.iter().any(|d| d.is_some());
            if !any_active {
                self.last_detected = None;
            }
        }

        activations
    }

    /// Returns the token with >= min_hold votes in the detection window, if any.
    fn majority_token(&self) -> Option<usize> {
        let mut counts = std::collections::HashMap::new();
        for id in self.detection_window.iter().flatten() {
            *counts.entry(*id).or_insert(0usize) += 1;
        }
        counts
            .into_iter()
            .filter(|&(_, count)| count >= self.min_hold)
            .max_by_key(|&(_, count)| count)
            .map(|(id, _)| id)
    }

    /// Called when a token has been stably detected for min_hold timesteps.
    fn on_token_confirmed(&mut self, token_id: usize) {
        // Add to history
        if self.token_history.len() >= self.history_capacity {
            self.token_history.remove(0);
        }
        self.token_history.push(Some(token_id));

        // Update existing matchers
        let mut new_matchers = Vec::new();
        self.matchers.retain_mut(|m| {
            let pattern = &self.patterns[m.pattern_idx];
            let expected = pattern.token_ids[m.matched_count];
            let gap = self.time_step - m.last_match_step;

            if token_id == expected && gap <= self.max_pattern_gap {
                m.matched_count += 1;
                m.last_match_step = self.time_step;
                // Keep the matcher if pattern isn't fully matched yet
                m.matched_count < pattern.token_ids.len()
            } else if gap > self.max_pattern_gap {
                false // expired
            } else {
                true // keep waiting
            }
        });

        // Start new matchers for patterns that begin with this token
        for (pi, pattern) in self.patterns.iter().enumerate() {
            if pattern.token_ids[0] == token_id {
                // Don't duplicate if we already have a matcher at position 1
                let already_exists = self.matchers.iter().any(|m| {
                    m.pattern_idx == pi && m.matched_count == 1
                });
                if !already_exists {
                    new_matchers.push(PatternMatcher {
                        pattern_idx: pi,
                        matched_count: 1,
                        last_match_step: self.time_step,
                        max_gap: self.max_pattern_gap,
                    });
                }
            }
        }

        self.matchers.extend(new_matchers);
    }

    /// Returns the best prediction for the next token, if any pattern is active.
    ///
    /// Examines all active pattern matchers and returns the prediction from
    /// the one with the highest confidence (longest match * weight).
    pub fn predict(&self) -> Option<Prediction> {
        let mut best: Option<Prediction> = None;

        for matcher in &self.matchers {
            let pattern = &self.patterns[matcher.pattern_idx];
            if matcher.matched_count >= pattern.token_ids.len() {
                continue; // fully matched, no next token to predict
            }

            let next_token_id = pattern.token_ids[matcher.matched_count];
            let progress = matcher.matched_count as f32 / pattern.token_ids.len() as f32;
            let confidence = progress * pattern.weight;

            let label = self
                .vocab
                .get_by_id(next_token_id)
                .map(|t| t.label.clone())
                .unwrap_or_else(|| "?".to_string());

            let pred = Prediction {
                token_id: next_token_id,
                label,
                confidence,
                pattern_name: pattern.name.clone(),
                matched_length: matcher.matched_count,
            };

            if best.as_ref().map_or(true, |b| confidence > b.confidence) {
                best = Some(pred);
            }
        }

        best
    }

    /// Returns all active predictions ranked by confidence.
    pub fn predict_all(&self) -> Vec<Prediction> {
        let mut predictions = Vec::new();

        for matcher in &self.matchers {
            let pattern = &self.patterns[matcher.pattern_idx];
            if matcher.matched_count >= pattern.token_ids.len() {
                continue;
            }

            let next_token_id = pattern.token_ids[matcher.matched_count];
            let progress = matcher.matched_count as f32 / pattern.token_ids.len() as f32;
            let confidence = progress * pattern.weight;

            let label = self
                .vocab
                .get_by_id(next_token_id)
                .map(|t| t.label.clone())
                .unwrap_or_else(|| "?".to_string());

            predictions.push(Prediction {
                token_id: next_token_id,
                label,
                confidence,
                pattern_name: pattern.name.clone(),
                matched_length: matcher.matched_count,
            });
        }

        predictions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        predictions
    }

    /// Returns the number of currently active pattern matchers.
    pub fn active_matchers(&self) -> usize {
        self.matchers.len()
    }

    /// Returns the token detection history as labels.
    pub fn history_labels(&self) -> Vec<String> {
        self.token_history
            .iter()
            .filter_map(|opt| {
                opt.and_then(|id| self.vocab.get_by_id(id).map(|t| t.label.clone()))
            })
            .collect()
    }

    /// Resets all state (bank, matchers, history).
    pub fn reset(&mut self) {
        self.bank.reset();
        self.matchers.clear();
        self.token_history.clear();
        self.time_step = 0;
        self.last_detected = None;
        self.detection_hold = 0;
        self.detection_window.clear();
    }

    /// Total neuron count across all resonator channels.
    pub fn total_neurons(&self) -> usize {
        self.bank.total_neurons()
    }

    /// Approximate memory usage in bytes.
    pub fn memory_usage_bytes(&self) -> usize {
        let bank_mem = self.bank.memory_usage_bytes();
        let pattern_mem: usize = self
            .patterns
            .iter()
            .map(|p| p.token_ids.len() * 8 + p.name.len() + 32)
            .sum();
        let matcher_mem = self.matchers.len() * std::mem::size_of::<PatternMatcher>();
        let history_mem = self.token_history.capacity() * std::mem::size_of::<Option<usize>>();
        bank_mem + pattern_mem + matcher_mem + history_mem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a well-spaced vocabulary for testing (avoids cross-activation).
    fn test_vocab() -> TokenVocabulary {
        // 5 tokens with ~1200 Hz spacing — far beyond the 10% Gaussian width
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
    fn test_pattern_registration() {
        let vocab = test_vocab();
        let mut pred = SequencePredictor::new(vocab);
        pred.register_pattern("abc", &["A", "B", "C", "D", "E"]);
        assert_eq!(pred.patterns.len(), 1);
        assert_eq!(pred.patterns[0].token_ids.len(), 5);
    }

    #[test]
    fn test_single_token_detection() {
        let vocab = test_vocab();
        let mut pred = SequencePredictor::new(vocab);
        pred.register_pattern("ab", &["A", "B"]);

        let a_freq = pred.vocab.get("A").unwrap().freq;
        for _ in 0..50 {
            pred.step(a_freq);
        }

        assert!(
            !pred.token_history.is_empty(),
            "Token should be detected after 50ms"
        );
    }

    #[test]
    fn test_prediction_after_partial_match() {
        let vocab = test_vocab();
        let mut pred = SequencePredictor::with_params(vocab, 8, 300);
        pred.register_pattern("seq", &["A", "B", "C"]);

        feed_token(&mut pred, "A", 50, 40);

        let prediction = pred.predict();
        assert!(prediction.is_some(), "Should predict after first token");
        let p = prediction.unwrap();
        assert_eq!(p.label, "B", "After 'A', should predict 'B'");
    }

    #[test]
    fn test_prediction_confidence_increases() {
        let vocab = test_vocab();
        let mut pred = SequencePredictor::with_params(vocab, 8, 300);
        pred.register_pattern("seq", &["A", "B", "C", "D", "E"]);

        feed_token(&mut pred, "A", 50, 40);
        let c1 = pred.predict().map(|p| p.confidence).unwrap_or(0.0);

        feed_token(&mut pred, "B", 50, 40);
        let c2 = pred.predict().map(|p| p.confidence).unwrap_or(0.0);

        assert!(
            c2 > c1,
            "Confidence should increase: {c1} → {c2}"
        );
    }

    #[test]
    fn test_no_prediction_without_patterns() {
        let vocab = test_vocab();
        let mut pred = SequencePredictor::new(vocab);

        let a_freq = pred.vocab.get("A").unwrap().freq;
        for _ in 0..50 {
            pred.step(a_freq);
        }

        assert!(pred.predict().is_none());
    }
}
