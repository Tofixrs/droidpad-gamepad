#[cfg(feature = "vjoy")]
mod vjoy;

#[cfg(feature = "vigem")]
mod vigembus;

use crate::input::Key;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, clap::Args, Serialize, Deserialize)]
pub struct Options {
    #[arg(long, value_enum, default_value_t = Backend::default())]
    pub backend: Backend,

    #[cfg(feature = "vjoy")]
    #[arg(long, default_value_t = 0)]
    /// Sets the vjoy device to use when `--backend vjoy` is selected
    pub vjoy_device: u8,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            backend: Backend::default(),
            #[cfg(feature = "vjoy")]
            vjoy_device: 0,
        }
    }
}

impl Options {
    pub fn initialize(&self) -> anyhow::Result<()> {
        #[cfg(feature = "vjoy")]
        if matches!(self.backend, Backend::Vjoy) {
            vjoy::set_device_id(self.vjoy_device)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize, Deserialize)]
pub enum Backend {
    #[cfg(feature = "vigem")]
    Vigem,
    #[cfg(feature = "vjoy")]
    Vjoy,
}

#[cfg(feature = "vigem")]
impl Default for Backend {
    fn default() -> Self {
        return Backend::Vigem;
    }
}

#[cfg(all(not(feature = "vigem"), feature = "vjoy"))]
impl Default for Backend {
    fn default() -> Self {
        return Backend::Vjoy;
    }
}

pub enum Controller {
    #[cfg(feature = "vigem")]
    Vigem(vigembus::Controller),
    #[cfg(feature = "vjoy")]
    Vjoy(vjoy::Controller),
}

impl Controller {
    pub fn new(device_name: &str, options: &Options) -> anyhow::Result<Self> {
        match options.backend {
            #[cfg(feature = "vigem")]
            Backend::Vigem => Ok(Self::Vigem(vigembus::Controller::new(device_name)?)),
            #[cfg(feature = "vjoy")]
            Backend::Vjoy => Ok(Self::Vjoy(vjoy::Controller::new(device_name)?)),
        }
    }

    pub fn write_input(&mut self, key: Key) -> anyhow::Result<()> {
        match self {
            #[cfg(feature = "vigem")]
            Self::Vigem(controller) => controller.write_input(key),
            #[cfg(feature = "vjoy")]
            Self::Vjoy(controller) => controller.write_input(key),
        }
    }

    pub fn synchronize(&mut self) -> anyhow::Result<()> {
        match self {
            #[cfg(feature = "vigem")]
            Self::Vigem(controller) => controller.synchronize(),
            #[cfg(feature = "vjoy")]
            Self::Vjoy(controller) => controller.synchronize(),
        }
    }
}
