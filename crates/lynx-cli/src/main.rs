mod cli;
mod cmd;
mod render;

use clap::Parser;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("lynxops=info".parse().unwrap()),
        )
        .with_target(false)
        .compact()
        .init();

    let cli = cli::Cli::parse();
    if let Err(err) = cmd::dispatch(cli) {
        eprintln!("lynxops: {err:#}");
        std::process::exit(1);
    }
}
