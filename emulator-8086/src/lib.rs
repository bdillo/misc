use std::{
    fmt::{self, write},
    io::{Cursor, Read},
    str::FromStr,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type DestinationIsReg = bool;
type IsWord = bool;

#[derive(Debug)]
enum DissassemblerError {
    InvalidOpcode,
    InvalidMode,
    InvalidRegister,
    FailedToDecode,
}

impl fmt::Display for DissassemblerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DissassemblerError")
    }
}

impl std::error::Error for DissassemblerError {}

#[derive(Debug, PartialEq, Eq)]
enum Opcode {
    MovRegisterMemoryToFromRegister(DestinationIsReg, IsWord),
}

impl TryFrom<u8> for Opcode {
    type Error = DissassemblerError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            val if value & 0b10001000 == 0b10001000 => {
                let direction = ((val & 0b10) != 0) as DestinationIsReg;
                let is_word = ((val & 0b1) != 0) as IsWord;
                Ok(Opcode::MovRegisterMemoryToFromRegister(direction, is_word))
            }
            _ => Err(DissassemblerError::InvalidOpcode),
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
            }
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
struct Statement {
    opcode: Opcode,
    mode: Mode,
    destination: Register,
    source: Register,
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let opcode = self.opcode.to_string();
        let dest = self.destination.to_string();
        let source = self.source.to_string();

        write!(f, "{} {}, {}", opcode, dest, source)
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
        let mut decoded = String::from_str("bits 16\n\n")?;

        while let Some(statement) = self.decode_next()? {
            decoded.push_str(&statement.to_string());
            decoded.push('\n');
        }

        Ok(decoded)
    }

    fn decode_next(&mut self) -> Result<Option<Statement>> {
        let mut next: [u8; 2] = [0; 2];
        let read_len = self.instructions_bin.read(&mut next)?;

        if read_len == 0 {
            return Ok(None);
        }

        let opcode = Opcode::try_from(next[0])?;
        let statement = match opcode {
            Opcode::MovRegisterMemoryToFromRegister(destination_is_reg, is_word) => {
                let val = next[1];
                let rm = Register::try_from_with_w(val, is_word)?;
                let mode = Mode::try_from(val)?;

                let val = val >> 3;
                let reg = Register::try_from_with_w(val, is_word)?;

                let (destination, source) = if destination_is_reg {
                    (reg, rm)
                } else {
                    (rm, reg)
                };

                Statement {
                    opcode,
                    mode,
                    destination,
                    source,
                }
            }
            _ => return Err(Box::new(DissassemblerError::FailedToDecode)),
        };

        Ok(Some(statement))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic_mov() -> Result<()> {
        let instructions: [u8; 2] = [0b10001001, 0b11011001];
        let mut d = Disassembler::new(&instructions);
        let statement = d.decode_next()?.unwrap();

        let expected = Statement {
            opcode: Opcode::MovRegisterMemoryToFromRegister(false, true),
            mode: Mode::Register,
            destination: Register::CX,
            source: Register::BX,
        };

        assert_eq!(expected, statement);
        assert_eq!(expected.to_string(), "mov cx, bx".to_owned());
        Ok(())
    }
}
