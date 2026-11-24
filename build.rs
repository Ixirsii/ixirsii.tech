use nine_dev_site_build::{
    catalog::{
        self,
        Registration::{Directory, File, MaxAge, Revalidate},
    },
    tailwind,
};
use std::path::{Path, PathBuf};

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
    let faces = [
        ("Mulish", "normal", "fonts/mulish-normal.woff2"),
        ("Mulish", "italic", "fonts/mulish-italic.woff2"),
        (
            "Source Code Pro",
            "normal",
            "fonts/sourcecodepro-normal.woff2",
        ),
        (
            "Source Code Pro",
            "italic",
            "fonts/sourcecodepro-italic.woff2",
        ),
    ]
    .map(|(family, style, source)| tailwind::FontFace {
        family,
        style,
        source,
        weight: "200 900",
    });
    tailwind::write_font_faces(
        static_dir,
        Path::new("src/style/fonts.generated.css"),
        &faces,
    )?;
    tailwind::write_catppuccin_aliases(Path::new("src/style/catppuccin.aliases.css"))?;
    tailwind::compile(
        Path::new("src/style/main.css"),
        &static_dir.join("main.css"),
        &out.join("main.css"),
    )?;

    let mut registrations = vec![
        Directory("fonts"),
        Directory("images"),
        File("main.css"),
        File("admin-confirm.js"),
        File("post-dates.js"),
        File("apple-touch-icon.png"),
        File("favicon.ico"),
        File("favicon-16x16.png"),
        File("favicon-32x32.png"),
        File("android-chrome-192x192.png"),
        File("android-chrome-512x512.png"),
        Revalidate("android-chrome-192x192.png", "/android-chrome-192x192.png"),
        Revalidate("android-chrome-512x512.png", "/android-chrome-512x512.png"),
        Revalidate("site.webmanifest", "/site.webmanifest"),
        File(".well-known/bimi/logo.svg"),
        MaxAge(
            ".well-known/bimi/logo.svg",
            "/.well-known/bimi/logo.svg",
            3600,
        ),
        MaxAge(
            ".well-known/security.txt",
            "/.well-known/security.txt",
            3600,
        ),
    ];
    // A README snapshot may have no screenshots (Git does not track empty directories).
    if static_dir.join("dotfiles/screenshots").exists() {
        registrations.push(Directory("dotfiles/screenshots"));
    }
    catalog::write(static_dir, &out.join("public_assets.rs"), &registrations)?;

    Ok(())
}
