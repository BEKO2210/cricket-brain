//! # Cricket-Brain
//!
//! A biomorphic AI inference engine based on the Münster model of cricket hearing.
//!
//! Cricket-Brain uses **delay-line coincidence detection** for pattern recognition.
//! No matrix multiplication, no CUDA, no weights — just biologically-inspired
//! resonators connected by delay synapses.
//!
//! ## Features
//!
//! - **v0.1**: Morse code recognition with 5-neuron cricket circuit
//! - **v0.2**: Multi-frequency token recognition via parallel resonator banks
//! - **v0.3**: Sequence prediction using delay-line pattern memory
//!
//! ## Quick Start
//!
//! ```rust
//! use cricket_brain::brain::CricketBrain;
//! use cricket_brain::patterns::{encode_morse, MORSE_FREQ};
//!
//! let mut brain = CricketBrain::new();
//! let signal = encode_morse("SOS");
//!
//! for &(freq, duration) in &signal {
//!     for _ in 0..duration {
//!         let output = brain.step(freq);
//!         if output > 0.0 {
//!             println!("Spike! amplitude={output:.3}");
//!         }
//!     }
//! }
//! ```

pub mod brain;
pub mod neuron;
pub mod patterns;
pub mod resonator_bank;
pub mod sequence;
pub mod synapse;
pub mod token;
