use serde::Serialize;
use std::fmt::{Display, Formatter, Result};
use tabled::Tabled;
use tabled::derive::display;
use yapu::{Bootloader, Opcode};

#[derive(Serialize, Debug)]
struct Opcodes(Vec<Opcode>);

impl<'a> Display for Opcodes {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|opcode: &Opcode| opcode.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        )
    }
}

#[derive(Serialize, Tabled, Debug)]
pub struct Device {
    #[tabled(display("display::option", "N/A"))]
    name: Option<String>,
    version: String,
    opcodes: Opcodes,
}

impl Device {
    pub fn from_bootloader(name: Option<String>, bootloader: &Bootloader) -> Self {
        Self {
            name: name.map(|s| s.into()),
            version: bootloader.version_string(),
            opcodes: Opcodes(bootloader.opcodes().to_vec()),
        }
    }
}

impl Display for Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(
            f,
            "Name: {}\tVersion: {}",
            self.name.as_ref().map(|s| s.as_ref()).unwrap_or("N/A"),
            self.version,
        )?;
        writeln!(f, "Opcodes: {}", self.opcodes)
    }
}
