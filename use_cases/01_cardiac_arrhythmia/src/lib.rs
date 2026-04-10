// SPDX-License-Identifier: AGPL-3.0-only
//! Cardiac arrhythmia pre-screening using CricketBrain.
//!
//! Detects rhythm abnormalities (tachycardia, bradycardia, irregular rhythm)
//! from frequency-encoded ECG signals using delay-line coincidence detection.

pub mod detector;
pub mod ecg_signal;
