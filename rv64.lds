OUTPUT_ARCH(riscv64gc)

ENTRY(_start);

SECTIONS {
    .text : {
        *(.text.init);
        *(.text .text.*);
    }

    .data : {
        *(.data .data.* .rodata .rodata.* .sdata .sdata.*);
        
        . = ALIGN(8);
        PROVIDE(__global_pointer$ = .);

        PROVIDE(__bss_start = .);
        *(.sbss .sbss.* .bss .bss.*);
        PROVIDE(__bss_end = .);
        . = ALIGN(8);
        
        PROVIDE(__tmp_stack_bottom = .);
        . += 1024 * 16;
        PROVIDE(__tmp_stack_top = .);
    }
    
    .hash : { *(.hash) }
    .gnu.hash : { *(.gnu.hash) }
    .dynsym : { *(.dynsym) }
    .dynstr : { *(.dynstr) }
    .rela.dyn : {
        *(.rela.init)
        *(.rela.text .rela.text.* .rela.gnu.linkonce.t.*)
        *(.rela.fini)
        *(.rela.rodata .rela.rodata.* .rela.gnu.linkonce.r.*)
        *(.rela.data .rela.data.* .rela.gnu.linkonce.d.*)
        *(.rela.tdata .rela.tdata.* .rela.gnu.linkonce.td.*)
        *(.rela.tbss .rela.tbss.* .rela.gnu.linkonce.tb.*)
        *(.rela.ctors)
        *(.rela.dtors)
        *(.rela.got)
        *(.rela.bss .rela.bss.* .rela.gnu.linkonce.b.*)
        *(.rela.ldata .rela.ldata.* .rela.gnu.linkonce.l.*)
        *(.rela.lbss .rela.lbss.* .rela.gnu.linkonce.lb.*)
        *(.rela.lrodata .rela.lrodata.* .rela.gnu.linkonce.lr.*)
        *(.rela.ifunc)
    }
    .rela.plt : {
        *(.rela.plt)
        *(.rela.iplt)
    }
    .relr.dyn : { *(.relr.dyn) }
    .plt : { *(.plt) *(.iplt) }
    .plt.got : { *(.plt.got) }
    .plt.sec : { *(.plt.sec) }
    .dynamic : { *(.dynamic) }
    .got : { *(.got) *(.igot) }
    .got.plt : { *(.got.plt) *(.igot.plt) }

    /DISCARD/ : { *(.eh_frame_hdr .eh_frame) }
}
