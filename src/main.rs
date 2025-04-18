mod cli;

use cli::{Cli, Parser};
use anyhow::anyhow;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli.execute()?;
    Ok(())
}
