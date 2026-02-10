use jarvis_arg0::arg0_dispatch_or_else;
use jarvis_common::CliConfigOverrides;
use jarvis_mcp_server::run_main;

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|jarvis_linux_sandbox_exe| async move {
        run_main(jarvis_linux_sandbox_exe, CliConfigOverrides::default()).await?;
        Ok(())
    })
}
