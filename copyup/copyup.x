/* STM32F446RE: copyup owns the first 1K of sector 0. The segment header and data follow immediately after */
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 1K
  RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}

ENTRY(reset);

_stack_start = ORIGIN(RAM) + LENGTH(RAM);
_segment_header = ORIGIN(FLASH) + LENGTH(FLASH);

SECTIONS
{
  .vector_table ORIGIN(FLASH) :
  {
    LONG(_stack_start);
    KEEP(*(.vector_table.exceptions));
  } > FLASH

  .text :
  {
    *(.text .text.*);
  } > FLASH

  .rodata :
  {
    *(.rodata .rodata.*);
  } > FLASH

  /* Nothing initializes RAM before copyup runs, so it must not have any statics there */
  .data : { *(.data .data.*); } > RAM
  .bss (NOLOAD) : { *(.bss .bss.* COMMON); } > RAM

  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.* .ARM.extab.*);
  }
}

ASSERT(SIZEOF(.data) == 0 && SIZEOF(.bss) == 0, "copyup must not use .data or .bss");
