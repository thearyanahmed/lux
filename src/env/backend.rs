//! backend detection.
//!
//! every backend here is driven through the `docker` (or `podman`) CLI — Lima,
//! Colima, OrbStack, Docker Desktop and rootful Docker all speak it, so the
//! choice of backend is really a choice of endpoint. `limactl` is used only to
//! create and start the VM, never on a per-probe path.

use super::{LIGHTHOUSE_IMAGE, LIMA_VM};
use blueprint::executor::context::HostFacts;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    /// full VM with a guest kernel we control — the parity backend
    Lima,
    Colima,
    OrbStack,
    DockerDesktop,
    /// rootful docker or podman on Linux — the native fast path
    Docker,
    Podman,
    /// nested userns limits break namespace and cgroup probes
    PodmanRootless,
}

impl BackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendKind::Lima => "lima",
            BackendKind::Colima => "colima",
            BackendKind::OrbStack => "orbstack",
            BackendKind::DockerDesktop => "docker desktop",
            BackendKind::Docker => "docker",
            BackendKind::Podman => "podman",
            BackendKind::PodmanRootless => "podman (rootless)",
        }
    }

    /// can this backend be given a kernel we pin?
    pub fn kernel_is_ours(self) -> bool {
        matches!(self, BackendKind::Lima)
    }

    /// backends whose kernel tracks their own release cannot be trusted for
    /// eBPF work even when a BTF probe happens to succeed — availability is
    /// version-dependent and the book's determinism contract needs the pinned
    /// kernel. this is why OrbStack and Docker Desktop are gated off that book.
    pub fn btf_is_trustworthy(self) -> bool {
        !matches!(self, BackendKind::OrbStack | BackendKind::DockerDesktop)
    }

    pub fn is_usable(self) -> bool {
        !matches!(self, BackendKind::PodmanRootless)
    }
}

#[derive(Debug, Clone)]
pub struct Backend {
    pub kind: BackendKind,
    /// the CLI that drives it
    pub cli: String,
    /// env to hand every invocation, e.g. DOCKER_HOST for a Lima socket
    pub env: Vec<(String, String)>,
    pub facts: HostFacts,
}

impl Backend {
    pub fn command(&self) -> Command {
        let mut cmd = Command::new(&self.cli);
        cmd.envs(self.env.iter().cloned());
        cmd
    }
}

/// docker and podman both expose `info --format`, but under different field
/// names and nesting. asking each for a tab-separated line in its own dialect
/// is shorter than reconciling two JSON shapes.
const DOCKER_FMT: &str =
    "{{.Name}}\t{{.OperatingSystem}}\t{{.KernelVersion}}\t{{.CgroupVersion}}\t";
const PODMAN_FMT: &str = "{{.Host.Hostname}}\t{{.Host.Distribution.Distribution}}\t{{.Host.Kernel}}\t{{.Host.CgroupVersion}}\t{{.Host.Security.Rootless}}";

#[derive(Debug, Default)]
struct Info {
    name: String,
    operating_system: String,
    kernel_version: String,
    cgroup_version: String,
    rootless: bool,
}

impl Info {
    fn from_line(line: &str) -> Self {
        let f: Vec<&str> = line.trim().split('\t').collect();
        let get = |i: usize| f.get(i).unwrap_or(&"").trim().to_string();
        Info {
            name: get(0),
            operating_system: get(1),
            kernel_version: get(2),
            cgroup_version: get(3),
            rootless: get(4).eq_ignore_ascii_case("true"),
        }
    }
}

/// find a usable backend, preferring the one whose kernel we control.
///
/// returns `None` when nothing is installed — the caller reports the missing
/// *capability* and every way to supply it, never "install Docker Desktop".
pub async fn detect() -> Option<Backend> {
    if let Some(b) = detect_lima().await {
        return Some(b);
    }
    if let Some(b) = detect_via_cli("docker").await {
        return Some(b);
    }
    detect_via_cli("podman").await
}

/// a running Lima VM named `lighthouse` exposes a docker socket we can point at
async fn detect_lima() -> Option<Backend> {
    let out = Command::new("limactl")
        .args(["list", "--format", "{{.Name}} {{.Status}}"])
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }

    let listing = String::from_utf8_lossy(&out.stdout);
    let running = listing.lines().any(|l| {
        let mut f = l.split_whitespace();
        f.next() == Some(LIMA_VM) && f.next().map(str::to_lowercase).as_deref() == Some("running")
    });
    if !running {
        return None;
    }

    let home = dirs::home_dir()?;
    let sock = home
        .join(".lima")
        .join(LIMA_VM)
        .join("sock")
        .join("docker.sock");
    let env = vec![(
        "DOCKER_HOST".to_string(),
        format!("unix://{}", sock.display()),
    )];

    let info = docker_info("docker", &env).await?;
    Some(build_backend(BackendKind::Lima, "docker", env, &info))
}

async fn detect_via_cli(cli: &str) -> Option<Backend> {
    let info = docker_info(cli, &[]).await?;
    let kind = classify(cli, &info);
    Some(build_backend(kind, cli, Vec::new(), &info))
}

async fn docker_info(cli: &str, env: &[(String, String)]) -> Option<Info> {
    let fmt = if cli == "podman" {
        PODMAN_FMT
    } else {
        DOCKER_FMT
    };
    let out = Command::new(cli)
        .args(["info", "--format", fmt])
        .envs(env.iter().cloned())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(Info::from_line(&String::from_utf8_lossy(&out.stdout)))
}

