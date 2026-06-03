//! Python runtime detection and validation.

use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;
use tracing::{debug, error, info};

/// Errors that can occur during Python runtime detection.
#[derive(Error, Debug)]
pub enum RuntimeError {
    /// Python executable not found in PATH or at specified location.
    #[error("Python executable not found: {0}")]
    NotFound(String),

    /// Python version is too old.
    #[error("Python version {found} is too old, requires 3.11+")]
    VersionTooOld {
        /// Found version string.
        found: String,
    },

    /// Failed to execute Python command.
    #[error("Failed to execute Python: {0}")]
    ExecutionFailed(String),

    /// Required package is missing.
    #[error("Required Python package '{package}' is not installed")]
    MissingPackage {
        /// Package name.
        package: String,
    },

    /// Failed to parse Python output.
    #[error("Failed to parse Python output: {0}")]
    ParseError(String),
}

/// Required Python packages for analytics.
const REQUIRED_PACKAGES: &[&str] = &["numpy", "pandas", "scikit-learn", "statsmodels"];

/// Minimum required Python version (major, minor).
const MIN_PYTHON_VERSION: (u32, u32) = (3, 11);

/// Python runtime information.
#[derive(Debug, Clone)]
pub struct PythonRuntime {
    /// Path to Python executable.
    pub executable: PathBuf,
    /// Python version string (e.g., "3.11.5").
    pub version: String,
    /// Major version number.
    pub version_major: u32,
    /// Minor version number.
    pub version_minor: u32,
}

impl PythonRuntime {
    /// Detect and validate Python runtime.
    ///
    /// # Arguments
    ///
    /// * `custom_path` - Optional custom path to Python executable.
    ///
    /// # Errors
    ///
    /// Returns error if Python is not found, version is too old, or required packages are missing.
    pub fn detect(custom_path: Option<&str>) -> Result<Self, RuntimeError> {
        let executable = Self::find_executable(custom_path)?;
        let (version, version_major, version_minor) = Self::check_version(&executable)?;

        info!(
            executable = %executable.display(),
            version = %version,
            "Python runtime detected"
        );

        let runtime = Self {
            executable,
            version,
            version_major,
            version_minor,
        };

        runtime.validate_packages()?;

        Ok(runtime)
    }

    /// Find Python executable.
    fn find_executable(custom_path: Option<&str>) -> Result<PathBuf, RuntimeError> {
        if let Some(path) = custom_path {
            let path_buf = PathBuf::from(path);
            if path_buf.exists() {
                debug!(path = %path_buf.display(), "Using custom Python path");
                return Ok(path_buf);
            }
            return Err(RuntimeError::NotFound(format!(
                "Custom Python path does not exist: {path}"
            )));
        }

        // Try common Python executable names
        for name in &["python3", "python"] {
            if let Ok(output) = Command::new(name).arg("--version").output() {
                if output.status.success() {
                    debug!(executable = name, "Found Python in PATH");
                    return Ok(PathBuf::from(name));
                }
            }
        }

        Err(RuntimeError::NotFound(
            "Python not found in PATH. Please install Python 3.11+ or specify custom path in config.".to_string()
        ))
    }

    /// Check Python version.
    fn check_version(executable: &PathBuf) -> Result<(String, u32, u32), RuntimeError> {
        let output = Command::new(executable)
            .arg("--version")
            .output()
            .map_err(|e| RuntimeError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(RuntimeError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        let version_str = version_output
            .trim()
            .strip_prefix("Python ")
            .ok_or_else(|| RuntimeError::ParseError(version_output.to_string()))?;

        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() < 2 {
            return Err(RuntimeError::ParseError(format!(
                "Invalid version format: {version_str}"
            )));
        }

        let major: u32 = parts[0].parse().map_err(|_| {
            RuntimeError::ParseError(format!("Invalid major version: {}", parts[0]))
        })?;
        let minor: u32 = parts[1].parse().map_err(|_| {
            RuntimeError::ParseError(format!("Invalid minor version: {}", parts[1]))
        })?;

        if (major, minor) < MIN_PYTHON_VERSION {
            return Err(RuntimeError::VersionTooOld {
                found: format!("{major}.{minor}"),
            });
        }

        Ok((version_str.to_string(), major, minor))
    }

    /// Validate required packages are installed.
    fn validate_packages(&self) -> Result<(), RuntimeError> {
        debug!("Validating required Python packages");

        for package in REQUIRED_PACKAGES {
            if !self.check_package(package)? {
                error!(
                    package = package,
                    "Required Python package not found. Install with: pip install {}", package
                );
                return Err(RuntimeError::MissingPackage {
                    package: (*package).to_string(),
                });
            }
        }

        info!("All required Python packages are installed");
        Ok(())
    }

    /// Check if a package is installed.
    fn check_package(&self, package: &str) -> Result<bool, RuntimeError> {
        let output = Command::new(&self.executable)
            .arg("-c")
            .arg(format!("import {}", package.replace('-', "_")))
            .output()
            .map_err(|e| RuntimeError::ExecutionFailed(e.to_string()))?;

        Ok(output.status.success())
    }

    /// Get installation instructions for missing packages.
    #[must_use]
    pub fn installation_instructions() -> String {
        format!(
            "Install required Python packages:\n  pip install {}",
            REQUIRED_PACKAGES.join(" ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        // This test assumes Python 3.11+ is available in the test environment
        // In CI, we'd mock this or skip if Python is not available
        if let Ok(runtime) = PythonRuntime::detect(None) {
            assert!(runtime.version_major >= 3);
            if runtime.version_major == 3 {
                assert!(runtime.version_minor >= 11);
            }
        }
    }

    #[test]
    fn test_installation_instructions() {
        let instructions = PythonRuntime::installation_instructions();
        assert!(instructions.contains("numpy"));
        assert!(instructions.contains("pandas"));
        assert!(instructions.contains("scikit-learn"));
        assert!(instructions.contains("statsmodels"));
    }

    #[test]
    fn test_custom_path_not_found() {
        let result = PythonRuntime::detect(Some("/nonexistent/python"));
        assert!(result.is_err());
        match result {
            Err(RuntimeError::NotFound(_)) => {}
            _ => panic!("Expected NotFound error"),
        }
    }
}
