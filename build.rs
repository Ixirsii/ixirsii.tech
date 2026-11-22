use std::{
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use nine_dev_site_build::{assets, tailwind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for input in [
        "src/style",
        "templates",
        "migrations",
        "public",
        "src/page/tag.rs",
    ] {
        println!("cargo:rerun-if-changed={input}");
    }

    let out = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    let static_dir = Path::new("public");
    let mut faces = String::new();
    for (family, stem) in [("Mulish", "mulish"), ("Source Code Pro", "sourcecodepro")] {
        for style in ["normal", "italic"] {
            let path = format!("fonts/{stem}-{style}.woff2");
            let url = assets::fingerprint(&path, &fs::read(static_dir.join(&path))?).url;
            writeln!(
                faces,
                "@font-face {{ font-family: '{family}'; font-style: {style}; font-weight: 200 900; \
                 font-display: swap; src: url('{url}') format('woff2'); }}"
            )?;
        }
    }
    let faces_path = Path::new("src/style/fonts.generated.css");
    if fs::read_to_string(faces_path).ok().as_ref() != Some(&faces) {
        fs::write(faces_path, faces)?;
    }
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
