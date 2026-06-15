use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(long, default_value = "configs/paper.toml")]
    pub config: String,
}
