/* STM32F446RE copyup: copyup copies this bootloader from sector 0 to the start of SRAM and runs it there.
   FLASH is the RAM range the bootloader executes from. Its length is the room left in sector 0 after copyup (1K) */
MEMORY
{
  FLASH : ORIGIN = 0x20000000, LENGTH = 15K
  RAM   : ORIGIN = 0x20003C00, LENGTH = 113K
}
