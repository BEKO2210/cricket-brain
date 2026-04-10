// SPDX-License-Identifier: AGPL-3.0-only
//! Predictive bearing fault detection using CricketBrain.
//!
//! Uses a ResonatorBank with one 5-neuron channel per fault frequency
//! (BPFO, BPFI, BSF, FTF) to classify bearing health from vibration data.

pub mod detector;
pub mod vibration_signal;
