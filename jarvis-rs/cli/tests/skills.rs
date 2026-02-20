//! Smoke tests for skills commands.

use anyhow::Result;
use predicates::str::contains;
use tempfile::TempDir;

fn jarvis_command(jarvis_home: &std::path::Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(jarvis_utils_cargo_bin::cargo_bin("Jarvis")?);
    cmd.env("jarvis_home", jarvis_home);
    Ok(cmd)
}

#[tokio::test]
async fn skills_create_creates_skill() -> Result<()> {
    let jarvis_home = TempDir::new()?;

    let mut cmd = jarvis_command(jarvis_home.path())?;
    cmd.args([
        "skills",
        "create",
        "csv-processor",
        "--requirements=processar CSV",
        "--language=rust",
        "--skill_type=library",
        "--evaluate=false",
        "--output=json",
    ]).assert().success();

    Ok(())
}

#[tokio::test]
async fn skills_list_shows_skills() -> Result<()> {
    let jarvis_home = TempDir::new()?;

    let mut cmd = jarvis_command(jarvis_home.path())?;
    cmd.args(["skills", "list"]).assert().success();

    Ok(())
}
