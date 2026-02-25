//! Smoke tests for safety commands.

use anyhow::Result;
use predicates::str::contains;
use tempfile::TempDir;

fn jarvis_command(jarvis_home: &std::path::Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(jarvis_utils_cargo_bin::cargo_bin("Jarvis")?);
    cmd.env("jarvis_home", jarvis_home);
    Ok(cmd)
}

#[tokio::test]
async fn safety_check_checks_safety() -> Result<()> {
    let jarvis_home = TempDir::new()?;

    let mut cmd = jarvis_command(jarvis_home.path())?;
    cmd.args([
        "safety",
        "check",
        "file_write",
        "--context=test",
        "--files=test.rs",
        "--output=json",
    ]).assert().success();

    Ok(())
}

#[tokio::test]
async fn safety_rules_shows_rules() -> Result<()> {
    let jarvis_home = TempDir::new()?;

    let mut cmd = jarvis_command(jarvis_home.path())?;
    cmd.args(["safety", "rules"]).assert().success();

    Ok(())
}
