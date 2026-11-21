use std::path::{Path, PathBuf};

use nine_dev_site_build::{assets, tailwind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for input in [
        "src/style",
        "templates",
        "migrations",
        "src/static",
        "src/page/tag.rs",
    ] {
        println!("cargo:rerun-if-changed={input}");
    }

    let out = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    let static_dir = Path::new("src/static");
    tailwind::write_catppuccin_aliases(Path::new("src/style/catppuccin.aliases.css"))?;
    tailwind::compile(
        Path::new("src/style/main.css"),
        &static_dir.join("main.css"),
        &out.join("main.css"),
    )?;

    for asset in assets::walk(static_dir)? {
        let source = asset.source;
        if source.contains('=') {
            return Err(format!("asset path cannot contain '=': {source}").into());
        }
        let assets::Fingerprint { hash, url } = asset.fingerprint;
        println!("cargo:rustc-env=ASSET_URL_{source}={url}");
        println!("cargo:rustc-env=ASSET_HASH_{source}={hash}");
    }

    Ok(())
}
