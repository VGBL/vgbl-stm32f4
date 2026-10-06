/*
    STM32F4 hooks

    Chip-specific steps come first in each hook. Add your own after them.
*/

use vgbl::hal::Watchdog;
use vgbl::{Bootloader, Hooks};

use crate::hal::Stm32f4;

pub struct Stm32f4Hooks;

impl<W: Watchdog> Hooks<Stm32f4<W>> for Stm32f4Hooks {
    fn init(&mut self, _bootloader: &mut Bootloader<Stm32f4<W>>) {
        // Chip

        // User
    }

    fn poll(&mut self, _bootloader: &mut Bootloader<Stm32f4<W>>) {
        // Chip

        // User
    }

    fn exit(&mut self, _bootloader: &mut Bootloader<Stm32f4<W>>) {
        // Chip

        // User
    }
}
