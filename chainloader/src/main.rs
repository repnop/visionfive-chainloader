#![feature(fn_align, naked_functions)]
#![no_std]
#![no_main]

mod io;

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
            lla a2, _start
            lla a3, __dynamic
            j relocate
    ", options(noreturn));
}

const R_RISCV_RELATIVE: usize = 3;
const R_RISCV_JUMP_SLOT: usize = 5;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Rela {
    offset: usize,
    info: usize,
    addend: isize,
}

const DT_NULL: isize = 0;
const DT_RELA: isize = 7;
const DT_RELA_COUNT: isize = 0x6FFF_FFF9;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Dyn {
    tag: isize,
    val: usize,
}

#[no_mangle]
unsafe extern "C" fn relocate(
    hart_id: usize,
    fdt: *const u8,
    base: *mut u8,
    mut dynamic: *const Dyn,
) -> ! {
    let mut rela_offset = 0usize;
    let mut rela_count = 0usize;

    while let Dyn { tag, val } = *dynamic {
        match tag {
            DT_NULL => break,
            DT_RELA => rela_offset = val,
            DT_RELA_COUNT => rela_count = val,
            _ => {}
        }

        dynamic = dynamic.add(1);
    }

    let mut relas = base.add(rela_offset).cast::<Rela>();
    for _ in 0..rela_count {
        let rela = *relas;

        match rela.info {
            R_RISCV_RELATIVE => {
                *base.add(rela.offset).cast::<usize>() = base.offset(rela.addend) as usize
            }
            R_RISCV_JUMP_SLOT => {} // ignore??
            ty => panic!("uknown relocation in binary: {}", ty),
        }

        relas = relas.add(1);
    }

    main(hart_id, fdt);
}

fn main(hart_id: usize, fdt: *const u8) -> ! {
    println!("Running at {:#p}, FDT is at: {fdt:#p}", _start as *const u8);
    panic!();
}

#[panic_handler]
fn handler(panic_info: &core::panic::PanicInfo) -> ! {
    println!("{}", panic_info);
    loop {
        unsafe { core::arch::asm!(".word 0x0100000F") };
    }
}
