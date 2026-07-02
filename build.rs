use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

const TAILWIND_VERSION: &str = "v4.3.0";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let cache_dir = manifest_dir.join(".tailwindcss");
    let static_dir = manifest_dir.join("src/static");

    let binary = resolve_tailwind(&cache_dir)?;

    for entry in std::fs::read_dir(manifest_dir.join("src/style"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "css") {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    let output = Command::new(&binary)
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

    write_static_asset_manifest(&static_dir)?;

    Ok(())
}

fn write_static_asset_manifest(static_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut assets = Vec::new();
    collect_static_assets(static_dir, static_dir, &mut assets)?;
    assets.sort_by(|left, right| left.source_path.cmp(&right.source_path));

    let mut output = String::new();
    output.push_str("#[derive(Debug)]\n");
    output.push_str("pub struct StaticAsset {\n");
    output.push_str("    pub source_path: &'static str,\n");
    output.push_str("    pub request_path: &'static str,\n");
    output.push_str("}\n\n");
    output.push_str("pub static STATIC_ASSETS: &[StaticAsset] = &[\n");

    for asset in &assets {
        writeln!(
            output,
            "    StaticAsset {{ source_path: {:?}, request_path: {:?} }},",
            asset.source_path, asset.request_path
        )?;
    }

    output.push_str("];\n\n");
    output.push_str("#[must_use]\n");
    output.push_str("pub fn static_url(source_path: &str) -> &'static str {\n");
    output.push_str("    match source_path {\n");

    for asset in &assets {
        writeln!(
            output,
            "        {:?} => {:?},",
            asset.source_path, asset.request_path
        )?;
    }

    output.push_str("        _ => \"/static/missing\",\n");
    output.push_str("    }\n");
    output.push_str("}\n\n");
    output.push_str("#[must_use]\n");
    output
        .push_str("pub fn source_path_for_request(request_path: &str) -> Option<&'static str> {\n");
    output.push_str("    match request_path {\n");

    for asset in &assets {
        writeln!(
            output,
            "        {:?} => Some({:?}),",
            asset.request_path.trim_start_matches("/static/"),
            asset.source_path
        )?;
    }

    output.push_str("        _ => None,\n");
    output.push_str("    }\n");
    output.push_str("}\n");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    std::fs::write(out_dir.join("static_assets.rs"), output)?;

    Ok(())
}

#[derive(Debug)]
struct StaticAssetBuildData {
    source_path: String,
    request_path: String,
}

fn collect_static_assets(
    root: &Path,
    dir: &Path,
    assets: &mut Vec<StaticAssetBuildData>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_static_assets(root, &path, assets)?;
            continue;
        }

        println!("cargo:rerun-if-changed={}", path.display());

        let source_path = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        let request_path = fingerprinted_request_path(&source_path, &path)?;

        assets.push(StaticAssetBuildData {
            source_path,
            request_path,
        });
    }

    Ok(())
}

fn fingerprinted_request_path(
    source_path: &str,
    path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let contents = std::fs::read(path)?;
    let digest = Sha256::digest(contents);
    let fingerprint = format!("{digest:x}");
    let fingerprint = &fingerprint[..16];

    let Some(file_name) = source_path.rsplit('/').next() else {
        return Err(format!("static asset path has no file name: {source_path}").into());
    };
    let directory = source_path
        .strip_suffix(file_name)
        .unwrap_or_default()
        .trim_end_matches('/');

    let fingerprinted_name = match file_name.rsplit_once('.') {
        Some((stem, extension)) => format!("{stem}.{fingerprint}.{extension}"),
        None => format!("{file_name}.{fingerprint}"),
    };

    if directory.is_empty() {
        Ok(format!("/static/{fingerprinted_name}"))
    } else {
        Ok(format!("/static/{directory}/{fingerprinted_name}"))
    }
}

fn resolve_tailwind(cache_dir: &std::path::Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let binary_name = platform_binary_name().ok_or_else(|| {
        format!(
            "unsupported platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;

    let cached = cache_dir.join(binary_name);
    if cached.exists() {
        return Ok(cached);
    }

    if let Ok(path) = which("tailwindcss") {
        return Ok(path);
    }

    let url = format!(
        "https://github.com/tailwindlabs/tailwindcss/releases/download/{TAILWIND_VERSION}/{binary_name}"
    );

    std::fs::create_dir_all(cache_dir)?;
    download(&url, &cached)?;
    set_executable(&cached)?;

    Ok(cached)
}

fn download(url: &str, dest: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("curl")
        .args(["-sSLf", "--retry", "3", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .map_err(|_| Box::<dyn std::error::Error>::from("failed to run curl — is it installed?"))?;

    if !status.success() {
        return Err(format!("tailwindcss download failed (exit: {status:?}) from {url}").into());
    }
    Ok(())
}

fn platform_binary_name() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("tailwindcss-macos-arm64"),
        ("macos", "x86_64") => Some("tailwindcss-macos-x64"),
        ("linux", "x86_64") => Some("tailwindcss-linux-x64"),
        ("linux", "aarch64") => Some("tailwindcss-linux-arm64"),
        ("windows", "x86_64") => Some("tailwindcss-windows-x64.exe"),
        _ => None,
    }
}

#[cfg(unix)]
fn set_executable(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn which(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = Command::new("which").arg(name).output()?;
    if output.status.success() {
        Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()))
    } else {
        Err("not found on PATH".into())
    }
}
