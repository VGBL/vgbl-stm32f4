/*
    STM32F446RE implementation of the VGBL flash trait

    Single bank, x32 parallelism (needs VDD 2.7-3.6 V). Erases one sector per operation and programs one word per
    poll step. When running from flash the CPU stalls during every operation anyway, so start_* runs the operation
    to completion before returning.
*/

use stm32f4xx_hal::pac::FLASH;
use vgbl::hal::{EraseError, ExecutionContext, Flash, FlashError, Hal, Region, WriteError};

use super::Stm32f4;

const UNLOCK_KEY1: u32 = 0x4567_0123;
const UNLOCK_KEY2: u32 = 0xCDEF_89AB;

/// EOP, OPERR, WRPERR, PGAERR, PGPERR, PGSERR, RDERR. All cleared by writing 1
const SR_FLAGS: u32 = 0x1F3;

/// STM32F446RE: 4x16K, 1x64K, 3x128K. Sector indices match the hardware sector numbers
const REGIONS: [Region; 3] = [
    Region { address: 0x0800_0000, sector_size: 16 * 1024, count: 4 },
    Region { address: 0x0801_0000, sector_size: 64 * 1024, count: 1 },
    Region { address: 0x0802_0000, sector_size: 128 * 1024, count: 3 },
];

/// Flash reads as this once erased
const ERASED_WORD: u32 = 0xFFFF_FFFF;

/// Words buffered by start_write
const WRITE_WORDS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    Idle,
    Erase,
    /// Programming `buffer[next..len]` to the words starting at `address`
    Write { address: usize, next: usize, len: usize },
    /// Ran to completion inside start_*, waiting for poll to report it
    Done(Result<(), FlashError>),
}

pub struct Stm32f4Flash {
    flash: FLASH,
    context: ExecutionContext,
    operation: Operation,
    buffer: [u32; WRITE_WORDS],
}

impl Stm32f4Flash {
    pub fn new(flash: FLASH) -> Self {
        Self { flash, context: <Stm32f4>::get_execution_context(), operation: Operation::Idle, buffer: [0; WRITE_WORDS] }
    }

    fn busy(&self) -> bool {
        self.operation != Operation::Idle || self.flash.sr().read().bsy().bit_is_set()
    }

    /// Lets a just-started operation proceed. From flash, runs it to completion so start_* only returns once
    /// flash is readable again
    fn started(&mut self) {
        if self.context == ExecutionContext::Flash {
            let result = loop {
                match self.step() {
                    Err(nb::Error::WouldBlock) => continue,
                    Err(nb::Error::Other(e)) => break Err(e),
                    Ok(()) => break Ok(()),
                }
            };
            self.operation = Operation::Done(result);
        }
    }

    /// Advances the running operation by at most one word
    fn step(&mut self) -> nb::Result<(), FlashError> {
        match self.operation {
            Operation::Idle => Ok(()),
            Operation::Done(result) => {
                self.operation = Operation::Idle;
                result.map_err(nb::Error::Other)
            }
            Operation::Erase => {
                if self.flash.sr().read().bsy().bit_is_set() {
                    return Err(nb::Error::WouldBlock);
                }
                self.finish(Ok(()))
            }
            Operation::Write { address, next, len } => {
                if self.flash.sr().read().bsy().bit_is_set() {
                    return Err(nb::Error::WouldBlock);
                }
                if self.error().is_some() || next == len {
                    return self.finish(Ok(()));
                }

                self.program(address + next * 4, self.buffer[next]);
                self.operation = Operation::Write { address, next: next + 1, len };
                Err(nb::Error::WouldBlock)
            }
        }
    }

    /// Ends the running operation, reporting any hardware error in preference to `result`
    fn finish(&mut self, result: Result<(), FlashError>) -> nb::Result<(), FlashError> {
        let result = match self.error() {
            Some(e) => Err(e),
            None => result,
        };

        // Safety: every bit in SR_FLAGS is a write-1-to-clear status flag
        self.flash.sr().write(|w| unsafe { w.bits(SR_FLAGS) });
        self.flash.cr().modify(|_, w| w.ser().clear_bit().pg().clear_bit().lock().set_bit());
        self.reset_caches();
        self.operation = Operation::Idle;
        result.map_err(nb::Error::Other)
    }

