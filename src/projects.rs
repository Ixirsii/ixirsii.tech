pub struct Project {
    pub slug: &'static str,
    pub name: &'static str,
    pub tagline: &'static str,
    pub description: &'static str,
    pub repo_url: &'static str,
    pub website_url: Option<&'static str>,
    pub screenshots: &'static [Screenshot],
    pub tech_tags: &'static [&'static str],
}

pub struct Screenshot {
    pub src: &'static str,
    pub alt: &'static str,
    pub caption: Option<&'static str>,
}

pub fn project_by_slug(slug: &str) -> Option<&'static Project> {
    ALL_PROJECTS.iter().find(|p| p.slug == slug)
}

pub const ALL_PROJECTS: &[Project] = &[
    Project {
        slug: "randstring",
        name: "Randstring",
        tagline: "A random string generator for Odin.",
        description: concat!(
            "A simple library for generating random strings in [Odin](https://odin-lang.org/).\n\n",
            "Supports configurable character sets, length, and multiple generation modes.\n",
            "Designed to be simple, fast, and easy to integrate into any Odin project.",
        ),
        repo_url: "https://codeberg.org/Ixirsii/randstring",
        website_url: None,
        screenshots: &[],
        tech_tags: &["Odin"],
    },
    Project {
        slug: "dotfiles",
        name: "Dotfiles",
        tagline: "Personal configuration files.",
        description: concat!(
            "My personal dotfiles managed with GNU Stow.\n\n",
            "Includes configurations for:\n\n",
            "- **Neovim** — Custom LSP setup, keybindings, and plugin configuration\n",
            "- **WezTerm** — Terminal emulator configuration with Catppuccin theme\n",
            "- **zsh** — Shell aliases, functions, and prompt customization\n",
            "- **Git** — Global gitconfig and aliases\n\n",
            "All configurations are optimized for my daily workflow and designed to be",
            " easily reproducible on a new system.",
        ),
        repo_url: "https://codeberg.org/Ixirsii/dotfiles",
        website_url: None,
        screenshots: &[],
        tech_tags: &["Lua", "Shell", "Zsh", "Neovim"],
    },
    Project {
        slug: "ixirsii-tech",
        name: "ixirsii.tech",
        tagline: "This website.",
        description: concat!(
            "My personal website built with [Rust](https://rust-lang.org/) and [Axum](https://github.com/tokio-rs/axum).\n\n",
            "Features:\n\n",
            "- Server-side rendered HTML with [Askama](https://github.com/askama-rs/askama) templates\n",
            "- Blog with Markdown content stored in MySQL\n",
            "- [HTMX](https://htmx.org/) for dynamic interactions without JavaScript frameworks\n",
            "- [Tailwind CSS](https://tailwindcss.com/) v4 with Catppuccin Mocha theme\n",
            "- Dark/light theme switcher with multiple Catppuccin flavors\n",
        ),
        repo_url: "https://codeberg.org/Ixirsii/ixirsii.tech",
        website_url: Some("https://ixirsii.tech"),
        screenshots: &[],
        tech_tags: &["Rust", "Axum", "HTMX", "Tailwind CSS", "MySQL"],
    },
];
