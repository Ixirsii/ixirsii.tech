use std::path::PathBuf;
use std::process::Command;

const TAILWIND_VERSION: &str = "v4.3.0";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let cache_dir = manifest_dir.join(".tailwindcss");

    let binary = resolve_tailwind(&cache_dir)?;

    println!("cargo:rerun-if-changed=src/style/main.css");

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

    Ok(())
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
