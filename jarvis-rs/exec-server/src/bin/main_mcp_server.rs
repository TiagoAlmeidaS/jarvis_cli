#[cfg(not(unix))]
fn main() {
    eprintln!("Jarvis-exec-mcp-server is only implemented for UNIX");
    std::process::exit(1);
}

#[cfg(unix)]
pub use JARVIS_exec_server::main_mcp_server as main;
