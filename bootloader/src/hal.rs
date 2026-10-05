/*
    STM32F4 implementation of the VGBL HAL

    The F4 parts have a single flash bank per operation: while it erases or programs, every flash read stalls,
    instruction fetches and vector fetches included.
*/

mod clock;
mod flash;
mod watchdog;

pub use clock::Stm32f4Clock;
pub use flash::Stm32f4Flash;
pub use watchdog::Stm32f4Watchdog;

use core::ops::Range;

use cortex_m::peripheral::SCB;
use vgbl::hal::{ExecutionContext, Hal};

/// The Cortex-M SRAM region. Everything below it (flash, its boot alias at 0x0, system memory) sits behind the
/// flash interface or may be aliased to it
const SRAM_REGION: Range<usize> = 0x2000_0000..0x4000_0000;

pub struct Stm32f4 {
    clock: Stm32f4Clock,
    flash: Stm32f4Flash,
    watchdog: Stm32f4Watchdog,
}

impl Stm32f4 {
    pub fn new(clock: Stm32f4Clock, flash: Stm32f4Flash, watchdog: Stm32f4Watchdog) -> Self {
        Self { clock, flash, watchdog }
    }
}

impl Hal for Stm32f4 {
    type Clock = Stm32f4Clock;
    type Flash = Stm32f4Flash;
    type Watchdog = Stm32f4Watchdog;

    fn get_execution_context() -> ExecutionContext {
        // Any function in this image tells us where the image is running from
        let code = Self::get_execution_context as fn() -> ExecutionContext as usize;
        // Safety: reading VTOR has no side effects
        let vectors = unsafe { (*SCB::PTR).vtor.read() } as usize;

        if SRAM_REGION.contains(&code) && SRAM_REGION.contains(&vectors) {
            ExecutionContext::Ram
        } else {
            ExecutionContext::Flash
        }
    }

    fn get_clock(&mut self) -> &mut Self::Clock {
        &mut self.clock
    }

    fn get_flash(&mut self) -> &mut Self::Flash {
        &mut self.flash
    }

    fn get_watchdog(&mut self) -> &mut Self::Watchdog {
        &mut self.watchdog
    }
}
