mod shell;
mod output;

use std::str::FromStr;
pub use shell::Shell;
use anyhow::anyhow;
use yapu::{Baudrate, Identify, Probe, Programmer, Signal, SignalScheme};
pub use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[clap(about, author, version, arg_required_else_help = true)]
pub struct Cli {
    #[clap(subcommand)]
    command: Command,

    #[clap(long, default_value = "text")]
    format: Format,
}

#[derive(Args)]
pub struct DeviceOptions {
    /// Specify the device port
    ///
    /// Automatically select the first device if omitted
    #[clap(short, long)]
    device: Option<String>,
}

#[derive(Args)]
pub struct ProbeOptions {
    /// Specify the baudrate for probing and programming
    #[clap(short, long, default_value_t = 115_200)]
    baudrate: Baudrate,

    /// Specify reset MODEM signal
    ///
    /// A signal could be "none", "rts", "dtr", "!rts", "!dtr".
    ///
    /// However, some operating systems automatically assert specific signals on
    /// open, which cannot be changed from userspace.
    #[clap(long, default_value_t = SignalScheme::new().reset().unwrap().into())]
    reset: DeviceSignal,

    /// Specify boot MODEM signal
    #[clap(long, default_value_t = SignalScheme::new().boot().unwrap().into())]
    boot: DeviceSignal,

    /// Identify a device by
    #[clap(short, long, default_value = "handshake")]
    identify: DeviceIdentify,
}

impl ProbeOptions {
    pub fn build_probe(&self) -> Probe {
        let mut scheme = SignalScheme::new();
        scheme.set_reset(self.reset.0);
        scheme.set_boot(self.boot.0);
        let mut builder = Probe::builder();
        builder
            .baudrate(self.baudrate)
            .signal_scheme(scheme)
            .identify(self.identify.into());
        builder.build()
    }
}

#[derive(ValueEnum, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Format {
    /// Normal output
    #[default]
    Text,

    /// Table output
    Table,

    /// JSON output
    Json,
}

impl Format {
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text)
    }

    pub fn is_table(&self) -> bool {
        matches!(self, Self::Table)
    }

    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json)
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Discover compliant devices
    Discover(DiscoverOptions),
    /// Enter interactive shell
    Shell(ShellOptions),
}

#[derive(Args)]
pub struct DiscoverOptions {
    #[clap(flatten)]
    probe: ProbeOptions,
}

#[derive(Args)]
pub struct ShellOptions {
    #[clap(flatten)]
    probe: ProbeOptions,
}

impl Cli {
    fn discover(&self, options: &DiscoverOptions) -> anyhow::Result<()> {
        let probe = options.probe.build_probe();
        if self.format.is_text() {
            eprintln!("Please wait for probing...");
        }
        for mut prog in Programmer::discover(&probe)? {
            let bootloader = prog.read_bootloader()?;
            let name = prog.inner().name();
            let device = output::Device::from_bootloader(
                name.as_ref().map(|s| s.as_ref()),
                &bootloader
            );
            print!("{}", device);
        }
        Ok(())
    }

    fn shell(&self, options: &ShellOptions) -> anyhow::Result<()> {
        let mut shell = Shell::new();
        shell.run()
    }

    pub fn execute(&self) -> anyhow::Result<()> {
        match &self.command {
            Command::Discover(options) => self.discover(options),
            Command::Shell(options) => self.shell(options),
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceSignal(Option<Signal>);

impl DeviceSignal {
    const HIGH: bool = true;
    const LOW: bool = false;
}

impl From<Signal> for DeviceSignal {
    fn from(value: Signal) -> Self {
        Self(Some(value))
    }
}

impl FromStr for DeviceSignal {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Active high by default.
        let mut active_when = Self::HIGH;

        let name = if s.starts_with('!') {
            // The prefix "!" indicates "active low".
            active_when = Self::LOW;

            s.trim_start_matches('!')
        } else {
            s
        };

        match name {
            "rts" => Ok(Self(Some(Signal::rts(active_when)))),
            "dtr" => Ok(Self(Some(Signal::dtr(active_when)))),
            _ if s == "none" => Ok(Self(None)),
            _ => Err(anyhow!("incorrect signal format: {}", s)),
        }
    }
}

impl std::fmt::Display for DeviceSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(signal) => {
                if signal.active_when() == Self::LOW {
                    write!(f, "!")?;
                }
                match signal {
                    Signal::Rts { .. } => write!(f, "{}", "rts"),
                    Signal::Dtr { .. } => write!(f, "{}", "dtr"),
                }
            }
            None => write!(f, "{}", "none"),
        }
    }
}

#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum DeviceIdentify {
    /// Baudrate handshaking (0x7f magic)
    Handshake,

    /// Sending GET command
    Get,
}

impl Into<Identify> for DeviceIdentify {
    fn into(self) -> Identify {
        match self {
            Self::Handshake => Identify::Handshake,
            Self::Get => Identify::Get,
        }
    }
}

