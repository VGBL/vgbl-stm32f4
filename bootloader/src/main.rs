#![no_std]
#![no_main]

mod hal;
mod hooks;

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f4xx_hal::rcc::Config;
use stm32f4xx_hal::{pac, prelude::*};
use vgbl::Bootloader;
use vgbl::hal::NoWatchdog;

use hal::{Stm32f4, Stm32f4Clock, Stm32f4Flash};
use hooks::Stm32f4Hooks;

#[entry]
fn main() -> ! {
    //  Chip init
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let rcc = dp.RCC.freeze(Config::hsi());

    let clock = Stm32f4Clock::new(cp.DCB, cp.DWT, rcc.clocks.sysclk().raw());
    let flash = Stm32f4Flash::new(dp.FLASH);
    // For the IWDG instead: Stm32f4Watchdog::new(dp.IWDG, timeout), where the timeout outlasts the longest flash
    // operation, since that stalls the CPU when running from flash (128K sector: up to 2 s)
    let watchdog = NoWatchdog;

    //  Bootloader
    let mut bootloader = Bootloader::new(Stm32f4::new(clock, flash, watchdog));
    let mut hooks = Stm32f4Hooks;

    bootloader.init(&mut hooks);
    loop {
        bootloader.poll(&mut hooks);
    }
}