    fn error(&self) -> Option<FlashError> {
        let sr = self.flash.sr().read();
        if sr.wrperr().bit_is_set() {
            Some(FlashError::WriteProtected)
        } else if sr.pgserr().bit_is_set()
            || sr.pgperr().bit_is_set()
            || sr.pgaerr().bit_is_set()
            || sr.operr().bit_is_set()
            || sr.rderr().bit_is_set()
        {
            Some(FlashError::Operation)
        } else {
            None
        }
    }

    fn unlock(&mut self) {
        if self.flash.cr().read().lock().bit_is_set() {
            // Safety: the documented unlock sequence
            self.flash.keyr().write(|w| unsafe { w.key().bits(UNLOCK_KEY1) });
            self.flash.keyr().write(|w| unsafe { w.key().bits(UNLOCK_KEY2) });
        }
        // Safety: every bit in SR_FLAGS is a write-1-to-clear status flag
        self.flash.sr().write(|w| unsafe { w.bits(SR_FLAGS) });
    }

    fn program(&mut self, address: usize, word: u32) {
        // Safety: start_write checked the address is word-aligned, inside flash, and PG is set
        unsafe { (address as *mut u32).write_volatile(word) };
        // Make sure the write reaches the flash interface before BSY is next read
        cortex_m::asm::dsb();
    }

    /// The ART caches can hold the old contents of erased or programmed flash. They only reset while disabled
    fn reset_caches(&mut self) {
        let acr = self.flash.acr().read();
        let (icen, dcen) = (acr.icen().bit(), acr.dcen().bit());
        self.flash.acr().modify(|_, w| w.icen().clear_bit().dcen().clear_bit());
        self.flash.acr().modify(|_, w| w.icrst().set_bit().dcrst().set_bit());
        self.flash.acr().modify(|_, w| w.icrst().clear_bit().dcrst().clear_bit());
        self.flash.acr().modify(|_, w| w.icen().bit(icen).dcen().bit(dcen));
    }
}

impl Flash for Stm32f4Flash {
    const MAX_WRITE: usize = WRITE_WORDS * 4;
    const WRITE_ALIGN: usize = 4;

    fn get_regions(&self) -> &[Region] {
        &REGIONS
    }

    fn start_erase_sector(&mut self, sector: usize) -> Result<(), EraseError> {
        if self.busy() {
            return Err(EraseError::Busy);
        }
        if self.sector(sector).is_none() {
            return Err(EraseError::OutOfRange);
        }

        self.unlock();
        self.flash.cr().modify(|_, w| {
            w.pg().clear_bit();
            w.ser().set_bit();
            w.psize().psize32();
            // Safety: sector indices match the hardware sector numbers, and the index was checked above
            unsafe { w.snb().bits(sector as u8) }
        });
        self.flash.cr().modify(|_, w| w.strt().set_bit());

        self.operation = Operation::Erase;
        self.started();
        Ok(())
    }

    fn start_write(&mut self, address: usize, data: &[u8]) -> Result<(), WriteError> {
        if self.busy() {
            return Err(WriteError::Busy);
        }
        if data.len() > Self::MAX_WRITE {
            return Err(WriteError::TooLarge);
        }
        if !address.is_multiple_of(Self::WRITE_ALIGN) || !data.len().is_multiple_of(Self::WRITE_ALIGN) {
            return Err(WriteError::Unaligned);
        }
        let end = address.checked_add(data.len()).ok_or(WriteError::OutOfRange)?;
        if address < REGIONS[0].address || end > REGIONS[REGIONS.len() - 1].end() {
            return Err(WriteError::OutOfRange);
        }
        if data.is_empty() {
            return Ok(());
        }
        // The hardware programs over non-erased words without complaint, leaving the AND of old and new
        // Safety: the range is word-aligned, inside flash, and flash is idle so it's readable
        if (address..end).step_by(4).any(|word| unsafe { (word as *const u32).read_volatile() } != ERASED_WORD) {
            return Err(WriteError::NotErased);
        }

        // Copied so the caller's buffer doesn't need to outlive the operation
        let len = data.len() / 4;
        for (word, bytes) in self.buffer.iter_mut().zip(data.chunks_exact(4)) {
            *word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }

        self.unlock();
        self.flash.cr().modify(|_, w| {
            w.ser().clear_bit();
            w.psize().psize32();
            w.pg().set_bit()
        });

        self.operation = Operation::Write { address, next: 0, len };
        self.started();
        Ok(())
    }

    fn poll(&mut self) -> nb::Result<(), FlashError> {
        self.step()
    }
}
