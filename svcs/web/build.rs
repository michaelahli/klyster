//! Build script: ensure `ui/dist/` exists before the crate compiles.
//!
//! Strategy:
//! 1. If `KLYSTER_SKIP_UI_BUILD=1` is set, skip everything (CI/Docker may
//!    pre-build the UI in a separate stage).
//! 2. If `ui/dist/index.html` already exists, skip (developer is iterating).
//! 3. Otherwise, try `npm run build` in `ui/`. If `npm` is missing, fall back
//!    to writing a minimal placeholder `dist/index.html` so the binary still
//!    links and runs (with a friendly "build the UI" message).
//!
//! Re-runs whenever `ui/src/` or `ui/package.json` changes — never on every
//! Rust source edit.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("svcs/web crate has a workspace root two levels up")
        .to_path_buf();
    let ui_dir = workspace_root.join("ui");
    let dist_dir = ui_dir.join("dist");
    let index_html = dist_dir.join("index.html");

    println!("cargo:rerun-if-env-changed=KLYSTER_SKIP_UI_BUILD");
    println!(
        "cargo:rerun-if-changed={}",
        ui_dir.join("package.json").display()
    );
    println!("cargo:rerun-if-changed={}", ui_dir.join("src").display());
    println!(
        "cargo:rerun-if-changed={}",
        ui_dir.join("index.html").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ui_dir.join("vite.config.ts").display()
    );

    if std::env::var_os("KLYSTER_SKIP_UI_BUILD").is_some() {
        ensure_placeholder(&dist_dir, &index_html, "KLYSTER_SKIP_UI_BUILD set");
        return;
    }

    if index_html.exists() {
        // Already built; let rust-embed pick it up.
        return;
    }

    if !ui_dir.exists() {
        ensure_placeholder(&dist_dir, &index_html, "ui/ directory missing");
        return;
    }

    let npm_ok = Command::new("npm")
        .arg("--version")
        .current_dir(&ui_dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !npm_ok {
        ensure_placeholder(
            &dist_dir,
            &index_html,
            "npm not found on PATH; install Node.js to build the UI",
        );
        return;
    }

    // Lazy install: only if node_modules is missing.
    if !ui_dir.join("node_modules").exists() {
        println!("cargo:warning=installing UI dependencies (npm ci)");
        let status = Command::new("npm").arg("ci").current_dir(&ui_dir).status();
        if !status.map(|s| s.success()).unwrap_or(false) {
            ensure_placeholder(&dist_dir, &index_html, "npm ci failed; UI not built");
            return;
        }
    }

    println!("cargo:warning=building UI bundle (npm run build)");
    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(&ui_dir)
        .status();
    if !status.map(|s| s.success()).unwrap_or(false) {
        ensure_placeholder(&dist_dir, &index_html, "npm run build failed");
    }
}

/// Create a minimal `dist/index.html` so `rust-embed` always has something
/// to embed. The placeholder explains how to build the real UI.
fn ensure_placeholder(dist_dir: &Path, index_html: &Path, reason: &str) {
    if index_html.exists() {
        return;
    }
    println!(
        "cargo:warning=using placeholder UI bundle: {reason}. Run `cd ui && npm install && npm run build` to produce a real bundle.",
    );
    if let Err(err) = std::fs::create_dir_all(dist_dir) {
        panic!("failed to create {}: {err}", dist_dir.display());
    }
    let body = format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>Klyster</title></head>\n<body><main style=\"font-family:system-ui;padding:2rem;\"><h1>Klyster</h1><p>UI bundle not built ({reason}). Run <code>cd ui &amp;&amp; npm install &amp;&amp; npm run build</code> and rebuild the binary.</p></main></body></html>\n",
    );
    if let Err(err) = std::fs::write(index_html, body) {
        panic!("failed to write {}: {err}", index_html.display());
    }
}
