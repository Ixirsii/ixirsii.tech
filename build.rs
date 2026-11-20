use std::path::{Path, PathBuf};
use std::process::Command;

use nine_dev_site_build::assets::fingerprint;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let static_dir = manifest_dir.join("src/static");

    println!("cargo:rerun-if-changed=src/style");
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=src/static");

    let output = Command::new("tailwindcss")
        .args([
            "-i",
            "src/style/main.css",
            "-o",
            "src/static/main.css",
            "--minify",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("tailwindcss build failed:\n{stdout}\n{stderr}").into());
    }

    emit_asset_urls(&static_dir, &static_dir)?;

    Ok(())
}

fn emit_asset_urls(root: &Path, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            emit_asset_urls(root, &path)?;
            continue;
        }

        let source_path = path
            .strip_prefix(root)?
            .to_str()
            .ok_or("static asset paths must be valid UTF-8")?
            .replace('\\', "/");
        let asset = fingerprint(&source_path, &std::fs::read(&path)?);
        println!("cargo:rustc-env=ASSET_URL_{source_path}={}", asset.url);
    }

    Ok(())
}
