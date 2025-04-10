use clap::Parser;
use yapu::{Probe, Programmer};

#[derive(Parser)]
struct Cli {
    #[clap(default_value_t = 9_600u32)]
    baudrate: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut builder = Probe::builder();
    builder.baudrate(cli.baudrate);
    let probe = builder.build();

    println!("Please wait for probing...");
    for mut prog in Programmer::discover(&probe)? {
        let bootloader = prog.read_bootloader()?;
        println!(
            "\n\
             Path: {}\n\
             Version: {}\n\
             Opcodes: {}",
            prog.inner().name().unwrap_or("N/A".to_string()),
            bootloader.version_string(),
            bootloader
                .opcodes()
                .iter()
                .map(|opcode| opcode.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    Ok(())
}
