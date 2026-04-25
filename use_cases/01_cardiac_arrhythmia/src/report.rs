// SPDX-License-Identifier: AGPL-3.0-only
//! Tiny zero-dependency JSON / CSV writer used by the cardiac
//! benchmarks.
//!
//! Why no `serde_json`? UC01 is a benchmark suite that must be
//! reproducible, fast to compile, and trivial to audit. The output
//! format is small and rigid (flat objects, arrays of flat objects,
//! short strings without unicode escapes), so a 100-line writer
//! beats pulling a serde stack into the build.
//!
//! All result files written here include a header block with
//! `generated_at`, `git_commit` (if available), `command`,
//! `dataset_type`, `seed`, `synthetic_generator_version`, etc., so
//! every CSV / JSON file is self-contained and reviewer-friendly.

use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Version stamp for the synthetic generator. Bump when [`crate::synthetic`]
/// changes in ways that would invalidate older result files.
pub const SYNTHETIC_GENERATOR_VERSION: &str = "0.2.0";

/// Static metadata that every result file carries.
#[derive(Debug, Clone)]
pub struct RunMetadata {
    pub generated_at: String,
    pub git_commit: Option<String>,
    pub command: String,
    pub dataset_type: String,
    pub dataset_name: String,
    pub synthetic_generator_version: &'static str,
    pub seed: u64,
    pub classes: Vec<String>,
    pub window_size_ms: u32,
    pub sample_rate_hz: u32,
    pub limitations: Vec<String>,
}

impl RunMetadata {
    pub fn new(
        command: &str,
        dataset_type: &str,
        dataset_name: &str,
        seed: u64,
        classes: &[&str],
        window_size_ms: u32,
        sample_rate_hz: u32,
    ) -> Self {
        Self {
            generated_at: iso_8601_now(),
            git_commit: detect_git_commit(),
            command: command.to_string(),
            dataset_type: dataset_type.to_string(),
            dataset_name: dataset_name.to_string(),
            synthetic_generator_version: SYNTHETIC_GENERATOR_VERSION,
            seed,
            classes: classes.iter().map(|c| c.to_string()).collect(),
            window_size_ms,
            sample_rate_hz,
            limitations: default_limitations(),
        }
    }
}

fn default_limitations() -> Vec<String> {
    vec![
        "Synthetic ECG only — no real-patient validation yet.".into(),
        "Frequency-encoded inputs; not a clinical waveform analyser.".into(),
        "Rate-regime triage only; no morphology, no AF, no VT, no ST analysis.".into(),
        "Not a medical device. Research / embedded prototype only.".into(),
    ]
}

fn iso_8601_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // y/m/d derivation from epoch seconds — UTC, no leap-second juggling.
    let (y, m, d, hh, mm, ss) = epoch_to_utc(secs);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn epoch_to_utc(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = (secs % 86_400) as u32;
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;

    // Howard Hinnant's date algorithm
    let z = days + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d, hh, mm, ss)
}

fn detect_git_commit() -> Option<String> {
    // Try `git rev-parse --short HEAD` from CWD, falling back to the
    // crate manifest path. We swallow any error silently — missing
    // commit info is not a benchmark failure.
    let out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Serialise a `RunMetadata` into JSON.
pub fn metadata_to_json(meta: &RunMetadata) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    push_kv_str(&mut out, "generated_at", &meta.generated_at, 2);
    push_kv_str_opt(&mut out, "git_commit", meta.git_commit.as_deref(), 2);
    push_kv_str(&mut out, "command", &meta.command, 2);
    push_kv_str(&mut out, "dataset_type", &meta.dataset_type, 2);
    push_kv_str(&mut out, "dataset_name", &meta.dataset_name, 2);
    push_kv_str(
        &mut out,
        "synthetic_generator_version",
        meta.synthetic_generator_version,
        2,
    );
    push_kv_u64(&mut out, "seed", meta.seed, 2);
    push_kv_str_array(&mut out, "classes", &meta.classes, 2);
    push_kv_u64(&mut out, "window_size_ms", meta.window_size_ms as u64, 2);
    push_kv_u64(&mut out, "sample_rate_hz", meta.sample_rate_hz as u64, 2);
    push_kv_str_array(&mut out, "limitations", &meta.limitations, 2);
    // Trim trailing comma/newline produced by helpers
    trim_json_trailing_comma(&mut out);
    out.push_str("\n}");
    out
}

/// One JSON-shaped report file. Constructs `metadata` + a list of
/// pre-rendered metric blocks.
pub struct JsonReport {
    pub metadata: RunMetadata,
    /// Pre-rendered fragments of the form `"key": <value>`.
    pub blocks: Vec<(String, String)>,
}

impl JsonReport {
    pub fn new(metadata: RunMetadata) -> Self {
        Self {
            metadata,
            blocks: Vec::new(),
        }
    }

    pub fn add_block(&mut self, key: &str, value: String) -> &mut Self {
        self.blocks.push((key.to_string(), value));
        self
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n  \"metadata\": ");
        let meta_json = metadata_to_json(&self.metadata);
        // re-indent the metadata block by 2 spaces for nested layout
        for (i, line) in meta_json.lines().enumerate() {
            if i == 0 {
                out.push_str(line);
            } else {
                out.push('\n');
                out.push_str("  ");
                out.push_str(line);
            }
        }
        out.push_str(",\n");
        for (i, (key, value)) in self.blocks.iter().enumerate() {
            out.push_str("  \"");
            out.push_str(key);
            out.push_str("\": ");
            // re-indent multi-line values
            for (j, line) in value.lines().enumerate() {
                if j == 0 {
                    out.push_str(line);
                } else {
                    out.push('\n');
                    out.push_str("  ");
                    out.push_str(line);
                }
            }
            if i + 1 < self.blocks.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push('}');
        out
    }

