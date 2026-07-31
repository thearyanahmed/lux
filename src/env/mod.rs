//! the `linux|` runner environment.
//!
//! `linux|` means "run in the pinned Lighthouse Linux environment", on every
//! host — not "run natively on Linux, emulate on mac". That single choice is
//! what makes macOS and Linux reach the same probe results: the pinned
//! environment is the contract, the host is an implementation detail.

pub mod backend;
pub mod preflight;
pub mod session;

pub use backend::{Backend, BackendKind};
pub use preflight::preflight;
pub use session::Session;

/// the pinned Lighthouse Linux image: kernel >=6.1, BTF at
/// /sys/kernel/btf/vmlinux, CAP_BPF.
pub const LIGHTHOUSE_IMAGE: &str = "ghcr.io/projectlighthouse/linux-base:6.1";

/// name of the Lima VM luxctl provisions for kernel-pinned work
pub const LIMA_VM: &str = "lighthouse";

/// how a blueprint's probes should be executed, from its `runner_image`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerImage {
    /// probes run on the host — what luxctl has always done. the language list
    /// is metadata; it does not gate anything.
    Local(Vec<String>),
    /// probes run inside the pinned Lighthouse Linux environment
    Linux(Vec<String>),
    /// a retired sentinel-hosted value (`go`, `docker`,
    /// `lighthouse-labs/os-process-sandbox:v0.1`). runs as `Local` with a
    /// warning until the platform side ports it to `linux|`.
    Retired(String),
}

impl RunnerImage {
    pub fn parse(raw: Option<&str>) -> Self {
        let raw = match raw.map(str::trim) {
            Some(s) if !s.is_empty() => s,
            _ => return RunnerImage::Local(Vec::new()),
        };

        let mut parts = raw.split('|');
        let head = parts.next().unwrap_or_default().trim().to_lowercase();
        let langs: Vec<String> = parts
            .map(|l| l.trim().to_lowercase())
            .filter(|l| !l.is_empty())
            .collect();

        match head.as_str() {
            "local" => RunnerImage::Local(langs),
            "linux" => RunnerImage::Linux(langs),
            _ => RunnerImage::Retired(raw.to_string()),
        }
    }

    pub fn is_linux(&self) -> bool {
        matches!(self, RunnerImage::Linux(_))
    }

    pub fn langs(&self) -> &[String] {
        match self {
            RunnerImage::Local(l) | RunnerImage::Linux(l) => l,
            RunnerImage::Retired(_) => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_local_with_langs() {
        assert_eq!(
            RunnerImage::parse(Some("local|go|rust|c")),
            RunnerImage::Local(vec!["go".into(), "rust".into(), "c".into()])
        );
    }

    #[test]
    fn parses_linux() {
        let r = RunnerImage::parse(Some("linux|rust"));
        assert_eq!(r, RunnerImage::Linux(vec!["rust".into()]));
        assert!(r.is_linux());
    }

    #[test]
    fn bare_local_has_no_langs() {
        assert_eq!(
            RunnerImage::parse(Some("local")),
            RunnerImage::Local(vec![])
        );
    }

    #[test]
    fn absent_is_local() {
        assert_eq!(RunnerImage::parse(None), RunnerImage::Local(vec![]));
        assert_eq!(RunnerImage::parse(Some("  ")), RunnerImage::Local(vec![]));
    }

    #[test]
    fn retired_sentinel_values_are_flagged() {
        assert_eq!(
            RunnerImage::parse(Some("go")),
            RunnerImage::Retired("go".into())
        );
        assert_eq!(
            RunnerImage::parse(Some("lighthouse-labs/os-process-sandbox:v0.1")),
            RunnerImage::Retired("lighthouse-labs/os-process-sandbox:v0.1".into())
        );
        // retired values still run somewhere, just not in the VM
        assert!(!RunnerImage::parse(Some("docker")).is_linux());
    }

    #[test]
    fn case_and_spacing_are_tolerated() {
        assert_eq!(
            RunnerImage::parse(Some(" LINUX | Rust ")),
            RunnerImage::Linux(vec!["rust".into()])
        );
    }
}
