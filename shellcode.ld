ENTRY(_start);

SECTIONS
{
	. = ALIGN(16);
	.text :
	{
		*(.text._start);
		*(.text.prologue)
		*(.text)
		*(.text.*)
		*(.rodata)
		*(.rodata.*)
		*(.rodata.*.*)
		*(.data)
		*(.data.*)
		*(.data.*.*)
		*(.got)
		*(.bss)
		*(.bss.*)
		*(.got.*)
		__environ = ABSOLUTE(.);
	}


	/DISCARD/ :
	{
		*(.interp)
		*(.comment)
		*(.debug_frame)
	}
}
