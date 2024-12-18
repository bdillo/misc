pub mod disassembler;
pub mod macros;
pub mod modrm;
pub mod opcodes;
pub mod operation;
pub mod reg;

use std::fmt;

// type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type DestinationIsReg = bool;
type IsWord = bool;
// type IsSigned = bool;

#[derive(Debug)]
pub enum DissassemblerError {
    InvalidOpcode(u8),
    InvalidMode,
    InvalidRegister,
    InvalidEffectiveAddress(u8),
}

impl fmt::Display for DissassemblerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: better errors
        let mut error_str = match self {
            Self::InvalidOpcode(op) => format!("Invalid Opcode 0b{:08b}", op),
            Self::InvalidMode => "Invalid mode".to_owned(),
            Self::InvalidRegister => "Invalid Register".to_owned(),
            Self::InvalidEffectiveAddress(addr) => {
                format!("Invalid effective address 0b{:08b}", addr)
            }
        };
        error_str.push('\n');

        write!(f, "{}", error_str)
    }
}

impl std::error::Error for DissassemblerError {}
