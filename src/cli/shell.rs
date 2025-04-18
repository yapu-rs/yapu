use anyhow::anyhow;
use std::borrow::Cow;

use yapu::{Baudrate, Probe, Programmer};

use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::time::Duration;

#[allow(unused_imports)]
use clap::{Args, Parser, Subcommand};

use super::{DeviceIdentify, DeviceSignal, ShellOptions};

#[derive(Debug)]
pub struct Shell {
    current: Option<Programmer>,
    options: ShellOptions,
    probe: Probe,
}

#[derive(Parser, Debug, Clone)]
#[clap(
    no_binary_name = true,
    help_template = "{all-args}",
    disable_help_flag = true
)]
enum Command {
    /// Clear messages
    Clear,
    /// Open a device and make it active
    Open {
        /// Device name
        device: String,
    },
    /// Change probe configuration
    Probe {
        /// Change probe baudrate
        #[clap(short, long)]
        baudrate: Option<Baudrate>,

        /// Change probe reset signal
        #[clap(long)]
        reset: Option<DeviceSignal>,

        /// Change probe boot signal
        #[clap(long)]
        boot: Option<DeviceSignal>,

        /// Change probe reset active duration (in milliseconds)
        #[clap(long)]
        reset_for: Option<u64>,

        /// Change probe identification scheme
        #[clap(long)]
        identify: Option<DeviceIdentify>,
    },
    /// List all ports available (without any probe)
    Ports,
    /// Discover devices available
    Discover,
    /// Get bootloader info of current active device
    Get,
    /// Get bootloader version of current active device
    Version,
    /// Get ID of current active device
    Id,
}

impl Command {
    fn requires_device(&self) -> bool {
        match self {
            Self::Get | Self::Version | Self::Id => true,
            _ => false,
        }
    }
}

impl Shell {
    pub fn new(options: ShellOptions) -> Self {
        Self {
            current: None,
            options,
            probe: Probe::default(),
        }
    }

    fn prompt(&self) -> Cow<str> {
        if self.options.no_prompt {
            "".into()
        } else {
            match &self.current {
                Some(p) => p
                    .inner()
                    .name()
                    .map_or("N/A".into(), |name| format!("yapu ({})> ", name).into()),
                None => "yapu> ".into(),
            }
        }
    }

    fn execute(&mut self, command: &Command) -> anyhow::Result<()> {
        match command {
            Command::Clear => {
                clearscreen::clear()?;
            }
            Command::Open { device } => {
                let programmer = Programmer::open(device, &Probe::default())?;
                self.current = Some(programmer);
            }
            Command::Probe {
                baudrate,
                reset,
                boot,
                reset_for,
                identify,
            } => {
                if let Some(baudrate) = baudrate {
                    self.probe.set_baudrate(*baudrate);
                }
                if let Some(reset) = reset {
                    self.probe.set_signal_reset(reset.0);
                }
                if let Some(boot) = boot {
                    self.probe.set_signal_boot(boot.0);
                }
                if let Some(reset_for) = reset_for {
                    self.probe.set_reset_for(Duration::from_millis(*reset_for));
                }
                if let Some(identify) = identify {
                    self.probe.set_identify((*identify).into());
                }
            }
            command if command.requires_device() => {
                let programmer = self
                    .current
                    .as_mut()
                    .ok_or(anyhow!("you need to open a device"))?;
                match command {
                    Command::Get => {
                        println!("{:?}", programmer.read_bootloader()?);
                    }
                    Command::Version => {
                        println!("{:?}", programmer.read_version()?);
                    }
                    Command::Id => {
                        println!("{:?}", programmer.read_id()?);
                    }
                    _ => unreachable!(),
                }
            }
            _ => todo!(),
        }
        Ok(())
    }

    fn dispatch(&mut self, line: &str) -> anyhow::Result<()> {
        #[allow(unused_imports)]
        use clap::error::{ContextKind, Error, ErrorKind};
        let segments = line.trim().split_ascii_whitespace().collect::<Vec<_>>();
        if segments.len() > 0 {
            let command = Command::try_parse_from(segments).map_err(|mut e| {
                e.remove(ContextKind::Usage);
                e
            })?;
            self.execute(&command)?;
        }
        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut editor = DefaultEditor::new()?;
        if !self.options.no_prompt {
            println!(
                "yapu: Yet Another Programmer via USART ({})\n\
                 Type \"help\" for more information.",
                env!("CARGO_PKG_VERSION"),
            );
        }
        loop {
            let line = editor.readline(self.prompt().as_ref());
            match line {
                Ok(line) => {
                    let result = self.dispatch(line.as_ref());
                    match result {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("{}", e);
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    eprintln!("Interrupted");
                    break;
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                    break;
                }
            }
        }
        Ok(())
    }
}
