/// Note that the cwd, env, and command args are preserved in the ultimate call
/// to `execv`, so the caller is responsible for ensuring those values are
/// correct.
#[cfg(target_os = "linux")]
fn main() -> ! {
    jarvis_linux_sandbox::run_main()
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Jarvis-linux-sandbox is only supported on Linux");
    std::process::exit(1);
}
