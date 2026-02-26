//! Smoke tests for context commands.

use anyhow::Result;
use predicates::str::contains;
use tempfile::TempDir;

fn jarvis_command(jarvis_home: &std::path::Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(jarvis_utils_cargo_bin::cargo_bin("Jarvis")?);
    cmd.env("jarvis_home", jarvis_home);
    Ok(cmd)
}

#[tokio::test]
async fn context_add_adds_document() -> Result<()> {
    let jarvis_home = TempDir::new()?;
    let temp_file = TempDir::new()?;
    std::fs::write(temp_file.path().join("test.md"), "# Test Content")?;

    let mut cmd = jarvis_command(jarvis_home.path())?;
    cmd.args([
        "context",
        "add",
        temp_file.path().join("test.md").to_str().unwrap(),
        "--type=docs",
        "--output=json",
    ])
    .assert()
    .success();

    Ok(())
}

#[tokio::test]
async fn context_list_lists_documents() -> Result<()> {
    let jarvis_home = TempDir::new()?;

    let mut cmd = jarvis_command(jarvis_home.path())?;
    cmd.args(["context", "list"]).assert().success();

    Ok(())
}
