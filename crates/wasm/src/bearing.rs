// SPDX-License-Identifier: AGPL-3.0-only
//! Browser-Bindings für die Wälzlager-Fehlererkennung aus
//! `use_cases/02_predictive_maintenance`.
//!
//! Die Detektionslogik ist identisch zur nativen Referenz-Implementierung
//! (`BearingDetector`): ein [`ResonatorBank`] mit vier Kanälen, die auf die
//! charakteristischen Schadensfrequenzen eines SKF 6205-2RS gestimmt sind.
//! Portiert wurde sie hierher, weil der Use-Case ein eigenständiger Workspace
//! ist und deshalb nicht als Dependency eingebunden werden kann.
//!
//! Gegenüber der nativen Variante kommt eine Batch-API dazu: der Browser
//! schiebt pro Animationsframe einen Block Samples hinein statt 1.000 Mal
//! pro Sekunde die WASM-Grenze zu überqueren.

use cricket_brain::resonator_bank::ResonatorBank;
use cricket_brain::token::TokenVocabulary;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Charakteristische Schadensfrequenzen SKF 6205-2RS bei 1797 U/min (Hz).
const CAL_RPM: f32 = 1797.0;

/// Kanalreihenfolge der Vokabular-Tokens. `TokenVocabulary` verteilt die
/// Frequenzen gleichmäßig über \[min, max\], deshalb müssen die Labels
/// aufsteigend nach Frequenz sortiert sein.
const CHANNELS: [&str; 4] = ["FTF", "BSF", "BPFO", "BPFI"];

/// Klassifikationsergebnis eines Detektionsfensters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FaultType {
    Normal,
    OuterRace,
    InnerRace,
    BallDefect,
}

impl FaultType {
    fn label(self) -> &'static str {
        match self {
            FaultType::Normal => "Normal",
            FaultType::OuterRace => "Außenring (BPFO)",
            FaultType::InnerRace => "Innenring (BPFI)",
            FaultType::BallDefect => "Wälzkörper (BSF)",
        }
    }
}

/// Eine abgeschlossene Fensterklassifikation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowVerdict {
    pub fault: FaultType,
    pub label: &'static str,
    pub confidence: f32,
    pub step: usize,
}

/// Rückgabe eines Batch-Aufrufs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    /// Kanalaktivierung des letzten Samples, Reihenfolge [FTF, BSF, BPFO, BPFI].
    pub activations: Vec<f32>,
    /// Akkumulierte Energie im laufenden Fenster, gleiche Reihenfolge.
    pub energy: Vec<f32>,
    /// Fortschritt im laufenden Fenster, 0.0–1.0.
    pub window_progress: f32,
    /// Alle Fenster, die in diesem Batch abgeschlossen wurden.
    pub verdicts: Vec<WindowVerdict>,
    /// Gesamtzahl verarbeiteter Zeitschritte.
    pub steps: usize,
}

/// Wälzlager-Fehlerdetektor auf Basis eines vierkanaligen [`ResonatorBank`].
#[wasm_bindgen]
pub struct BearingDetector {
    bank: ResonatorBank,
    channel_energy: [f32; 4],
    window_size: usize,
    window_step: usize,
    last_confidence: f32,
    step_count: usize,
    current_rpm: Option<f32>,
}

#[wasm_bindgen]
impl BearingDetector {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BearingDetector {
        let vocab = TokenVocabulary::new(&CHANNELS, 15.0, 162.0);
        BearingDetector {
            bank: ResonatorBank::new(&vocab),
            channel_energy: [0.0; 4],
            window_size: 50,
            window_step: 0,
            last_confidence: 0.0,
            step_count: 0,
            current_rpm: None,
        }
    }

