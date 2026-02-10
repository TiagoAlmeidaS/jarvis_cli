#[cfg(not(unix))]
fn main() {
    eprintln!("Jarvis-execve-wrapper is only implemented for UNIX");
    std::process::exit(1);
}

#[cfg(unix)]
pub use JARVIS_exec_server::main_execve_wrapper as main;
