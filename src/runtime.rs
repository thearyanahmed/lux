//! supported runtime definitions for luxctl
//!
//! currently supports Go, Rust, and C runtimes.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

/// supported runtimes for project validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedRuntime {
    Go,
    Rust,
    C,
}

impl SupportedRuntime {
    /// file extension for source files (without dot)
    pub fn extension(&self) -> &'static str {
        match self {
            SupportedRuntime::Go => "go",
            SupportedRuntime::Rust => "rs",
            SupportedRuntime::C => "c",
        }
    }

    /// module/manifest file name
    pub fn module_file(&self) -> &'static str {
        match self {
            SupportedRuntime::Go => "go.mod",
            SupportedRuntime::Rust => "Cargo.toml",
            SupportedRuntime::C => "Makefile",
        }
    }

    /// build command executable
    pub fn build_command(&self) -> &'static str {
        match self {
            SupportedRuntime::Go => "go",
            SupportedRuntime::Rust => "cargo",
            SupportedRuntime::C => "make",
        }
    }

    /// build command arguments
    pub fn build_args(&self) -> Vec<&'static str> {
        match self {
            SupportedRuntime::Go => vec!["build", "."],
            SupportedRuntime::Rust => vec!["build"],
            SupportedRuntime::C => vec![],
        }
    }

    /// runtime name as lowercase string (for CLI args, storage)
    pub fn as_str(&self) -> &'static str {
        match self {
            SupportedRuntime::Go => "go",
            SupportedRuntime::Rust => "rust",
            SupportedRuntime::C => "c",
        }
    }

    /// all supported runtimes
    pub fn all() -> &'static [SupportedRuntime] {
        &[
            SupportedRuntime::Go,
            SupportedRuntime::Rust,
            SupportedRuntime::C,
        ]
    }

    /// detect runtime from workspace directory by checking for module files
    pub fn detect(workspace: &Path) -> Option<SupportedRuntime> {
        for runtime in Self::all() {
            if workspace.join(runtime.module_file()).exists() {
                return Some(*runtime);
            }
        }
        None
    }

    /// check if workspace has source files for this runtime
    pub fn has_source_files(&self, workspace: &Path) -> bool {
        let ext = self.extension();
        std::fs::read_dir(workspace)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.path().extension().map(|x| x == ext).unwrap_or(false))
            })
            .unwrap_or(false)
    }
}

impl fmt::Display for SupportedRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for SupportedRuntime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "go" | "golang" => Ok(SupportedRuntime::Go),
            "rust" | "rs" => Ok(SupportedRuntime::Rust),
            "c" => Ok(SupportedRuntime::C),
            _ => Err(format!(
                "unsupported runtime '{}'. supported: go, rust, c",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension() {
        assert_eq!(SupportedRuntime::Go.extension(), "go");
        assert_eq!(SupportedRuntime::Rust.extension(), "rs");
        assert_eq!(SupportedRuntime::C.extension(), "c");
    }

    #[test]
    fn test_module_file() {
        assert_eq!(SupportedRuntime::Go.module_file(), "go.mod");
        assert_eq!(SupportedRuntime::Rust.module_file(), "Cargo.toml");
        assert_eq!(SupportedRuntime::C.module_file(), "Makefile");
    }

    #[test]
    fn test_build_command() {
        assert_eq!(SupportedRuntime::Go.build_command(), "go");
        assert_eq!(SupportedRuntime::Rust.build_command(), "cargo");
        assert_eq!(SupportedRuntime::C.build_command(), "make");
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            "go".parse::<SupportedRuntime>().unwrap(),
            SupportedRuntime::Go
        );
        assert_eq!(
            "golang".parse::<SupportedRuntime>().unwrap(),
            SupportedRuntime::Go
        );
        assert_eq!(
            "rust".parse::<SupportedRuntime>().unwrap(),
            SupportedRuntime::Rust
        );
        assert_eq!(
            "rs".parse::<SupportedRuntime>().unwrap(),
            SupportedRuntime::Rust
        );
        assert_eq!(
            "c".parse::<SupportedRuntime>().unwrap(),
            SupportedRuntime::C
        );
        assert!("python".parse::<SupportedRuntime>().is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", SupportedRuntime::Go), "go");
        assert_eq!(format!("{}", SupportedRuntime::Rust), "rust");
        assert_eq!(format!("{}", SupportedRuntime::C), "c");
    }

    #[test]
    fn test_all_runtimes() {
        let all = SupportedRuntime::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&SupportedRuntime::Go));
        assert!(all.contains(&SupportedRuntime::Rust));
        assert!(all.contains(&SupportedRuntime::C));
    }
}
