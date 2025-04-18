mod cli;

use cli::{Cli, Parser};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli.execute()?;
    Ok(())
}
