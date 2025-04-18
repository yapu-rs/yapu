use std::time::Duration;
#[allow(unused_imports)]
use crate::Command;

pub type Baudrate = u32;

/// MODEM control signals as GPIOs
///
/// The enum variants are part of standard MODEM control signals.
///
/// These signals are not particularly common nowadays, and their usage has
/// changed compared to before. They're generally treated as GPIOs that can be
/// controlled by the DTE (Data Terminal Equipment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Request To Send
    Rts { active_when: bool },
    /// Data Terminal Ready
    Dtr { active_when: bool },
}

impl Signal {
    /// Creates a [`Signal`] for RTS.
    pub fn rts(active_when: bool) -> Self {
        Self::Rts { active_when }
    }

    /// Creates a [`Signal`] for DTR.
    pub fn dtr(active_when: bool) -> Self {
        Self::Dtr { active_when }
    }

    /// Gets the bool value when the signal is active.
    pub fn active_when(&self) -> bool {
        match self {
            Self::Rts { active_when } | Self::Dtr { active_when } => *active_when,
        }
    }

    /// Checks whether the signal is RTS (Request To Send).
    pub fn is_rts(&self) -> bool {
        matches!(self, Self::Rts { .. })
    }

    /// Checks whether the signal is DTR (Data Terminal Ready).
    pub fn is_dtr(&self) -> bool {
        matches!(self, Self::Dtr { .. })
    }

    /// Converts a bool value to the raw one based on [`Self::active_when()`].
    pub fn raw_level(&self, active: bool) -> bool {
        // if the signal is "active high", then just pass through the value
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
    /// Creates a default [`SignalScheme`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a default [`SignalScheme`] builder.
    pub fn builder() -> SignalSchemeBuilder {
        Default::default()
    }

    /// Gets reset signal of the scheme.
    pub fn reset(&self) -> Option<Signal> {
        self.reset
    }

    /// Gets boot signal of the scheme.
    pub fn boot(&self) -> Option<Signal> {
        self.boot
    }

    /// Sets reset signal of the scheme.
    pub fn set_reset(&mut self, signal: Option<Signal>) {
        self.reset = signal;
    }

    /// Sets boot signal of the scheme.
    pub fn set_boot(&mut self, signal: Option<Signal>) {
        self.boot = signal;
    }
}

/// [`SignalScheme`] builder
///
/// A builder can be created by any of
///
/// * [`SignalScheme::builder()`]
/// * [`SignalSchemeBuilder::new()`]
/// * [`SignalSchemeBuilder::default()`].
#[derive(Default, Debug, Clone, Copy)]
pub struct SignalSchemeBuilder {
    inner: SignalScheme,
}

impl SignalSchemeBuilder {
    /// Creates a default [`SignalScheme`] builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets reset signal of the signal scheme.
    pub fn reset(&mut self, signal: Signal) -> &mut Self {
        self.inner.set_reset(Some(signal));
        self
    }

    /// Sets boot signal of the signal scheme.
    pub fn boot(&mut self, signal: Signal) -> &mut Self {
        self.inner.set_boot(Some(signal));
        self
    }

    /// Builds a [`SignalScheme`].
    pub fn build(self) -> SignalScheme {
        self.inner
    }
}

impl From<SignalScheme> for SignalSchemeBuilder {
    /// Creates a [`SignalScheme`] builder from an existing [`SignalScheme`].
    fn from(value: SignalScheme) -> Self {
        Self { inner: value }
    }
}

/// Decides how to identify a device
///
/// Sometimes a device is already in bootloader mode, thus the initial handshake
/// is not necessary. However, we still have to distinguish a device from other
/// ports, probably by using some command: at the moment, only [`Command::Get`]
/// is supported.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Identify {
    /// Identify a device by sending [`Command::Synchronize`].
    #[default]
    Handshake,

    /// Identify a device by sending [`Command::Get`].
    Get,
}

/// Probe contains necessary parameters for probing an AN3155-compliant device.
#[derive(Debug, Clone)]
pub struct Probe {
    baudrate: Baudrate,
    signal_scheme: SignalScheme,
    reset_for: Duration,
    max_attempts: usize,
    timeout: Duration,
    identify: Identify,
}

impl Default for Probe {
    fn default() -> Self {
        Self {
            baudrate: 115_200u32,
            signal_scheme: SignalScheme::default(),
            reset_for: Duration::from_millis(10),
            max_attempts: 8,
            timeout: Duration::from_millis(100),
            identify: Identify::default(),
        }
    }
}

