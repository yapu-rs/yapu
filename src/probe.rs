use std::time::Duration;

/// MODEM signals as GPIO
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Rts { active_when: bool },
    Dtr { active_when: bool },
}

impl Signal {
    pub fn active_when(&self) -> bool {
        match self {
            Self::Rts { active_when } | Self::Dtr { active_when } => *active_when,
        }
    }

    pub fn is_rts(&self) -> bool {
        matches!(self, Self::Rts { .. })
    }

    pub fn is_dtr(&self) -> bool {
        matches!(self, Self::Dtr { .. })
    }

    pub fn raw_level(&self, active: bool) -> bool {
        // if it's active high, then just pass through the active value
        if self.active_when() == true {
            active
        } else {
            !active
        }
    }
}

/// Signal scheme used for automatic bootloader
///
/// A lot of boards connect MODEM control signals like `RTS` / `DTR` to special
/// pins of the MCU. It's then possible to put the MCU into bootloader by
/// manipulating the signals automatically.
///
/// The signal scheme varies; there might be vendor-specific standards on it,
/// but it tends to be more board-specific.
#[derive(Debug, Clone, Copy)]
pub struct SignalScheme {
    reset: Option<Signal>,
    boot: Option<Signal>,
}

impl Default for SignalScheme {
    fn default() -> Self {
        Self {
            reset: Some(Signal::Rts { active_when: true }),
            boot: Some(Signal::Dtr { active_when: false }),
        }
    }
}

impl SignalScheme {
    pub fn builder() -> SignalSchemeBuilder {
        Default::default()
    }
    pub fn reset(&self) -> Option<Signal> {
        self.reset
    }
    pub fn boot(&self) -> Option<Signal> {
        self.boot
    }
    pub fn set_reset(&mut self, signal: Option<Signal>) {
        self.reset = signal;
    }
    pub fn set_boot(&mut self, signal: Option<Signal>) {
        self.boot = signal;
    }
}

/// `SignalScheme::builder()`
#[derive(Default, Debug, Clone, Copy)]
pub struct SignalSchemeBuilder {
    inner: SignalScheme,
}

impl SignalSchemeBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn reset(&mut self, signal: Signal) -> &mut Self {
        self.inner.set_reset(Some(signal));
        self
    }
    pub fn boot(&mut self, signal: Signal) -> &mut Self {
        self.inner.set_boot(Some(signal));
        self
    }
    pub fn build(self) -> SignalScheme {
        self.inner
    }
}

impl From<SignalScheme> for SignalSchemeBuilder {
    fn from(value: SignalScheme) -> Self {
        Self { inner: value }
    }
}

#[derive(Debug, Clone)]
pub struct Probe {
    baudrate: u32,
    signal_scheme: SignalScheme,
    reset_for: Duration,
    max_attempts: usize,
    timeout: Duration,
}

impl Default for Probe {
    fn default() -> Self {
        Self {
            baudrate: 115_200u32,
            signal_scheme: SignalScheme::default(),
            reset_for: Duration::from_millis(10),
            max_attempts: 8,
            timeout: Duration::from_millis(100),
        }
    }
}

impl Probe {
    pub fn builder() -> ProbeBuilder {
        Default::default()
    }
    pub fn baudrate(&self) -> u32 {
        self.baudrate
    }
    pub fn signal_scheme(&self) -> SignalScheme {
        self.signal_scheme
    }
    pub fn signal_reset(&self) -> Option<Signal> {
        self.signal_scheme.reset()
    }
    pub fn signal_boot(&self) -> Option<Signal> {
        self.signal_scheme.boot()
    }
    pub fn reset_for(&self) -> Duration {
        self.reset_for
    }
    pub fn max_attempts(&self) -> usize {
        self.max_attempts
    }
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[derive(Default, Debug, Clone)]
pub struct ProbeBuilder {
    inner: Probe,
}

impl ProbeBuilder {
    /// Creates a probe builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a probe.
    pub fn build(self) -> Probe {
        self.inner
    }

    /// Set baudrate of the probe.
    pub fn baudrate(&mut self, baudrate: u32) -> &mut Self {
        self.inner.baudrate = baudrate;
        self
    }

    /// Set reset signal of the probe.
    pub fn signal_reset(&mut self, signal: Signal) -> &mut Self {
        self.inner.signal_scheme.set_reset(Some(signal));
        self
    }

    /// Set boot signal of the probe.
    pub fn signal_boot(&mut self, signal: Signal) -> &mut Self {
        self.inner.signal_scheme.set_boot(Some(signal));
        self
    }

    /// Set signal scheme of the probe.
    pub fn signal_scheme(&mut self, scheme: SignalScheme) -> &mut Self {
        self.inner.signal_scheme = scheme;
        self
    }

    /// Set the duration of time for keeping the reset signal active.
    pub fn reset_for(&mut self, duration: Duration) -> &mut Self {
        self.inner.reset_for = duration;
        self
    }
}

impl From<Probe> for ProbeBuilder {
    fn from(value: Probe) -> Self {
        Self { inner: value }
    }
}
