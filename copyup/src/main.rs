/*
    Copyup

    Runs from the start of flash. Reads the image header that follows copyup, copies the bootloader image to the
    RAM address it names, points VTOR at the copied vector table and jumps to it. Running from RAM lets the
    bootloader keep executing while the single flash bank is busy erasing.
*/

#![no_std]
#![no_main]

use core::ops::Range;
use core::panic::PanicInfo;
use core::ptr::addr_of;

use cortex_m::peripheral::SCB;
use vgbl::image::header::ImageHeader;

const SRAM: Range<u32> = 0x2000_0000..0x2002_0000;
const STACK_RESERVE: u32 = 1024;
const VECTOR_TABLE_ALIGN: u32 = 512;

unsafe extern "C" {
    static _image_header: [u8; ImageHeader::SIZE];
}

#[unsafe(link_section = ".vector_table.exceptions")]
#[used]
static EXCEPTIONS: [unsafe extern "C" fn() -> !; 15] = [
    reset, halt, halt, halt, halt, halt, halt, halt, halt, halt, halt, halt, halt, halt, halt,
];

#[unsafe(no_mangle)]
unsafe extern "C" fn reset() -> ! {
    let header_bytes = unsafe { &*addr_of!(_image_header) };
    let header = ImageHeader::from_bytes(header_bytes);
    let Some(vector_table) = validate(&header) else { halt() };

    // Word copy instead of copy_nonoverlapping: compiler_builtins' memcpy alone would fill most of copyup's 1K.
    // Volatile so the loop isn't turned back into a memcpy call
    let source = unsafe { header_bytes.as_ptr().add(ImageHeader::SIZE) }.cast::<u32>();
    let destination = header.load_addr as *mut u32;
    for word in 0..(header.size / 4) as usize {
        unsafe { destination.add(word).write_volatile(source.add(word).read_volatile()) };
    }
    cortex_m::asm::dsb();
    cortex_m::asm::isb();

    // Safety: the vector table was just copied in and validate checked its alignment
    unsafe {
        (*SCB::PTR).vtor.write(vector_table);
        cortex_m::asm::bootload(vector_table as *const u32)
    }
}

fn validate(header: &ImageHeader) -> Option<u32> {
    let end = header.load_addr.checked_add(header.size)?;
    let vector_table = header.load_addr.checked_add(header.vector_table_offset)?;

    let fits = SRAM.contains(&header.load_addr) && end <= SRAM.end - STACK_RESERVE;
    let has_table = header.vector_table_offset.checked_add(8)? <= header.size;
    let aligned = vector_table % VECTOR_TABLE_ALIGN == 0 && header.load_addr % 4 == 0 && header.size % 4 == 0;

    (fits && has_table && aligned).then_some(vector_table)
}

extern "C" fn halt() -> ! {
    loop {
        cortex_m::asm::wfi();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    halt()
}
