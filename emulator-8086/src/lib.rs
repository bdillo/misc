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
    InvalidEffectiveAddress(u8),
    FailedToDecode,
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
            value if (value & 0b11111100) == 0b10001000 => {
                // why
                let direction = ((value & 0b10) != 0) as DestinationIsReg;
                let is_word = ((value & 0b1) != 0) as IsWord;
                Ok(Self::MovRegisterMemoryToFromRegister(direction, is_word))
            }
            // immediate to register
            value if (value & 0b11110000) == 0b10110000 => {
                let is_word = ((value & 0b1000) != 0) as IsWord;
                let reg = Register::try_from_with_w(value, is_word)?;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Displacement {
    /// No displacement
    None,
    /// 8 bit displacement
    Byte,
    /// 16 bit displacement
    Word,
}

impl Displacement {
    fn get_len(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Byte => 1,
            Self::Word => 2,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Memory(Displacement),
    // MemoryNoDisplacement,
    // Memory8BitDisplacement,
    // Memory16BitDisplacement,
    Register,
}

impl TryFrom<u8> for Mode {
    type Error = DissassemblerError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        let masked = value & 0b11000000;
        let masked = masked >> 6;

        let mode = match masked {
            0b00 => Self::Memory(Displacement::None),
            0b01 => Self::Memory(Displacement::Byte),
            0b10 => Self::Memory(Displacement::Word),
            0b11 => Self::Register,
            _ => return Err(DissassemblerError::InvalidMode),
        };

        Ok(mode)
    }
}

impl Mode {
    fn is_memory_mode(&self) -> bool {
        !matches!(self, Self::Register)
    }
}

#[derive(Debug)]
enum EffectiveAddress {
    DirectAddress,
    SingleReg(Register, Displacement),
    DoubleReg(Register, Register, Displacement),
}

impl EffectiveAddress {
    fn from_with_mode(value: u8, mode: Mode) -> Result<Self> {
        let displacement = match mode {
            Mode::Memory(displacement) => displacement,
            // TODO: is this right? not sure if we would ever have register mode here
            Mode::Register => Displacement::None,
        };

        let masked = value & 0b00000111;
        Ok(match masked {
            0b000 => Self::DoubleReg(Register::BX, Register::SI, displacement),
            0b001 => Self::DoubleReg(Register::BX, Register::DI, displacement),
            0b010 => Self::DoubleReg(Register::BP, Register::SI, displacement),
            0b011 => Self::DoubleReg(Register::BP, Register::DI, displacement),
            0b100 => Self::SingleReg(Register::SI, displacement),
            0b101 => Self::SingleReg(Register::DI, displacement),
            0b110 => {
                if displacement == Displacement::None {
                    Self::DirectAddress
                } else {
                    Self::SingleReg(Register::BP, displacement)
                }
            }
            0b111 => Self::SingleReg(Register::BX, displacement),
            _ => return Err(Box::new(DissassemblerError::InvalidEffectiveAddress(value))),
        })
    }
}

impl fmt::Display for EffectiveAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::DirectAddress => todo!(),
            Self::SingleReg(reg, _) => format!("[{}]", reg),
            Self::DoubleReg(first, second, _) => format!("[{} + {}]", first, second),
        };
        write!(f, "{}", s)
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

                        let mode = Mode::try_from(next_val)?;

                        let shifted_reg = next_val >> 3;
                        let reg = Register::try_from_with_w(shifted_reg, is_word)?;
                        let mut src = reg.to_string();

                        let mut dest = if let Mode::Memory(disp) = mode {
                            // effective address calculation
                            let effective_address =
                                EffectiveAddress::from_with_mode(next_val, mode)?;
                            effective_address.to_string()
                        } else {
                            // otherwise rm field is a register
                            let rm_reg = Register::try_from_with_w(next_val, is_word)?;
                            rm_reg.to_string()
                        };

                        if destination_is_reg {
                            std::mem::swap(&mut src, &mut dest)
                        }

                        Statement {
                            opcode,
                            destination: dest,
                            source: src,
                        }
                    }
                    Opcode::MovImmediateToRegister(is_word, reg) => {
                        let data = if is_word {
                            let low = self.read_next()?.unwrap() as u16;
                            let high = (self.read_next()?.unwrap() as u16) << 8;
                            low + high
                        } else {
                            self.read_next()?.unwrap() as u16
                        };

                        Statement {
                            opcode,
                            destination: reg.to_string(),
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
