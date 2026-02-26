//! Smoke tests for intent detection commands.

use anyhow::Result;
use predicates::str::contains;
use tempfile::TempDir;

fn jarvis_command(jarvis_home: &std::path::Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(jarvis_utils_cargo_bin::cargo_bin("Jarvis")?);
    cmd.env("jarvis_home", jarvis_home);
    Ok(cmd)
}

#[tokio::test]
async fn intent_detect_detects_intent() -> Result<()> {
    let jarvis_home = TempDir::new()?;

    let mut cmd = jarvis_command(jarvis_home.path())?;
    cmd.args([
        "intent",
        "detect",
        "criar uma skill para processar CSV",
        "--output",
        "json",
    ])
    .assert()
    .success();

    Ok(())
}

#[tokio::test]
async fn intent_list_shows_supported_intents() -> Result<()> {
    let jarvis_home = TempDir::new()?;

    let mut cmd = jarvis_command(jarvis_home.path())?;
    cmd.args(["intent", "list"])
        .assert()
        .success()
        .stdout(contains("CreateSkill"));

    Ok(())
}

#[tokio::test]
async fn intent_test_runs_test_suite() -> Result<()> {
    let jarvis_home = TempDir::new()?;

    let mut cmd = jarvis_command(jarvis_home.path())?;
    cmd.args(["intent", "test"])
        .assert()
        .success()
        .stdout(contains("All tests completed"));

    Ok(())
}
