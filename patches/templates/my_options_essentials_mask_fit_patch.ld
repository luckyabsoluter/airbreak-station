/*
 * Linker script for the My Options append-hook payload.
 *
 * Code cave:
 *   start: 0x080FF000
 *   max size: 0x0FFE
 */

PATCH_FUNC_START = 0x080FF000;
PATCH_FUNC_MAX_SIZE = 0x0FFE;

ENTRY(patch_my_options_essentials_append_hook)

SECTIONS
{
  . = PATCH_FUNC_START;

  .patch_target_fn : ALIGN(2)
  {
    KEEP(*(.patch_target_fn))
    KEEP(*(.patch_target_fn.*))
    KEEP(*(.patch_custom_page_seed_fn))
    KEEP(*(.patch_custom_page_seed_fn.*))
    KEEP(*(.patch_custom_page_tail_fn))
    KEEP(*(.patch_custom_page_tail_fn.*))
  }

  .text : ALIGN(2)
  {
    *(.text)
    *(.text.*)
    *(.rodata)
    *(.rodata.*)
    *(.glue_7)
    *(.glue_7t)
  }

  . = ALIGN(4);
  __patch_end = .;

  /DISCARD/ :
  {
    *(.comment)
    *(.note)
    *(.eh_frame)
    *(.ARM.exidx)
    *(.ARM.extab)
  }
}

ASSERT((__patch_end - PATCH_FUNC_START) <= PATCH_FUNC_MAX_SIZE,
       "my_options_essentials_mask_fit_patch payload exceeds code cave size")
