/* STM32F446RE: the bootloader owns sector 0 (16K). The application starts at sector 1 (0x08004000) */
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 16K
  RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}
