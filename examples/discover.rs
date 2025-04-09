use yapu::{Probe, Programmer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let probe = Probe::default();
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
            bootloader.opcodes()
                .iter()
                .map(|opcode| opcode.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    Ok(())
}
