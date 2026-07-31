//! one container per blueprint run, and the privilege contract that shapes it.
//!
//! blueprints declare *what* they need (`requires { }`); this module decides
//! *how* a given backend supplies it. that split is deliberate — it is what
//! keeps blueprints host-agnostic.

use super::{Backend, BackendKind, LIGHTHOUSE_IMAGE};
use blueprint::executor::context::{HostFacts, Runner};
use blueprint::transpiler::ir::Requirements;
use color_eyre::eyre::{bail, Result};
use std::path::Path;
use std::process::Stdio;

/// where the learner's workspace is mounted inside the guest
pub const GUEST_WORKSPACE: &str = "/workspace";

pub struct Session {
    backend: Backend,
    container: String,
}

impl Session {
    /// start the pinned environment for a task and leave it running.
    pub async fn start(
        backend: Backend,
        task_slug: &str,
        workspace: Option<&Path>,
        port: Option<u16>,
        reqs: Option<&Requirements>,
    ) -> Result<Self> {
        if !backend.kind.is_usable() {
            bail!(
                "{} cannot run these probes — nested user namespace limits break \
                 namespace and cgroup checks.\n  \
                 run `luxctl env up` to use the pinned Lima environment instead.",
                backend.kind.as_str()
            );
        }

        let container = container_name(task_slug);
        // a container left over from an interrupted run would shadow this one
        remove_container(&backend, &container).await;

        let mut args = vec!["run".to_string(), "-d".to_string()];
        args.push("--name".to_string());
        args.push(container.clone());

        if let Some(ws) = workspace {
            args.push("-v".to_string());
            args.push(format!("{}:{}", ws.display(), GUEST_WORKSPACE));
        }

        // ponytail: publishes the one port the blueprint declares. dynamic or
        // ephemeral ports would need probes to run guest-side instead.
        if let Some(p) = port {
            args.push("-p".to_string());
            args.push(format!("{p}:{p}"));
        }

        if let Some(r) = reqs {
            args.extend(run_flags(r, backend.kind));
        }

        args.push(LIGHTHOUSE_IMAGE.to_string());
        args.extend(["sleep".to_string(), "infinity".to_string()]);

        let out = backend.command().args(&args).output().await?;
        if !out.status.success() {
            bail!(
                "could not start the {} environment: {}",
                backend.kind.as_str(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }

        Ok(Session { backend, container })
    }

    /// the command prefix probes are executed behind
    pub fn runner(&self) -> Runner {
        Runner::Linux {
            prefix: vec![
                self.backend.cli.clone(),
                "exec".to_string(),
                "-w".to_string(),
                GUEST_WORKSPACE.to_string(),
                self.container.clone(),
            ],
            env: self.backend.env.clone(),
        }
    }

    /// what the guest actually provides — `requires { }` is judged against this
    pub fn facts(&self) -> HostFacts {
        self.backend.facts.clone()
    }

    pub async fn teardown(self) {
        remove_container(&self.backend, &self.container).await;
    }
}

fn container_name(task_slug: &str) -> String {
    let safe: String = task_slug
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("lux-{}", safe.trim_matches('-'))
}

async fn remove_container(backend: &Backend, name: &str) {
    let _ = backend
        .command()
        .args(["rm", "-f", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
}

/// translate declared requirements into flags for a specific backend.
///
/// pure function — the whole privilege contract is testable without a daemon.
pub fn run_flags(reqs: &Requirements, backend: BackendKind) -> Vec<String> {
    let mut flags = Vec::new();

    // full privilege subsumes individual capabilities, so don't ask for both
    if reqs.privileged {
        flags.push("--privileged".to_string());
    } else {
        for cap in &reqs.caps {
            flags.push(format!("--cap-add={cap}"));
        }
    }

    if reqs.cgroup_v2 {
        flags.push("--cgroupns=host".to_string());
        flags.push("-v".to_string());
        flags.push("/sys/fs/cgroup:/sys/fs/cgroup:rw".to_string());
    }

    if reqs.pid_host {
        flags.push("--pid=host".to_string());
    }

    // podman defaults to a user namespace of its own, which hides the host
    // cgroup hierarchy the probes are trying to read
    if backend == BackendKind::Podman && (reqs.cgroup_v2 || reqs.privileged) {
        flags.push("--userns=host".to_string());
    }

    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reqs() -> Requirements {
        Requirements::default()
    }

    #[test]
    fn empty_requirements_ask_for_nothing() {
        assert!(run_flags(&reqs(), BackendKind::Colima).is_empty());
    }

    #[test]
    fn cgroup_v2_mounts_the_hierarchy_writable() {
        let mut r = reqs();
        r.cgroup_v2 = true;
        let f = run_flags(&r, BackendKind::Colima);
        assert!(f.contains(&"--cgroupns=host".to_string()));
        assert!(f.contains(&"/sys/fs/cgroup:/sys/fs/cgroup:rw".to_string()));
    }

    #[test]
    fn privileged_replaces_individual_caps() {
        let mut r = reqs();
        r.privileged = true;
        r.caps = vec!["SYS_ADMIN".into()];
        let f = run_flags(&r, BackendKind::Lima);
        assert!(f.contains(&"--privileged".to_string()));
        assert!(!f.iter().any(|x| x.starts_with("--cap-add")));
    }

    #[test]
    fn caps_are_added_when_not_privileged() {
        let mut r = reqs();
        r.caps = vec!["SYS_ADMIN".into(), "BPF".into()];
        let f = run_flags(&r, BackendKind::Lima);
        assert!(f.contains(&"--cap-add=SYS_ADMIN".to_string()));
        assert!(f.contains(&"--cap-add=BPF".to_string()));
    }

    #[test]
    fn podman_needs_host_userns_for_cgroup_work() {
        let mut r = reqs();
        r.cgroup_v2 = true;
        assert!(run_flags(&r, BackendKind::Podman).contains(&"--userns=host".to_string()));
        assert!(!run_flags(&r, BackendKind::Colima).contains(&"--userns=host".to_string()));
    }

    #[test]
    fn pid_host_is_opt_in() {
        let mut r = reqs();
        r.pid_host = true;
        assert!(run_flags(&r, BackendKind::Lima).contains(&"--pid=host".to_string()));
        assert!(!run_flags(&reqs(), BackendKind::Lima).contains(&"--pid=host".to_string()));
    }

    #[test]
    fn container_names_are_slug_safe() {
        assert_eq!(
            container_name("build-your-own-docker"),
            "lux-build-your-own-docker"
        );
        assert_eq!(container_name("phase/2 cgroups"), "lux-phase-2-cgroups");
    }
}
