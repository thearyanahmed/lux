//! assert the environment *before* the first probe, not on its failure.
//!
//! the point is the difference between "your colima guest is on 5.15, run
//! `luxctl env upgrade`" and an incomprehensible error on a mac learner's first
//! run.

use super::backend::{self, Backend};
use crate::ui::UI;
use blueprint::transpiler::ir::Requirements;
use color_eyre::eyre::{bail, Result};

/// select a backend and confirm it satisfies what the blueprint declared.
///
/// prints what it found, naming the platform and the backend, then returns the
/// backend to run against.
pub async fn preflight(reqs: Option<&Requirements>) -> Result<Backend> {
    if cfg!(windows) {
        bail!(
            "Windows/WSL2 is not supported in this round — it is a known gap, not a broken \
             install.\n  macOS and Linux are both fully supported today."
        );
    }

    UI::section("Environment");
    UI::ok("platform", Some(std::env::consts::OS));

    let mut be = match backend::detect().await {
        Some(b) => b,
        None => bail!(missing_backend_message()),
    };

    if !be.kind.is_usable() {
        bail!(
            "{} cannot run these probes — nested user namespace limits break namespace and \
             cgroup checks, so results would be wrong rather than merely slow.\n  \
             run `luxctl env up` to use the pinned Lima environment instead.",
            be.kind.as_str()
        );
    }

    backend::probe_guest(&mut be).await;
    UI::ok("backend", Some(be.kind.as_str()));

    let kernel = be
        .facts
        .kernel
        .map(|(a, b, c)| format!("{a}.{b}.{c}"))
        .unwrap_or_else(|| "unknown".to_string());
    UI::ok("kernel", Some(&kernel));

    let reqs = match reqs {
        Some(r) if !r.is_empty() => r,
        _ => return Ok(be),
    };

    for (label, met, detail) in checklist(reqs, &be) {
        if met {
            UI::ok(label, detail.as_deref());
        } else {
            UI::error(label, detail.as_deref());
            bail!(unmet_message(label, &be));
        }
    }

    Ok(be)
}

/// the checks a blueprint's `requires { }` actually asked for. requirements it
/// did not declare are not reported — a blueprint that needs nothing should not
/// be told about BTF.
fn checklist(reqs: &Requirements, be: &Backend) -> Vec<(&'static str, bool, Option<String>)> {
    let mut out = Vec::new();

    if let Some(want) = reqs.kernel {
        let have = be.facts.kernel;
        let met = have.map(|h| h >= want).unwrap_or(false);
        out.push((
            "kernel version",
            met,
            Some(format!("needs >={}.{}.{}", want.0, want.1, want.2)),
        ));
    }
    if reqs.cgroup_v2 {
        out.push((
            "cgroup v2",
            be.facts.cgroup_v2,
            Some("unified hierarchy".into()),
        ));
    }
    if reqs.userns {
        out.push(("user namespaces", be.facts.userns, None));
    }
    if reqs.btf {
        let detail = (!be.kind.btf_is_trustworthy()).then(|| {
            format!(
                "{} tracks its own kernel release — BTF cannot be assumed",
                be.kind.as_str()
            )
        });
        out.push(("BTF", be.facts.btf, detail));
    }

    out
}

fn unmet_message(label: &str, be: &Backend) -> String {
    let fix = if be.kind.kernel_is_ours() {
        "run `luxctl env upgrade` to move it to the pinned Lighthouse kernel."
    } else {
        "run `luxctl env up` to provision the pinned Lighthouse environment, whose kernel \
         is ours rather than the backend's."
    };
    format!(
        "this book needs {label}, and {} on {} does not provide it.\n  {fix}",
        be.kind.as_str(),
        std::env::consts::OS
    )
}

/// name the missing *capability* and every way to supply it. never "install
/// Docker Desktop" — that is one answer to a question the learner did not ask.
fn missing_backend_message() -> String {
    let mac = cfg!(target_os = "macos");
    let mut msg = String::from(
        "no Linux container backend is available, so probes that touch namespaces, cgroups \
         or the kernel have nowhere to run.\n  any one of these supplies it:\n",
    );
    msg.push_str("    lima      — full VM, kernel is ours; needed for eBPF work\n");
    msg.push_str("    colima    — lima-based, good default\n");
    if mac {
        msg.push_str("    orbstack  — fine for container and kubernetes books\n");
    }
    msg.push_str("    docker    — Docker Desktop, or rootful docker on Linux\n");
    if !mac {
        msg.push_str("    podman    — rootful only; rootless cannot run these probes\n");
    }
    msg.push_str("\n  `luxctl env up` will set up lima for you.");
    msg
}
