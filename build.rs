use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bun = Command::new("bun").arg("install").status()?;
    if !bun.success() {
        return Err("bun install failed".into());
    }

    let tailwindcss = Command::new("bunx")
        .args([
            "tailwindcss",
            "-i",
            "src/main.css",
            "-o",
            "src/static/main.css",
            "--minify",
        ])
        .output()?;

    if !tailwindcss.status.success() {
        let stderr = String::from_utf8_lossy(&tailwindcss.stderr);
        let stdout = String::from_utf8_lossy(&tailwindcss.stdout);
        let msg = format!("tailwindcss build failed:\n{stdout}\n{stderr}");

        return Err(msg.as_str().into());
    }

    Ok(())
}