    /// Drehzahlkompensation setzen. Eingangsfrequenzen werden mit
    /// `CAL_RPM / rpm` skaliert, damit Schadensfrequenzen bei beliebiger
    /// Wellendrehzahl auf die kalibrierten Kanäle fallen.
    /// `rpm <= 0` schaltet die Kompensation ab.
    #[wasm_bindgen(js_name = setRpm)]
    pub fn set_rpm(&mut self, rpm: f32) {
        self.current_rpm = if rpm > 0.0 { Some(rpm) } else { None };
    }

    /// Fenstergröße in Zeitschritten (Referenz: 50 = 50 ms bei 1 kHz).
    #[wasm_bindgen(js_name = setWindowSize)]
    pub fn set_window_size(&mut self, size: usize) {
        self.window_size = size.max(1);
    }

    /// Verarbeitet einen Block Frequenz-Samples (Hz, 0.0 = Stille).
    #[wasm_bindgen(js_name = stepBatch)]
    pub fn step_batch(&mut self, freqs: &[f32]) -> Result<JsValue, JsValue> {
        let mut activations = vec![0.0f32; 4];
        let mut verdicts = Vec::new();

        for &freq in freqs {
            let compensated = match self.current_rpm {
                Some(rpm) if freq > 0.0 => freq * (CAL_RPM / rpm),
                _ => freq,
            };
            let outputs = self.bank.step(compensated);
            self.step_count += 1;
            self.window_step += 1;

            for (i, slot) in activations.iter_mut().enumerate() {
                let out = outputs.get(i).copied().unwrap_or(0.0);
                *slot = out;
                if out > 0.0 {
                    self.channel_energy[i] += out;
                }
            }

            if self.window_step >= self.window_size {
                let fault = self.classify();
                self.channel_energy = [0.0; 4];
                self.window_step = 0;
                verdicts.push(WindowVerdict {
                    fault,
                    label: fault.label(),
                    confidence: self.last_confidence,
                    step: self.step_count,
                });
            }
        }

        let result = BatchResult {
            activations,
            energy: self.channel_energy.to_vec(),
            window_progress: self.window_step as f32 / self.window_size as f32,
            verdicts,
            steps: self.step_count,
        };
        serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Setzt Bank und Fensterzustand zurück.
    pub fn reset(&mut self) {
        self.bank.reset();
        self.channel_energy = [0.0; 4];
        self.window_step = 0;
        self.last_confidence = 0.0;
        self.step_count = 0;
    }

    /// Neuronen insgesamt (4 Kanäle × 5).
    #[wasm_bindgen(js_name = totalNeurons)]
    pub fn total_neurons(&self) -> usize {
        self.bank.total_neurons()
    }

    /// Ungefährer RAM-Bedarf der Resonator-Bank in Bytes.
    #[wasm_bindgen(js_name = memoryUsageBytes)]
    pub fn memory_usage_bytes(&self) -> usize {
        self.bank.memory_usage_bytes()
    }

    /// Kanalbeschriftungen in Auswertungsreihenfolge.
    #[wasm_bindgen(js_name = channelLabels)]
    pub fn channel_labels() -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&CHANNELS).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

impl BearingDetector {
    /// Klassifikation über die im Fenster akkumulierte Kanalenergie.
    fn classify(&mut self) -> FaultType {
        let total: f32 = self.channel_energy.iter().sum();
        if total < 0.1 {
            // Keine Energie in irgendeinem Kanal — sicher unauffällig.
            self.last_confidence = 1.0;
            return FaultType::Normal;
        }

        let max_idx = self
            .channel_energy
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(i, _)| i)
            .unwrap_or(0);

        self.last_confidence = (self.channel_energy[max_idx] / total).clamp(0.0, 1.0);

        match max_idx {
            0 => FaultType::Normal, // FTF = normales Verschleißmuster
            1 => FaultType::BallDefect,
            2 => FaultType::OuterRace,
            3 => FaultType::InnerRace,
            _ => FaultType::Normal,
        }
    }
}

impl Default for BearingDetector {
    fn default() -> Self {
        Self::new()
    }
}