fn classify(cli: &str, info: &Info) -> BackendKind {
    if cli == "podman" {
        return if info.rootless {
            BackendKind::PodmanRootless
        } else {
            BackendKind::Podman
        };
    }

    let haystack = format!("{} {}", info.name, info.operating_system).to_lowercase();
    if haystack.contains("orbstack") {
        BackendKind::OrbStack
    } else if haystack.contains("colima") {
        BackendKind::Colima
    } else if haystack.contains("lima") {
        BackendKind::Lima
    } else if haystack.contains("docker desktop") || haystack.contains("linuxkit") {
        BackendKind::DockerDesktop
    } else {
        BackendKind::Docker
    }
}

fn build_backend(kind: BackendKind, cli: &str, env: Vec<(String, String)>, info: &Info) -> Backend {
    Backend {
        kind,
        cli: cli.to_string(),
        env,
        facts: HostFacts {
            host_os: std::env::consts::OS.to_string(),
            backend: kind.as_str().to_string(),
            kernel: parse_kernel(&info.kernel_version),
            // docker reports "2", podman reports "v2"
            cgroup_v2: info.cgroup_version.trim().trim_start_matches('v') == "2",
            // filled in by probe_guest — docker info cannot answer these
            userns: false,
            btf: false,
        },
    }
}

/// ask the guest what it actually has. one throwaway container answers both
/// questions `docker info` cannot.
pub async fn probe_guest(backend: &mut Backend) {
    let script = "test -e /proc/self/ns/user && echo userns; \
                  test -f /sys/kernel/btf/vmlinux && echo btf";
    let out = backend
        .command()
        .args(["run", "--rm", LIGHTHOUSE_IMAGE, "sh", "-c", script])
        .stderr(Stdio::null())
        .output()
        .await;

    let stdout = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return,
    };

    // ponytail: presence of /proc/self/ns/user, not a full unprivileged-clone
    // check. tighten if a book needs nested userns specifically.
    backend.facts.userns = stdout.contains("userns");
    backend.facts.btf = stdout.contains("btf") && backend.kind.btf_is_trustworthy();
}

/// parse a kernel version triple.
///
/// `upgrade.rs`'s semver parser demands exactly three dot-separated parts, so it
/// rejects every real kernel string: `5.15.0-91-generic`, `6.1.0-rc1`.
pub fn parse_kernel(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches('v');
    let mut parts = s.split(['.', '-', '_', '+']);
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(name: &str, os: &str) -> Info {
        Info {
            name: name.to_string(),
            operating_system: os.to_string(),
            ..Info::default()
        }
    }

    #[test]
    fn info_line_survives_missing_trailing_fields() {
        // docker's format string has no rootless field, so the line is short
        let i = Info::from_line("colima\tUbuntu 23.10\t6.1.0-rc1\t2\t");
        assert_eq!(i.name, "colima");
        assert_eq!(i.cgroup_version, "2");
        assert!(!i.rootless);

        let i = Info::from_line("");
        assert_eq!(i.name, "");
        assert!(!i.rootless);
    }

    #[test]
    fn parses_real_kernel_strings() {
        assert_eq!(parse_kernel("6.1.0"), Some((6, 1, 0)));
        assert_eq!(parse_kernel("5.15.0-91-generic"), Some((5, 15, 0)));
        assert_eq!(parse_kernel("6.1.0-rc1"), Some((6, 1, 0)));
        assert_eq!(parse_kernel("6.6"), Some((6, 6, 0)));
        assert_eq!(parse_kernel(""), None);
    }

    #[test]
    fn kernel_ordering_is_tuple_ordering() {
        assert!(parse_kernel("6.1.0") >= parse_kernel("5.15.0-91-generic"));
        assert!(parse_kernel("5.15.0") < parse_kernel("6.1.0"));
    }

    #[test]
    fn classifies_docker_family_backends() {
        assert_eq!(
            classify("docker", &info("orbstack", "OrbStack")),
            BackendKind::OrbStack
        );
        assert_eq!(
            classify("docker", &info("colima", "Ubuntu 23.10")),
            BackendKind::Colima
        );
        assert_eq!(
            classify("docker", &info("docker-desktop", "Docker Desktop")),
            BackendKind::DockerDesktop
        );
        assert_eq!(
            classify("docker", &info("linuxkit-abc", "Docker Desktop")),
            BackendKind::DockerDesktop
        );
        assert_eq!(
            classify("docker", &info("thinkpad", "Ubuntu 22.04")),
            BackendKind::Docker
        );
    }

    #[test]
    fn rootless_podman_is_detected_and_unusable() {
        let mut i = info("localhost", "Fedora 39");
        i.rootless = true;
        assert_eq!(classify("podman", &i), BackendKind::PodmanRootless);
        assert!(!BackendKind::PodmanRootless.is_usable());
        assert!(BackendKind::Podman.is_usable());
    }

    #[test]
    fn btf_is_gated_on_kernels_we_do_not_pin() {
        assert!(!BackendKind::OrbStack.btf_is_trustworthy());
        assert!(!BackendKind::DockerDesktop.btf_is_trustworthy());
        assert!(BackendKind::Lima.btf_is_trustworthy());
        assert!(BackendKind::Lima.kernel_is_ours());
        assert!(!BackendKind::Colima.kernel_is_ours());
    }
}
