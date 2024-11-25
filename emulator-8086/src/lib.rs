use std::{
    fmt,
    io::{Cursor, Read},
    str::FromStr,
};

use log::{debug, info};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type DestinationIsReg = bool;
type IsWord = bool;

#[derive(Debug)]
enum DissassemblerError {
    InvalidOpcode(u8),
    InvalidMode,
    InvalidRegister,
    FailedToDecode,
}

impl fmt::Display for DissassemblerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut error_str = match self {
            Self::InvalidOpcode(op) => format!("Invalid Opcode 0b{:08b}", op),
            Self::InvalidMode => "Invalid mode".to_owned(),
            Self::InvalidRegister => "Invalid Register".to_owned(),
            Self::FailedToDecode => "Failed to decode".to_owned(),
        };
        error_str.push('\n');

        write!(f, "{}", error_str)
    }
}

impl std::error::Error for DissassemblerError {}

#[derive(Debug, PartialEq, Eq)]
enum Opcode {
    MovRegisterMemoryToFromRegister(DestinationIsReg, IsWord),
    MovImmediateToRegister(IsWord, Register),
}

impl TryFrom<u8> for Opcode {
    type Error = DissassemblerError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            // register/memory to/from register
            val if value & 0b10001000 == 0b10001000 => {
                let direction = ((val & 0b10) != 0) as DestinationIsReg;
                let is_word = ((val & 0b1) != 0) as IsWord;
                Ok(Self::MovRegisterMemoryToFromRegister(direction, is_word))
            }
            // immediate to register
            val if value & 0b10110000 == 0b10110000 => {
                let is_word = ((val & 0b1000) != 0) as IsWord;
                let reg = Register::try_from_with_w(val, is_word)?;
                Ok(Self::MovImmediateToRegister(is_word, reg))
            }
            _ => Err(DissassemblerError::InvalidOpcode(value)),
        }
    }
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::MovRegisterMemoryToFromRegister(_, _) => "mov",
                Self::MovImmediateToRegister(_, _) => "mov",
            }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Register {
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
    fn try_from_with_w(
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
}

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    MemoryNoDisplacement,
    Memory8BitDisplacement,
    Memory16BitDisplacement,
    Register,
}

impl TryFrom<u8> for Mode {
    type Error = DissassemblerError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        let masked = value & 0b11000000;
        let masked = masked >> 6;

        let mode = match masked {
            0b00 => Mode::MemoryNoDisplacement,
            0b01 => Mode::Memory8BitDisplacement,
            0b10 => Mode::Memory16BitDisplacement,
            0b11 => Mode::Register,
            _ => return Err(DissassemblerError::InvalidMode),
        };

        Ok(mode)
    }
}

impl Mode {
    fn get_displacement_len(&self) -> usize {
        match self {
            Self::MemoryNoDisplacement => 0,
            Self::Memory8BitDisplacement => 1,
            Self::Memory16BitDisplacement => 1,
            // TODO: there is some caveat here
            Self::Register => 0,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Statement {
    opcode: Opcode,
    destination: String,
    source: String,
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let opcode = self.opcode.to_string();

        write!(f, "{} {}, {}", opcode, self.destination, self.source)
    }
}

#[derive(Debug)]
pub struct Disassembler {
    instructions_bin: Cursor<Vec<u8>>,
}

impl Disassembler {
    pub fn new(instructions: &[u8]) -> Self {
        Self {
            instructions_bin: Cursor::new(instructions.to_vec()),
        }
    }

    pub fn decode(&mut self) -> Result<String> {
        let mut decoded = String::from_str("bits 16\n")?;

        while let Some(statement) = self.decode_next_op()? {
            decoded.push('\n');

            let statement_str = &statement.to_string();
            info!("{}", &statement_str);
            decoded.push_str(statement_str);
        }

        Ok(decoded)
    }

    fn read_next(&mut self) -> Result<Option<u8>> {
        let mut next = [0u8; 1];
        let read_len = self.instructions_bin.read(&mut next)?;

        if read_len == 0 {
            return Ok(None);
        }

        debug!("read {:08b}", next[0]);

        Ok(Some(next[0]))
    }

    fn decode_next_op(&mut self) -> Result<Option<Statement>> {
        let next = self.read_next()?;

        match next {
            Some(next) => {
                let opcode = Opcode::try_from(next)?;

                let statement = match opcode {
                    // TODO: handle disp-lo,high
                    Opcode::MovRegisterMemoryToFromRegister(destination_is_reg, is_word) => {
                        let next_val = self.read_next()?.unwrap();

                        let rm = Register::try_from_with_w(next_val, is_word)?;
                        let mode = Mode::try_from(next_val)?;

                        let next_val = next_val >> 3;
                        let reg = Register::try_from_with_w(next_val, is_word)?;

                        let (destination, source) = if destination_is_reg {
                            (reg, rm)
                        } else {
                            (rm, reg)
                        };

                        Statement {
                            opcode,
                            destination: destination.to_string(),
                            source: source.to_string(),
                        }
                    }
                    Opcode::MovImmediateToRegister(is_word, reg) => {
                        let data = if is_word {
                            (self.read_next()?.unwrap() + self.read_next()?.unwrap()) as u16
                        } else {
                            self.read_next()?.unwrap() as u16
                        };

                        Statement {
                            opcode,
                            destination: reg.clone().to_string(),
                            source: data.to_string(),
                        }
                    } // _ => return Err(Box::new(DissassemblerError::FailedToDecode)),
                };
                Ok(Some(statement))
            }
            None => Ok(None),
        }
    }

    // fn decode_statement(&mut self, opcode: &Opcode) -> Result<Statement> {
    //     match opcode {
    //         _ => return Err(Box::new(DissassemblerError::FailedToDecode)),
    //     }
    // }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic_mov() -> Result<()> {
        let instructions: [u8; 2] = [0b10001001, 0b11011001];
        let mut d = Disassembler::new(&instructions);
        let statement = d.decode_next_op()?.unwrap();

        let expected = Statement {
            opcode: Opcode::MovRegisterMemoryToFromRegister(false, true),
            destination: Register::CX.to_string(),
            source: Register::BX.to_string(),
        };

        assert_eq!(expected, statement);
        assert_eq!(expected.to_string(), "mov cx, bx".to_owned());
        Ok(())
    }
}
