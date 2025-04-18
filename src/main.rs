mod shell;

use shell::Shell;
use anyhow::anyhow;
use clap::{Args, Parser, Subcommand, ValueEnum};
use log::{debug, error, info, trace, warn};
use std::str::FromStr;
use yapu::{Baudrate, Identify, Probe, Programmer, Signal, SignalScheme};

#[derive(Parser)]
#[clap(about, author, version, arg_required_else_help = true)]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    #[clap(long, default_value = "normal")]
    format: Format,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeviceSignal(Option<Signal>);

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
enum DeviceIdentify {
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

#[derive(Args)]
struct DeviceOptions {
    /// Specify the device port
    ///
    /// Automatically select the first device if omitted
    #[clap(short, long)]
    device: Option<String>,
}

#[derive(Args)]
struct ProbeOptions {
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
    fn build_probe(&self) -> Probe {
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
    Normal,

    /// JSON output
    Json,
}

impl Format {
    fn is_normal(&self) -> bool {
        matches!(self, Self::Normal)
    }
    fn is_json(&self) -> bool {
        matches!(self, Self::Json)
    }
}

#[derive(Subcommand)]
enum Command {
    /// Discover compliant devices
    Discover(DiscoverOptions),
    Shell(ShellOptions),
}

#[derive(Args)]
struct DiscoverOptions {
    #[clap(flatten)]
    probe: ProbeOptions,
}

#[derive(Args)]
struct ShellOptions {
    #[clap(flatten)]
    probe: ProbeOptions,
}

impl Cli {
    fn discover(&self, options: &DiscoverOptions) -> anyhow::Result<()> {
        let probe = options.probe.build_probe();
        if self.format.is_normal() {
            eprintln!("Please wait for probing...");
        }
        for mut prog in Programmer::discover(&probe)? {
            let bootloader = prog.read_bootloader()?;
            println!(
                "\n\
                 Port: {}\n\
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

    fn shell(&self, options: &ShellOptions) -> anyhow::Result<()> {
        let mut shell = Shell::new();
        shell.run()
    }

    fn execute(&self) -> anyhow::Result<()> {
        match &self.command {
            Command::Discover(options) => self.discover(options),
            Command::Shell(options) => self.shell(options),
            _ => todo!(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli.execute()?;
    Ok(())
}
