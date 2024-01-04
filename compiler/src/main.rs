
fn main() {
    #[cfg(feature = "cli")]
    {
        use clap::Parser;
        use compiler::cli::{Cli, run};
        run(Cli::parse()).unwrap();
    }

    // Don't bother pulling in dependencies if only want to use the library part.
    #[cfg(not(feature = "cli"))]
    compile_error!("cli disabled")
}
