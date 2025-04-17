use yapu::{Probe, Programmer};

fn main() -> anyhow::Result<()> {
    let progs = Programmer::discover(&Probe::default())?;
    for mut p in progs {
        let memory = p.read_memory(0x08000000, 128)?;
        memory.as_slice();
        println!("{:#02x?} {}", memory, memory.len());
    }
    Ok(())
}
