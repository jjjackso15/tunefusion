//! Vocal isolation using Demucs (Python ML model).
//!
//! Demucs separates audio into stems: vocals, drums, bass, other.
//! We use the vocals stem for more accurate pitch detection.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get the user's home directory.
fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// Debug information about the Python/Demucs environment.
#[derive(Debug, Clone)]
pub struct DemucsDebugInfo {
    pub home_dir: Option<String>,
    pub python_version: Option<String>,
    pub pythonpath_set: Option<String>,
    pub path_set: Option<String>,
    pub site_packages_found: Vec<String>,
    pub demucs_import_result: String,
    pub demucs_location: Option<String>,
}

/// Get debug information about the Demucs environment.
pub fn debug_demucs_environment() -> DemucsDebugInfo {
    let home = home_dir();
    let home_str = home.as_ref().map(|h| h.display().to_string());

    // Check which site-packages exist
    let mut site_packages_found = Vec::new();
    let mut pythonpath_parts = Vec::new();

    if let Some(ref home_path) = home {
        for version in &["3.14", "3.13", "3.12", "3.11", "3.10", "3.9"] {
            let site_packages = home_path.join(format!(".local/lib/python{}/site-packages", version));
            let exists = site_packages.exists();
            let has_demucs = site_packages.join("demucs").exists();
            site_packages_found.push(format!(
                "python{}: {} (demucs: {})",
                version,
                if exists { "EXISTS" } else { "not found" },
                if has_demucs { "YES" } else { "no" }
            ));
            if exists {
                pythonpath_parts.push(site_packages.to_string_lossy().to_string());
            }
        }
    }

    let pythonpath_set = if pythonpath_parts.is_empty() {
        None
    } else {
        Some(pythonpath_parts.join(":"))
    };

    // Get the clean PATH we would set
    let path_set = Some(build_clean_path());

    // Find which system Python we'd use
    let system_python = find_system_python();
    site_packages_found.insert(0, format!("System Python: {}", system_python));

    // Get Python version using system Python
    let python_version = Command::new(&system_python)
        .env_remove("PYTHONHOME")
        .args(["--version"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    // Try to import demucs with our clean environment
    let mut cmd = python_command();
    cmd.args(["-c", "import demucs; print(demucs.__file__)"]);

    let (demucs_import_result, demucs_location) = match cmd.output() {
        Ok(o) => {
            if o.status.success() {
                let location = String::from_utf8_lossy(&o.stdout).trim().to_string();
                ("SUCCESS".to_string(), Some(location))
            } else {
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                (format!("FAILED: {}", stderr), None)
            }
        }
        Err(e) => (format!("ERROR running python: {}", e), None),
    };

    DemucsDebugInfo {
        home_dir: home_str,
        python_version,
        pythonpath_set,
        path_set,
        site_packages_found,
        demucs_import_result,
        demucs_location,
    }
}

/// Find the system Python executable (avoiding AppImage-bundled Python).
fn find_system_python() -> String {
    // Check common system Python locations in order of preference
    let candidates = [
        "/usr/bin/python3",
        "/usr/bin/python3.14",
        "/usr/bin/python3.13",
        "/usr/bin/python3.12",
        "/usr/bin/python3.11",
        "/usr/bin/python3.10",
        "/bin/python3",
    ];

    for candidate in &candidates {
        if std::path::Path::new(candidate).exists() {
            println!("[Demucs] Using system Python: {}", candidate);
            return candidate.to_string();
        }
    }

    // Fallback to PATH-based python3 (may not work in AppImage)
    println!("[Demucs] WARNING: No system Python found, falling back to 'python3'");
    "python3".to_string()
}

/// Build a Python command with proper environment for user packages.
/// This carefully avoids AppImage environment pollution.
fn python_command() -> Command {
    let python_path = find_system_python();
    let mut cmd = Command::new(&python_path);

    // CRITICAL: Remove any PYTHONHOME set by AppImage - this breaks system Python
    cmd.env_remove("PYTHONHOME");

    // Remove AppImage-specific variables that might interfere
    cmd.env_remove("APPIMAGE");
    cmd.env_remove("APPDIR");
    cmd.env_remove("OWD");

    // Build a clean PATH that prioritizes system directories
    let clean_path = build_clean_path();
    println!("[Demucs] Setting clean PATH: {}", clean_path);
    cmd.env("PATH", &clean_path);

    // Add user site-packages to PYTHONPATH (without AppImage paths)
    if let Some(home) = home_dir() {
        let mut pythonpaths = Vec::new();
        for version in &["3.14", "3.13", "3.12", "3.11", "3.10", "3.9"] {
            let site_packages = home.join(format!(".local/lib/python{}/site-packages", version));
            if site_packages.exists() {
                pythonpaths.push(site_packages.to_string_lossy().to_string());
                println!("[Demucs] Found site-packages: {}", site_packages.display());
            }
        }

        if !pythonpaths.is_empty() {
            let pythonpath = pythonpaths.join(":");
            println!("[Demucs] Setting PYTHONPATH: {}", pythonpath);
            // Don't inherit existing PYTHONPATH - it may contain AppImage junk
            cmd.env("PYTHONPATH", pythonpath);
        } else {
            println!("[Demucs] WARNING: No site-packages directories found in {}", home.display());
        }
    } else {
        println!("[Demucs] WARNING: HOME environment variable not set!");
    }

    cmd
}

/// Build a clean PATH without AppImage mount directories.
fn build_clean_path() -> String {
    let mut paths = Vec::new();

    // Add user's local bin first
    if let Some(home) = home_dir() {
        let local_bin = home.join(".local/bin");
        if local_bin.exists() {
            paths.push(local_bin.to_string_lossy().to_string());
        }
    }

    // Add standard system paths
    let system_paths = [
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/local/sbin",
        "/usr/sbin",
        "/sbin",
    ];

    for p in &system_paths {
        if std::path::Path::new(p).exists() {
            paths.push(p.to_string());
        }
    }

    // Also include linuxbrew if present (common on Fedora/Bazzite)
    let linuxbrew = "/home/linuxbrew/.linuxbrew/bin";
    if std::path::Path::new(linuxbrew).exists() {
        paths.push(linuxbrew.to_string());
    }

    paths.join(":")
}

/// Check if Demucs is available on the system.
pub fn is_demucs_available() -> bool {
    println!("[Demucs] Checking if Demucs is available...");
    println!("[Demucs] HOME={:?}", std::env::var("HOME"));

    let output = python_command()
        .args(["-c", "import demucs; print('OK:', demucs.__file__)"])
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            println!("[Demucs] Exit code: {:?}", o.status.code());
            println!("[Demucs] stdout: {}", stdout);
            println!("[Demucs] stderr: {}", stderr);

            if o.status.success() {
                println!("[Demucs] Demucs is available!");
                true
            } else {
                eprintln!("[Demucs] Import failed: {}", stderr);
                false
            }
        }
        Err(e) => {
            eprintln!("[Demucs] Failed to run python3: {}", e);
            false
        }
    }
}

/// Get Demucs version if available.
pub fn demucs_version() -> Option<String> {
    let output = python_command()
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
            "Demucs is not installed or not found. Install with:\n\
             python3 -m pip install --user demucs\n\n\
             If already installed, make sure it's accessible to the app."
        );
    }

    // Create output directory
    std::fs::create_dir_all(&config.output_dir)
        .context("Failed to create output directory for vocal isolation")?;

    let audio_path_str = audio_path.to_string_lossy();
    let output_dir_str = config.output_dir.to_string_lossy();

    // Build demucs command with proper environment
    let mut cmd = python_command();
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
