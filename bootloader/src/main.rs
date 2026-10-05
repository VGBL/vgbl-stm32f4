#![no_std]
#![no_main]

mod hal;

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f4xx_hal::rcc::Config;
use stm32f4xx_hal::{pac, prelude::*};

#[entry]
fn main() -> ! {
    //  Chip init
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi());

    loop {
        cortex_m::asm::wfi();
    }
}
