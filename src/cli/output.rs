use tabled::Tabled;
use tabled::derive::display;
use serde::{Serialize, Deserialize};
use std::borrow::Cow;
use std::fmt::{Formatter, Display, Result};
use yapu::{Bootloader, Opcode};

#[derive(Serialize, Debug)]
struct Opcodes<'a>(Cow<'a, [Opcode]>);

impl<'a> Display for Opcodes<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            self.0.iter()
                .map(|opcode: &Opcode| opcode.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        )
    }
}

#[derive(Serialize, Tabled, Debug)]
pub struct Device<'a> {
    #[tabled(display("display::option", "N/A"))]
    name: Option<Cow<'a, str>>,
    version: Cow<'a, str>,
    opcodes: Opcodes<'a>,
}

impl<'a> Device<'a> {
    pub fn from_bootloader(name: Option<&'a str>, bootloader: &'a Bootloader) -> Self {
        Self {
            name: name.map(|s| s.into()),
            version: bootloader.version_string().into(),
            opcodes: Opcodes(bootloader.opcodes().into()),
        }
    }
}

impl<'a> Display for Device<'a> {
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

