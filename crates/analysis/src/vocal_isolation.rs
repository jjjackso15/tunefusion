//! Vocal isolation using Demucs (Python ML model).
//!
//! Demucs separates audio into stems: vocals, drums, bass, other.
//! We use the vocals stem for more accurate pitch detection.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if Demucs is available on the system.
pub fn is_demucs_available() -> bool {
    Command::new("python3")
        .args(["-c", "import demucs"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get Demucs version if available.
pub fn demucs_version() -> Option<String> {
    let output = Command::new("python3")
        .args(["-c", "import demucs; print(demucs.__version__)"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Configuration for vocal isolation.
#[derive(Debug, Clone)]
pub struct VocalIsolationConfig {
    /// Demucs model to use (htdemucs, htdemucs_ft, mdx_extra, etc.)
    pub model: String,
    /// Output directory for separated stems
    pub output_dir: PathBuf,
    /// Whether to use GPU (if available)
    pub use_gpu: bool,
    /// Number of processing jobs
    pub jobs: u32,
}

impl Default for VocalIsolationConfig {
    fn default() -> Self {
        Self {
            model: "htdemucs".to_string(),
            output_dir: PathBuf::from("/tmp/tunefusion_stems"),
            use_gpu: true,
            jobs: 1,
        }
    }
}

/// Result of vocal isolation.
#[derive(Debug, Clone)]
pub struct VocalIsolationResult {
    /// Path to the isolated vocals audio file
    pub vocals_path: PathBuf,
    /// Path to the accompaniment (everything except vocals)
    pub accompaniment_path: Option<PathBuf>,
    /// Model used for separation
    pub model: String,
}

/// Isolate vocals from an audio file using Demucs.
///
/// Returns the path to the isolated vocals WAV file.
pub fn isolate_vocals(
    audio_path: &Path,
    config: &VocalIsolationConfig,
) -> Result<VocalIsolationResult> {
    if !is_demucs_available() {
        bail!(
            "Demucs is not installed. Install with: pip install demucs\n\
             Or use: pip install torch demucs"
        );
    }

    // Create output directory
    std::fs::create_dir_all(&config.output_dir)
        .context("Failed to create output directory for vocal isolation")?;

    let audio_path_str = audio_path.to_string_lossy();
    let output_dir_str = config.output_dir.to_string_lossy();

    // Build demucs command
    let mut cmd = Command::new("python3");
    cmd.args(["-m", "demucs"]);
    cmd.args(["-n", &config.model]);
    cmd.args(["-o", &output_dir_str]);
    cmd.args(["--two-stems", "vocals"]); // Only separate vocals vs accompaniment

    if !config.use_gpu {
        cmd.arg("--device").arg("cpu");
    }

    cmd.args(["-j", &config.jobs.to_string()]);
    cmd.arg(&*audio_path_str);

    println!("Running Demucs: {:?}", cmd);

    let output = cmd
        .output()
        .context("Failed to execute Demucs")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Demucs failed: {}", stderr);
    }

    // Find the output files
    // Demucs outputs to: {output_dir}/{model}/{track_name}/vocals.wav
    let track_name = audio_path
        .file_stem()
        .context("Invalid audio path")?
        .to_string_lossy();

    let stems_dir = config.output_dir.join(&config.model).join(&*track_name);
    let vocals_path = stems_dir.join("vocals.wav");
    let no_vocals_path = stems_dir.join("no_vocals.wav");

    if !vocals_path.exists() {
        bail!(
            "Demucs completed but vocals file not found at: {}",
            vocals_path.display()
        );
    }

    Ok(VocalIsolationResult {
        vocals_path,
        accompaniment_path: if no_vocals_path.exists() {
            Some(no_vocals_path)
        } else {
            None
        },
        model: config.model.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demucs_detection() {
        // This test just checks if the detection works, not if Demucs is installed
        let available = is_demucs_available();
        println!("Demucs available: {}", available);

        if available {
            let version = demucs_version();
            println!("Demucs version: {:?}", version);
        }
    }
}
