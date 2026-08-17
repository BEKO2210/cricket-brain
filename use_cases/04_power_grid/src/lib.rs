// SPDX-License-Identifier: AGPL-3.0-only
//! Power-grid harmonic & stability triage using CricketBrain.
//!
//! A 4-channel ResonatorBank classifies power-quality (PQ) events into:
//!   - Nominal        (50 Hz fundamental dominant)
//!   - SecondHarmonic (100 Hz — DC offset, transformer saturation, asymmetry)
//!   - ThirdHarmonic  (150 Hz — non-linear loads: rectifiers, VFDs, SMPS)
//!   - FourthHarmonic (200 Hz — switching artefacts, fast EMI)
//!
//! When total channel energy is below the outage threshold, the
//! classifier returns `GridEvent::Outage`. Same architecture as
//! `cricket-brain-marine` (UC03) and `cricket-brain-bearings` (UC02).

pub mod detector;
pub mod grid_signal;
