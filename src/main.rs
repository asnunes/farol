fn main() {
    if let Err(e) = farol::cmd::Cli::run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
