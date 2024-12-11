use std::fmt;

use crate::{DissassemblerError, IsWord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Register {
    AL,
    CL,
    DL,
    BL,
    AH,
    CH,
    DH,
    BH,
    AX,
    CX,
    DX,
    BX,
    SP,
    BP,
    SI,
    DI,
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Register::AL => "al",
                Register::CL => "cl",
                Register::DL => "dl",
                Register::BL => "bl",
                Register::AH => "ah",
                Register::CH => "ch",
                Register::DH => "dh",
                Register::BH => "bh",
                Register::AX => "ax",
                Register::CX => "cx",
                Register::DX => "dx",
                Register::BX => "bx",
                Register::SP => "sp",
                Register::BP => "bp",
                Register::SI => "si",
                Register::DI => "di",
            }
        )
    }
}

impl Register {
    /// Must shift before using this in the case of the "REG" field, this just checks the 3 LSB for register name
    pub fn try_from_with_w(
        value: u8,
        is_word: IsWord,
    ) -> std::result::Result<Self, DissassemblerError> {
        let masked = value & 0b111;

        let reg = match masked {
            0b000 => {
                if is_word {
                    Register::AX
                } else {
                    Register::AL
                }
            }
            0b001 => {
                if is_word {
                    Register::CX
                } else {
                    Register::CL
                }
            }
            0b010 => {
                if is_word {
                    Register::DX
                } else {
                    Register::DL
                }
            }
            0b011 => {
                if is_word {
                    Register::BX
                } else {
                    Register::BL
                }
            }
            0b100 => {
                if is_word {
                    Register::SP
                } else {
                    Register::AH
                }
            }
            0b101 => {
                if is_word {
                    Register::BP
                } else {
                    Register::CH
                }
            }
            0b110 => {
                if is_word {
                    Register::SI
                } else {
                    Register::DH
                }
            }
            0b111 => {
                if is_word {
                    Register::DI
                } else {
                    Register::BH
                }
            }
            _ => return Err(DissassemblerError::InvalidRegister),
        };

        Ok(reg)
    }

    pub fn accumulator_from_w(is_word: IsWord) -> Self {
        match is_word {
            true => Register::AX,
            false => Register::AL,
        }
    }
}
