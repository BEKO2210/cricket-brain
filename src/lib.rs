//! # Cricket-Brain
//!
//! A biomorphic AI inference engine based on the Münster model of cricket hearing.
//!
//! Cricket-Brain uses **delay-line coincidence detection** for pattern recognition.
//! No matrix multiplication, no CUDA, no weights — just biologically-inspired
//! resonators connected by delay synapses.
//!
//! ## Architecture
//!
//! The standard configuration models 5 neurons from the cricket auditory pathway:
//!
//! | Neuron | Role | Frequency |
//! |--------|------|-----------|
//! | AN1 | Auditory receptor | 4500 Hz |
//! | LN2 | Inhibitory interneuron | 4500 Hz |
//! | LN3 | Excitatory interneuron | 4500 Hz |
//! | LN5 | Inhibitory interneuron | 4500 Hz |
//! | ON1 | Output neuron | 4500 Hz |
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
pub mod synapse;
