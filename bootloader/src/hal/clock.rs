use commkit::Instant;
use cortex_m::peripheral::{DCB, DWT};
use vgbl::hal::Clock;

/// Monotonic microsecond clock built on the DWT cycle counter
///
/// The 32-bit counter wraps every ~268 s at 16 MHz, so `now` must be called at least that often.
pub struct Stm32f4Clock {
    cycles_per_us: u64,
    last: u32,
    cycles: u64,
}

impl Stm32f4Clock {
    pub fn new(mut dcb: DCB, mut dwt: DWT, sysclk_hz: u32) -> Self {
        dcb.enable_trace();
        dwt.enable_cycle_counter();
        Self { cycles_per_us: (sysclk_hz / 1_000_000) as u64, last: DWT::cycle_count(), cycles: 0 }
    }
}

impl Clock for Stm32f4Clock {
    fn now(&mut self) -> Instant {
        let count = DWT::cycle_count();
        self.cycles += count.wrapping_sub(self.last) as u64;
        self.last = count;
        Instant::from_ticks(self.cycles / self.cycles_per_us)
    }
}
