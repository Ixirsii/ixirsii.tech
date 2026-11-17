use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let static_dir = manifest_dir.join("src/static");

    println!("cargo:rerun-if-changed=src/style");
    println!("cargo:rerun-if-changed=templates");
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

    emit_static_asset_hashes(&static_dir, &static_dir)?;

    Ok(())
}

fn emit_static_asset_hashes(root: &Path, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            emit_static_asset_hashes(root, &path)?;
            continue;
        }

        let source_path = path
            .strip_prefix(root)?
            .to_str()
            .ok_or("static asset paths must be valid UTF-8")?
            .replace('\\', "/");
        let digest = Sha256::digest(std::fs::read(&path)?);
        let mut fingerprint = String::with_capacity(12);
        for byte in &digest[..8] {
            fingerprint.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
            fingerprint.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
        }

        // Keep the path's case and punctuation so different filenames have distinct keys.
        println!(
            "cargo:rustc-env=ASSET_HASH_{source_path}={}",
            &fingerprint[..16]
        );
    }

    Ok(())
}
