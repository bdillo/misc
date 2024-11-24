use std::path::PathBuf;

use clap::Parser;
use emulator_8086::Disassembler;

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long)]
    file: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let asm_bin = std::fs::read(args.file)?;

    let mut disassembler = Disassembler::new(&asm_bin);
    let disassembled = disassembler.decode()?;
    println!("{}", disassembled);

    Ok(())
}
