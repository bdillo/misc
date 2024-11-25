use std::path::PathBuf;

use clap::Parser;
use emulator_8086::Disassembler;
use log::error;

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long)]
    file: PathBuf,
    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let log_level = if args.debug {
        log::Level::Debug
    } else {
        log::Level::Info
    };

    simple_logger::init_with_level(log_level).expect("Failed to init logger!");

    let asm_bin = std::fs::read(args.file)?;

    let mut disassembler = Disassembler::new(&asm_bin);
    match disassembler.decode() {
        Ok(disassembled) => println!("{}", disassembled),
        Err(e) => error!("{}", e),
    };

    Ok(())
}