    pub fn write_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut f = fs::File::create(path)?;
        f.write_all(self.render().as_bytes())?;
        Ok(())
    }
}

/// Quote a string as a minimal JSON string. Backslash, quote and ASCII
/// control characters are escaped; everything else is passed through.
pub fn json_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn push_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push(' ');
    }
}

fn push_kv_str(out: &mut String, key: &str, value: &str, depth: usize) {
    push_indent(out, depth);
    out.push('"');
    out.push_str(key);
    out.push_str("\": ");
    out.push_str(&json_quote(value));
    out.push_str(",\n");
}

fn push_kv_str_opt(out: &mut String, key: &str, value: Option<&str>, depth: usize) {
    push_indent(out, depth);
    out.push('"');
    out.push_str(key);
    out.push_str("\": ");
    match value {
        Some(s) => out.push_str(&json_quote(s)),
        None => out.push_str("null"),
    }
    out.push_str(",\n");
}

fn push_kv_u64(out: &mut String, key: &str, value: u64, depth: usize) {
    push_indent(out, depth);
    out.push('"');
    out.push_str(key);
    out.push_str("\": ");
    out.push_str(&value.to_string());
    out.push_str(",\n");
}

fn push_kv_str_array(out: &mut String, key: &str, value: &[String], depth: usize) {
    push_indent(out, depth);
    out.push('"');
    out.push_str(key);
    out.push_str("\": [");
    for (i, s) in value.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&json_quote(s));
    }
    out.push_str("],\n");
}

fn trim_json_trailing_comma(out: &mut String) {
    // Remove the trailing ",\n" produced by the last push_kv_*.
    while out.ends_with('\n') || out.ends_with(',') {
        out.pop();
    }
}

/// Render a flat float as JSON (`null` if NaN / infinite).
pub fn json_f64(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.6}")
    } else {
        "null".to_string()
    }
}

/// Render a flat key:value pair (no trailing comma) for a tiny inline object.
pub fn json_pair(k: &str, v: &str) -> String {
    format!("\"{k}\": {v}")
}

/// Convert a slice of `(key, json_value)` to a JSON object string.
pub fn json_object(pairs: &[(&str, String)]) -> String {
    let mut out = String::from("{");
    for (i, (k, v)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&json_quote(k));
        out.push_str(": ");
        out.push_str(v);
    }
    out.push('}');
    out
}

/// Convert a vec of `String`s (each itself JSON) to a JSON array.
pub fn json_array(items: &[String]) -> String {
    let mut out = String::from("[");
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(item);
    }
    out.push(']');
    out
}

/// Helper: write a CSV file with a metadata header (`# key: value` lines)
/// followed by the actual CSV body.
pub fn write_csv_with_header(
    path: &Path,
    meta: &RunMetadata,
    csv_body: &str,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(path)?;
    writeln!(f, "# generated_at: {}", meta.generated_at)?;
    if let Some(c) = &meta.git_commit {
        writeln!(f, "# git_commit: {c}")?;
    }
    writeln!(f, "# command: {}", meta.command)?;
    writeln!(f, "# dataset_type: {}", meta.dataset_type)?;
    writeln!(f, "# dataset_name: {}", meta.dataset_name)?;
    writeln!(
        f,
        "# synthetic_generator_version: {}",
        meta.synthetic_generator_version
    )?;
    writeln!(f, "# seed: {}", meta.seed)?;
    writeln!(f, "# sample_rate_hz: {}", meta.sample_rate_hz)?;
    writeln!(f, "# window_size_ms: {}", meta.window_size_ms)?;
    writeln!(f, "# limitations: {}", meta.limitations.join(" | "))?;
    f.write_all(csv_body.as_bytes())?;
    if !csv_body.ends_with('\n') {
        f.write_all(b"\n")?;
    }
    Ok(())
}

/// Reconstruct the exact command-line that launched this benchmark
/// (best effort — the args of `cargo run --release --example X` may
/// be invisible if cargo handed us only `target/release/examples/X`).
pub fn current_command_line() -> String {
    let args: Vec<String> = env::args().collect();
    if args.is_empty() {
        return "(unknown)".to_string();
    }
    args.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_quote_escapes_specials() {
        assert_eq!(json_quote("a\"b"), "\"a\\\"b\"");
        assert_eq!(json_quote("\n"), "\"\\n\"");
    }

    #[test]
    fn metadata_serialises() {
        let meta = RunMetadata::new(
            "cargo run --example x",
            "synthetic",
            "uc01_synth_v0.2",
            42,
            &["Normal", "Tachy", "Brady", "Irregular"],
            1,
            1000,
        );
        let s = metadata_to_json(&meta);
        assert!(s.contains("\"seed\": 42"));
        assert!(s.contains("\"dataset_type\": \"synthetic\""));
    }

    #[test]
    fn json_report_renders() {
        let meta = RunMetadata::new("c", "synthetic", "ds", 1, &["A"], 1, 1000);
        let mut report = JsonReport::new(meta);
        report.add_block("aggregate", json_object(&[("accuracy", json_f64(0.93))]));
        let s = report.render();
        assert!(s.starts_with("{\n  \"metadata\":"));
        assert!(s.contains("\"aggregate\": {"));
    }
}
