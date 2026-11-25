use std::{io, process::Command};

// Run in Cargo's scratch directory so the checkout's .env and real database
// cannot affect which startup path the binary takes.
fn command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ixirsii-tech"));
    cmd.current_dir(env!("CARGO_TARGET_TMPDIR")).env_clear();
    cmd
}

#[test]
fn server_rejects_invalid_listener_settings_before_opening_database() -> io::Result<()> {
    for (name, value, message) in [
        ("HOST", "localhost", "HOST must be an IP address"),
        ("PORT", "65536", "PORT must be an integer from 1 to 65535"),
    ] {
        let output = command().env(name, value).output()?;
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(message), "{stderr}");
    }
    Ok(())
}

#[test]
fn admin_create_needs_database_but_ignores_server_configuration() -> io::Result<()> {
    let output = command()
        .args(["admin", "create", "test-admin"])
        .env("HOST", "invalid")
        .env("PORT", "invalid")
        .env("OTEL_EXPORTER_OTLP_ENDPOINT", "invalid")
        .output()?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DATABASE_URL is required"), "{stderr}");
    Ok(())
}
