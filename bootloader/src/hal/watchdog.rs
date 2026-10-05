/*
    STM32F4 independent watchdog (IWDG)

    Runs from the ~32 kHz LSI, so it keeps counting while the CPU is stalled on a flash operation. Once started,
    only a reset stops it: the application must keep feeding it after the bootloader jumps there.
*/

use fugit::MillisDurationU32 as MilliSeconds;
use stm32f4xx_hal::pac::IWDG;
use stm32f4xx_hal::watchdog::IndependentWatchdog;
use vgbl::hal::{Watchdog, WatchdogError};

pub struct Stm32f4Watchdog {
    iwdg: IndependentWatchdog,
    /// Up to ~32.7 s
    timeout: MilliSeconds,
    enabled: bool,
}

impl Stm32f4Watchdog {
    pub fn new(iwdg: IWDG, timeout: MilliSeconds) -> Self {
        Self { iwdg: IndependentWatchdog::new(iwdg), timeout, enabled: false }
    }
}

impl Watchdog for Stm32f4Watchdog {
    fn feed(&mut self) {
        if self.enabled {
            self.iwdg.feed();
        }
    }

    fn enable(&mut self) -> Result<(), WatchdogError> {
        if !self.enabled {
            self.iwdg.start(self.timeout);
            self.enabled = true;
        }
        Ok(())
    }

    fn disable(&mut self) -> Result<(), WatchdogError> {
        if self.enabled { Err(WatchdogError::Unsupported) } else { Ok(()) }
    }
}
