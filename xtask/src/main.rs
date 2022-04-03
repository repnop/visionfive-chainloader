use clap::StructOpt;
use nix::sys::termios;
use std::{
    fs::File,
    io::{Read, Write},
    os::unix::prelude::AsRawFd,
    path::PathBuf,
};

#[derive(StructOpt)]
enum Command {
    FlashChainloader { serial: PathBuf },
    Transfer,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Command::parse();

    match args {
        Command::FlashChainloader { serial } => {
            let file = File::options().read(true).write(true).open(serial)?;
            flash_chainloader(file)?;
        }
        Command::Transfer => todo!(),
    }

    Ok(())
}

fn flash_chainloader(serial: File) -> Result<(), Box<dyn std::error::Error>> {
    let sh = xshell::Shell::new()?;
    xshell::cmd!(
        sh,
        "cargo build -p chainloader --target riscv64imac-unknown-none-elf --release -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem"
    )
    .run()?;
    xshell::cmd!(sh, "llvm-objcopy -O binary target/riscv64imac-unknown-none-elf/release/chainloader build/chainloader.bin").run()?;
    let dir = sh.push_dir("opensbi/");
    xshell::cmd!(
        sh,
        "make PLATFORM=generic LLVM=1 CROSS_COMPILE=riscv64-unknown-linux-gnu- FW_FDT_PATH=../u-boot.dtb FW_PAYLOAD_PATH=../build/chainloader.bin"
    )
    .run()?;

    sh.copy_file(
        "build/platform/generic/firmware/fw_payload.bin",
        "../build/fw_payload.bin",
    )?;
    drop(dir);

    let metadata = std::fs::metadata("build/fw_payload.bin")?;
    sh.write_file(
        "build/fw_payload.bin.out",
        &(metadata.len() as u32).to_le_bytes()[..],
    )?;

    std::io::copy(
        &mut File::open("build/fw_payload.bin")?,
        &mut File::options()
            .append(true)
            .open("build/fw_payload.bin.out")?,
    )?;

    println!("Press enter to flash...");
    let _ = std::io::stdin().read(&mut [0u8])?;
    println!("Flashing...");

    let fd = serial.as_raw_fd();
    let mut termios = termios::tcgetattr(fd.as_raw_fd())?;

    termios.control_flags &= !termios::ControlFlags::PARENB;
    termios.control_flags &= !termios::ControlFlags::CSTOPB;
    termios.control_flags &= !termios::ControlFlags::CSIZE;
    termios.control_flags |= termios::ControlFlags::CS8;
    termios.control_flags &= !termios::ControlFlags::CRTSCTS;
    termios.control_flags |= termios::ControlFlags::CREAD | termios::ControlFlags::CLOCAL;

    termios.local_flags &= !termios::LocalFlags::ICANON;
    termios.local_flags &=
        !(termios::LocalFlags::ECHO | termios::LocalFlags::ECHOE | termios::LocalFlags::ECHONL);
    termios.local_flags &= !termios::LocalFlags::ISIG;

    termios.input_flags &=
        !(termios::InputFlags::IXON | termios::InputFlags::IXOFF | termios::InputFlags::IXANY);
    termios.input_flags &= !(termios::InputFlags::IGNBRK
        | termios::InputFlags::BRKINT
        | termios::InputFlags::PARMRK
        | termios::InputFlags::ISTRIP
        | termios::InputFlags::INLCR
        | termios::InputFlags::IGNCR
        | termios::InputFlags::ICRNL);

    termios.output_flags &= !termios::OutputFlags::OPOST;
    termios.output_flags &= !termios::OutputFlags::ONLCR;

    termios.control_chars[termios::SpecialCharacterIndices::VTIME as usize] = 0;
    termios.control_chars[termios::SpecialCharacterIndices::VMIN as usize] = 0;

    termios::cfsetspeed(&mut termios, termios::BaudRate::B115200)?;
    termios::tcsetattr(fd.as_raw_fd(), termios::SetArg::TCSANOW, &termios)?;

    let (mut read, mut write) = (serial.try_clone()?, serial);
    let (tx1, rx1) = std::sync::mpsc::channel::<()>();
    let (tx2, rx2) = std::sync::mpsc::channel::<()>();
    let thread = std::thread::spawn(move || loop {
        let mut buf = [0u8; 64];

        match read.read(&mut buf[..]) {
            Ok(n) => {
                if n == 1 && buf[0] == b'C' {
                    tx1.send(()).unwrap();
                    rx2.recv().unwrap();
                    continue;
                }

                for byte in &buf[..n] {
                    print!("{}", *byte as char);
                }
            }
            Err(e) => println!("DBG: ERR: {:?}", e),
        }

        let _ = std::io::stdout().lock().flush();
    });

    std::thread::sleep(std::time::Duration::from_millis(1000));

    write.write_all(&[b'a'])?;

    std::thread::sleep(std::time::Duration::from_millis(2000));

    write.write_all(&[b'0', b'\r', b'\n'])?;

    rx1.recv().unwrap();

    let now = std::time::Instant::now();

    let mut sender = xmodem::Sender::new(FileSend(write));

    let contents = std::fs::read("build/fw_payload.bin.out")?;
    sender.send(&contents[..])?;

    println!("\nPayload sent, took: {:?}", now.elapsed());

    tx2.send(()).unwrap();
    let _ = thread.join();

    Ok(())
}

struct FileSend(File);

impl xmodem::SerialDevice for FileSend {
    type Error = std::io::Error;

    fn read(&mut self) -> Result<u8, Self::Error> {
        let mut byte = [0u8];
        while let 0 = self.0.read(&mut byte[..])? {}
        Ok(byte[0])
    }

    fn write(&mut self, c: u8) -> Result<(), Self::Error> {
        self.0.write_all(&[c][..])
    }
}
