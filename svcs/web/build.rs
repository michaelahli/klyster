//! Build script: run `PostCSS` to build Tailwind CSS.
//!
//! Strategy:
//! 1. If `KLYSTER_SKIP_UI_BUILD=1` is set, skip everything.
//! 2. If `static/dist/styles.css` already exists, skip.
//! 3. Run `npm run build` to compile Tailwind.
//! 4. Falls back to placeholder if npm is missing.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let static_dir = manifest_dir.join("static");
    let dist_dir = static_dir.join("dist");
    let styles_css = dist_dir.join("styles.css");

    println!("cargo:rerun-if-env-changed=KLYSTER_SKIP_UI_BUILD");
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("package.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        static_dir.join("css").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("tailwind.config.js").display()
    );

    if std::env::var_os("KLYSTER_SKIP_UI_BUILD").is_some() {
        ensure_placeholder(&dist_dir, &styles_css, "KLYSTER_SKIP_UI_BUILD set");
        return;
    }

    if styles_css.exists() {
        return;
    }

    let npm_ok = Command::new("npm")
        .arg("--version")
        .current_dir(&manifest_dir)
        .output()
        .is_ok_and(|o| o.status.success());
    if !npm_ok {
        ensure_placeholder(&dist_dir, &styles_css, "npm not found");
        return;
    }

    if !manifest_dir.join("node_modules").exists() {
        println!("cargo:warning=installing deps (npm install)");
        let status = Command::new("npm")
            .arg("install")
            .current_dir(&manifest_dir)
            .status();
        if !status.is_ok_and(|s| s.success()) {
            ensure_placeholder(&dist_dir, &styles_css, "npm install failed");
            return;
        }
    }

    println!("cargo:warning=building CSS (npm run build)");
    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(&manifest_dir)
        .status();
    if !status.is_ok_and(|s| s.success()) {
        ensure_placeholder(&dist_dir, &styles_css, "npm run build failed");
    }
}

fn ensure_placeholder(dist_dir: &Path, styles_css: &Path, reason: &str) {
    if styles_css.exists() {
        return;
    }
    println!("cargo:warning=using placeholder CSS: {reason}");
    if let Err(err) = std::fs::create_dir_all(dist_dir) {
        panic!("failed to create {}: {err}", dist_dir.display());
    }
    let body = "body { font-family: system-ui; padding: 2rem; }\n";
    if let Err(err) = std::fs::write(styles_css, body) {
        panic!("failed to write {}: {err}", styles_css.display());
    }
}
