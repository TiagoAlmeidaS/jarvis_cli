//! Smoke tests for agent commands.

use anyhow::Result;
use predicates::str::contains;
use tempfile::TempDir;

fn jarvis_command(jarvis_home: &std::path::Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(jarvis_utils_cargo_bin::cargo_bin("Jarvis")?);
    cmd.env("jarvis_home", jarvis_home);
    Ok(cmd)
}

#[tokio::test]
async fn agent_explore_explores_codebase() -> Result<()> {
    let jarvis_home = TempDir::new()?;

    let mut cmd = jarvis_command(jarvis_home.path())?;
    cmd.args([
        "agent",
        "explore",
        ".",
        "--thoroughness=fast",
        "--output=json",
    ])
    .assert()
    .success();

    Ok(())
}

#[tokio::test]
async fn agent_plan_creates_plan() -> Result<()> {
    let jarvis_home = TempDir::new()?;

    let mut cmd = jarvis_command(jarvis_home.path())?;
    cmd.args(["agent", "plan", "criar função de soma", "--output=json"])
        .assert()
        .success();

    Ok(())
}
