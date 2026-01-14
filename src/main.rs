fn main() {
    if let Err(err) = spriteforge::run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
