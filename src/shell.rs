use anyhow::anyhow;
use std::borrow::Cow;
use serialport::SerialPort;
use yapu::{Probe, Programmer};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

#[derive(Default, Debug)]
pub struct Shell {
    current: Option<Programmer>
}

struct Command {
}

impl Shell {
    pub fn new() -> Self { Self::default() }

    fn prompt(&self) -> Cow<str> {
        match &self.current {
            Some(p) => p.inner()
                .name()
                .map_or("N/A".into(), |name| format!("yapu ({})> ", name).into()),
            None => "yapu> ".into(),
        }
    }

    fn open(&mut self, name: &str) -> anyhow::Result<()> {
        let programmer = Programmer::open(name, &Probe::default())?;
        self.current = Some(programmer);
        Ok(())
    }

    fn get(&mut self) -> anyhow::Result<()> {
        let programmer = self.current.as_mut().ok_or(anyhow!("you need to open a device"))?;
        let bootloader = programmer.read_bootloader()?;
        println!("{:?}", bootloader);
        Ok(())
    }

    fn version(&mut self) -> anyhow::Result<()> {
        let programmer = self.current.as_mut().ok_or(anyhow!("you need to open a device"))?;
        println!("{:?}", programmer.read_version()?);
        Ok(())
    }

    fn id(&mut self) -> anyhow::Result<()> {
        let programmer = self.current.as_mut().ok_or(anyhow!("you need to open a device"))?;
        println!("{:?}", programmer.read_id()?);
        Ok(())
    }

    fn dispatch(&mut self, line: &str) -> anyhow::Result<()> {
        let segments = line.trim().split_ascii_whitespace().collect::<Vec<_>>();
        if let Some((&command, args)) = segments.split_first() {
            match command {
                "open" => {
                    let name = args.get(0).ok_or(anyhow!("invalid argument"))?;
                    self.open(name)?;
                }
                "get" => { self.get()?; }
                "version" => { self.version()?; }
                "id" => { self.id()?; }
                "help" => println!("help"),
                _ => println!("unknown command: {}", command),
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut editor = DefaultEditor::new()?;
        println!(
            "yapu: Yet Another Programmer via USART ({})\n\
             Type \"help\" for more information.",
            env!("CARGO_PKG_VERSION"),
        );
        loop {
            let line = editor.readline(self.prompt().as_ref());
            match line {
                Ok(line) => {
                    let result = self.dispatch(line.as_ref());
                    match result {
                        Ok(_) => {}
                        Err(e) => { eprintln!("error: {}", e); }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Interrupted");
                }
                Err(e) => {
                    println!("{:?}", e);
                    break
                }
            }
        }
        Ok(())

    }
}

