fn main() {
    restore_sigpipe();

    if let Err(e) = farol::cmd::Cli::run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

/// Die quietly when the reader goes away, the way every command-line tool does.
///
/// Rust starts with `SIGPIPE` ignored, so a closed pipe surfaces as a write
/// error and then a panic — `farol map show | head` would end in a backtrace
/// instead of just ending.
#[cfg(unix)]
fn restore_sigpipe() {
    // SAFETY: called before anything else runs, and the only thing it does is
    // put the signal back to the disposition the process would have had if it
    // were not written in Rust.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_sigpipe() {}
