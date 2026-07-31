//! `luxctl env` — manage the pinned Lighthouse Linux environment.

use crate::env::{backend, LIGHTHOUSE_IMAGE, LIMA_VM};
use crate::ui::UI;
use color_eyre::eyre::Result;
use std::process::Stdio;
use tokio::process::Command;

const LIMA_TEMPLATE: &str = include_str!("../env/lima.yaml");

/// what luxctl found, and whether it is good enough for kernel-pinned work.
pub async fn status() -> Result<()> {
    UI::header();
    UI::section("Environment");
    UI::ok("platform", Some(std::env::consts::OS));

    let Some(mut be) = backend::detect().await else {
        UI::error("backend", Some("none found"));
        UI::blank();
        UI::note("run `luxctl env up` to provision the pinned Lima environment.");
        return Ok(());
    };

    backend::probe_guest(&mut be).await;

    if be.kind.is_usable() {
        UI::ok("backend", Some(be.kind.as_str()));
    } else {
        UI::error(
            "backend",
            Some(&format!(
                "{} — nested userns limits break namespace and cgroup probes",
                be.kind.as_str()
            )),
        );
    }

    match be.facts.kernel {
        Some((a, b, c)) => UI::ok("kernel", Some(&format!("{a}.{b}.{c}"))),
        None => UI::warn("kernel", Some("unknown")),
    }
    report("cgroup v2", be.facts.cgroup_v2);
    report("user namespaces", be.facts.userns);

    if be.facts.btf {
        UI::ok("BTF", None);
    } else if !be.kind.btf_is_trustworthy() {
        UI::skip(
            "BTF",
            Some("gated — this backend's kernel tracks its own release"),
        );
    } else {
        UI::warn("BTF", Some("not available"));
    }

    UI::blank();
    if be.kind.kernel_is_ours() {
        UI::note("this is the pinned environment; kernel-sensitive books are fully supported.");
    } else {
        UI::note("for eBPF work, run `luxctl env up` to use the pinned Lima kernel.");
    }
    Ok(())
}

fn report(label: &str, ok: bool) {
    if ok {
        UI::ok(label, None);
    } else {
        UI::warn(label, Some("not available"));
    }
}

/// create and start the pinned Lima VM.
pub async fn up() -> Result<()> {
    UI::header();

    if which("limactl").await.is_none() {
        UI::error("limactl", Some("not found"));
        UI::blank();
        UI::note("lima supplies the kernel we pin. install it with:");
        UI::info("  brew install lima        # macOS");
        UI::info("  https://lima-vm.io       # Linux");
        return Ok(());
    }

    let config = write_template()?;
    UI::step(&format!(
        "starting the '{LIMA_VM}' environment (first run pulls an image)"
    ));

    let status = Command::new("limactl")
        .args(["start", "--tty=false", "--name", LIMA_VM])
        .arg(&config)
        .status()
        .await?;

    if !status.success() {
        UI::error("env up", Some("lima could not start the environment"));
        return Ok(());
    }

    UI::success(&format!("'{LIMA_VM}' is running"));
    UI::note("run `luxctl env status` to confirm what it provides.");
    Ok(())
}

/// stop the VM, leaving its disk in place.
pub async fn down() -> Result<()> {
    UI::header();
    let status = Command::new("limactl")
        .args(["stop", "-f", LIMA_VM])
        .status()
        .await;

    match status {
        Ok(s) if s.success() => UI::success(&format!("'{LIMA_VM}' stopped")),
        _ => UI::warn("env down", Some(&format!("'{LIMA_VM}' was not running"))),
    }
    Ok(())
}

/// pull the current pinned image into whichever backend is active.
///
/// this is the answer to "your colima guest is on 5.15" — it refreshes the
/// image the environment runs, not luxctl itself.
pub async fn upgrade() -> Result<()> {
    UI::header();

    let Some(be) = backend::detect().await else {
        UI::error("backend", Some("none found"));
        UI::note("run `luxctl env up` first.");
        return Ok(());
    };

    UI::step(&format!(
        "pulling {LIGHTHOUSE_IMAGE} into {}",
        be.kind.as_str()
    ));
    let status = be
        .command()
        .args(["pull", LIGHTHOUSE_IMAGE])
        .status()
        .await?;

    if status.success() {
        UI::success("environment image is current");
    } else {
        UI::error("env upgrade", Some("could not pull the pinned image"));
    }
    Ok(())
}

/// lima needs the template on disk; keep it beside the rest of luxctl's state.
fn write_template() -> Result<std::path::PathBuf> {
    let dir = dirs::home_dir()
        .map(|h| h.join(".luxctl"))
        .unwrap_or_else(|| std::path::PathBuf::from(".luxctl"));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("lighthouse.yaml");
    std::fs::write(&path, LIMA_TEMPLATE)?;
    Ok(path)
}

async fn which(bin: &str) -> Option<()> {
    Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .ok()
        .filter(|s| s.success())
        .map(|_| ())
}
