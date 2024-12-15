use clap::Parser;

#[derive(Parser)]
pub(super) struct Args {
    #[arg(short, long)]
    pub(super) connection_str: String,
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    pub(super) server_url: String,
}
