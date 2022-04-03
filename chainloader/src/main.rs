#![feature(fn_align, naked_functions)]
#![no_std]
#![no_main]

#[link_section = ".text.init"]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    #[rustfmt::skip]
    core::arch::asm!("
        .option push
        .option norelax
        lla gp, __global_pointer$
        .option pop

        lla t0, __bss_start
        lla t1, __bss_end

        # We must clear the .bss section here since its assumed to be zero on first access
        1:
            beq t0, t1, 2f
            sd zero, (t0)
            addi t0, t0, 8
            j 1b

        2:
            lla sp, __tmp_stack_top
            j main
    ", options(noreturn));
}

#[no_mangle]
extern "C" fn main() -> ! {
    for b in b"Hello world!" {
        sbi::legacy::console_putchar(*b);
    }

    panic!();
}

#[panic_handler]
fn handler(_: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!(".word 0x0100000F") };
    }
}
