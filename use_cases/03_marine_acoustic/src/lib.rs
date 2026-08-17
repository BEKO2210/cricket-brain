// SPDX-License-Identifier: AGPL-3.0-only
//! Marine acoustic monitoring using CricketBrain.
//!
//! A 4-channel ResonatorBank classifies hydrophone signals into:
//!   - FinWhale  (20 Hz stereotyped pulse)
//!   - BlueWhale (80 Hz A-call tonal)
//!   - ShipNoise (140 Hz cargo-ship cavitation peak)
//!   - Humpback  (200 Hz song unit)
//!
//! When total channel energy is below the ambient threshold, the classifier
//! returns `AcousticEvent::Ambient`. This matches the pattern used by
//! `cricket-brain-bearings` (UC02) and `cricket-brain-cardiac` (UC01).

pub mod acoustic_signal;
pub mod detector;