impl Probe {
    /// Creates a default [`Probe`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a default [`Probe`] builder.
    pub fn builder() -> ProbeBuilder {
        Default::default()
    }

    /// Gets baudrate of the probe.
    pub fn baudrate(&self) -> Baudrate {
        self.baudrate
    }

    /// Sets baudrate of the probe.
    pub fn set_baudrate(&mut self, baudrate: Baudrate) {
        self.baudrate = baudrate;
    }

    /// Gets signal scheme of the probe.
    pub fn signal_scheme(&self) -> SignalScheme {
        self.signal_scheme
    }

    /// Sets signal scheme of the probe.
    pub fn set_signal_scheme(&mut self, scheme: SignalScheme) {
        self.signal_scheme = scheme;
    }

    /// Gets reset signal of the signal scheme.
    pub fn signal_reset(&self) -> Option<Signal> {
        self.signal_scheme.reset()
    }

    /// Sets reset signal of the signal scheme.
    pub fn set_signal_reset(&mut self, signal: Option<Signal>) {
        self.signal_scheme.set_reset(signal);
    }

    /// Gets boot signal of the signal scheme.
    pub fn signal_boot(&self) -> Option<Signal> {
        self.signal_scheme.boot()
    }

    /// Sets boot signal of the signal scheme.
    pub fn set_signal_boot(&mut self, signal: Option<Signal>) {
        self.signal_scheme.set_boot(signal);
    }

    /// Gets active period of the reset signal.
    pub fn reset_for(&self) -> Duration {
        self.reset_for
    }

    /// Sets active period of the reset signal.
    pub fn set_reset_for(&mut self, duration: Duration) {
        self.reset_for = duration;
    }

    /// Gets maximum retries for probing a device.
    pub fn max_attempts(&self) -> usize {
        self.max_attempts
    }

    /// Sets maximum retries for probing a device.
    pub fn set_max_attempts(&mut self, max: usize) {
        self.max_attempts = max;
    }

    /// Gets timeout for reading from the underlying port.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Sets timeout for reading from the underlying port.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Gets identification scheme.
    pub fn identify(&self) -> Identify {
        self.identify
    }

    /// Sets identification scheme.
    pub fn set_identify(&mut self, scheme: Identify) {
        self.identify = scheme;
    }
}

/// [`Probe`] builder
///
/// A builder can be created by any of
///
/// * [`Probe::builder()`]
/// * [`ProbeBuilder::new()`]
/// * [`ProbeBuilder::default()`].
#[derive(Default, Debug, Clone)]
pub struct ProbeBuilder {
    inner: Probe,
}

impl ProbeBuilder {
    /// Creates a default [`Probe`] builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a [`Probe`].
    pub fn build(self) -> Probe {
        self.inner
    }

    /// Sets baudrate of the probe.
    pub fn baudrate(&mut self, baudrate: Baudrate) -> &mut Self {
        self.inner.baudrate = baudrate;
        self
    }

    /// Sets reset signal of the probe.
    pub fn signal_reset(&mut self, signal: Signal) -> &mut Self {
        self.inner.signal_scheme.set_reset(Some(signal));
        self
    }

    /// Sets boot signal of the probe.
    pub fn signal_boot(&mut self, signal: Signal) -> &mut Self {
        self.inner.signal_scheme.set_boot(Some(signal));
        self
    }

    /// Sets signal scheme of the probe.
    pub fn signal_scheme(&mut self, scheme: SignalScheme) -> &mut Self {
        self.inner.signal_scheme = scheme;
        self
    }

    /// Disables reset signal of the probe.
    pub fn disable_reset(&mut self) -> &mut Self {
        self.inner.signal_scheme.set_reset(None);
        self
    }

    /// Disables boot signal of the probe.
    pub fn disable_boot(&mut self) -> &mut Self {
        self.inner.signal_scheme.set_boot(None);
        self
    }

    /// Sets the duration of time for keeping the reset signal active.
    pub fn reset_for(&mut self, duration: Duration) -> &mut Self {
        self.inner.reset_for = duration;
        self
    }

    /// Sets identification scheme.
    pub fn identify(&mut self, identify: Identify) -> &mut Self {
        self.inner.identify = identify;
        self
    }
}

impl From<Probe> for ProbeBuilder {
    /// Creates a [`Probe`] builder from an existing [`Probe`].
    fn from(value: Probe) -> Self {
        Self { inner: value }
    }
}
