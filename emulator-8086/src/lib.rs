use std::{
    fmt,
    io::{Cursor, Read},
    str::FromStr,
};

use log::{debug, info};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type DestinationIsReg = bool;
type IsWord = bool;
type IsSigned = bool;

#[derive(Debug)]
enum DissassemblerError {
    InvalidOpcode(u8),
    InvalidMode,
    InvalidRegister,
    InvalidEffectiveAddress(u8),
    // FailedToDecode,
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
            } // Self::FailedToDecode => "Failed to decode".to_owned(),
        };
        error_str.push('\n');

        write!(f, "{}", error_str)
    }
}

impl std::error::Error for DissassemblerError {}

#[derive(Debug, PartialEq, Eq)]
// TODO: clean up these names to be more consistent
enum Opcode {
    MovRegisterMemoryToFromRegister(DestinationIsReg, IsWord),
    MovImmediateToRegister(IsWord, Register),
    AddRegMemWithRegToEither(DestinationIsReg, IsWord),
    AddImmediateToRegOrMem(IsSigned, IsWord),
    AddImmediateToAccumulator(IsWord),
}

impl TryFrom<u8> for Opcode {
    type Error = DissassemblerError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            // mov register/memory to/from register
            value if (value & 0b11111100) == 0b10001000 => {
                let direction = ((value & 0b10) != 0) as DestinationIsReg;
                let is_word = ((value & 0b1) != 0) as IsWord;
                Ok(Self::MovRegisterMemoryToFromRegister(direction, is_word))
            }
            // mov immediate to register
            value if (value & 0b11110000) == 0b10110000 => {
                let is_word = ((value & 0b1000) != 0) as IsWord;
                let reg = Register::try_from_with_w(value, is_word)?;
                Ok(Self::MovImmediateToRegister(is_word, reg))
            }
            // add reg/memory with register to either
            value if (value & 0b11111100) == 0b0 => {
                let direction = ((value & 0b10) != 0) as DestinationIsReg;
                let is_word = ((value & 0b1) != 0) as IsWord;
                Ok(Self::AddRegMemWithRegToEither(direction, is_word))
            }
            // add immediate to register/memory
            value if (value & 0b11111100) == 0b10000000 => {
                let is_signed = ((value & 0b10) != 0) as IsSigned;
                let is_word = ((value & 0b1) != 0) as IsWord;
                Ok(Self::AddImmediateToRegOrMem(is_signed, is_word))
            }
            // add immediate to accumulator
            value if (value & 0b11111110) == 0b00000100 => {
                let is_word = ((value & 0b1) != 0) as IsWord;
                Ok(Self::AddImmediateToAccumulator(is_word))
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
                Self::AddRegMemWithRegToEither(_, _) => "add",
                Self::AddImmediateToRegOrMem(_, _) => "add",
                Self::AddImmediateToAccumulator(_) => "add",
            }
        )
    }
}

impl Opcode {
    fn to_string_with_w(&self, is_word: IsWord) -> String {
        let mut s = self.to_string();
        if is_word {
            s.push_str(" word");
        } else {
            s.push_str(" byte");
        }
        s
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Mode {
    Memory(Displacement),
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

#[derive(Debug)]
enum EffectiveAddress {
    DirectAddress,
    SingleReg(Register),
    DoubleReg(Register, Register),
}

impl EffectiveAddress {
    fn from_with_mode(value: u8, mode: Mode) -> Result<Self> {
        let displacement = match mode {
            Mode::Memory(displacement) => displacement,
            Mode::Register => {
                // TODO: make error
                panic!("can't have register mode with effective address calculation!")
            }
        };

        let masked = value & 0b00000111;
        Ok(match masked {
            0b000 => Self::DoubleReg(Register::BX, Register::SI),
            0b001 => Self::DoubleReg(Register::BX, Register::DI),
            0b010 => Self::DoubleReg(Register::BP, Register::SI),
            0b011 => Self::DoubleReg(Register::BP, Register::DI),
            0b100 => Self::SingleReg(Register::SI),
            0b101 => Self::SingleReg(Register::DI),
            0b110 => {
                if displacement == Displacement::None {
                    Self::DirectAddress
                } else {
                    Self::SingleReg(Register::BP)
                }
            }
            0b111 => Self::SingleReg(Register::BX),
            _ => return Err(Box::new(DissassemblerError::InvalidEffectiveAddress(value))),
        })
    }
}

impl fmt::Display for EffectiveAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::DirectAddress => todo!(),
            Self::SingleReg(reg) => format!("[{}]", reg),
            Self::DoubleReg(first, second) => format!("[{} + {}]", first, second),
        };
        write!(f, "{}", s)
    }
}

impl EffectiveAddress {
    fn to_string_with_displacement(&self, disp: Option<u16>) -> String {
        let mut s = String::new();
        match self {
            Self::DirectAddress => todo!(),
            Self::SingleReg(reg) => {
                s.push_str(&format!("[{}", reg));
                if let Some(disp_val) = disp {
                    s.push_str(&format!(" + {}", disp_val));
                }
                s.push(']');
            }
            Self::DoubleReg(first, second) => {
                s.push_str(&format!("[{} + {}", first, second));
                if let Some(disp_val) = disp {
                    s.push_str(&format!(" + {}", disp_val));
                }
                s.push(']');
            }
        };
        s
    }
}

#[derive(Debug)]
enum RmField {
    EffectiveAddressCalculation(EffectiveAddress, Displacement),
    Register(Register),
}

fn parse_mod_reg_rm(value: u8, is_word: IsWord) -> Result<(Mode, Register, RmField)> {
    // parse mode - this can have a displacement
    // if mode == register mode, then rm field is a register, otherwise it's an effective address calculation
    let mode = Mode::try_from(value)?;

    let shifted_reg = value >> 3;
    let register = Register::try_from_with_w(shifted_reg, is_word)?;

    let rm = match mode {
        Mode::Memory(disp) => RmField::EffectiveAddressCalculation(
            EffectiveAddress::from_with_mode(value, mode)?,
            disp,
        ),
        Mode::Register => RmField::Register(Register::try_from_with_w(value, is_word)?),
    };

    Ok((mode, register, rm))
}

#[derive(Debug, PartialEq, Eq)]
struct Statement {
    opcode: String,
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

    fn read_word(&mut self) -> Result<u16> {
        let low = self.read_next()?.unwrap() as u16;
        let high = (self.read_next()?.unwrap() as u16) << 8;
        Ok(low + high)
    }

    fn read_displacement(&mut self, disp: Displacement) -> Result<Option<u16>> {
        Ok(match disp {
            Displacement::None => None,
            Displacement::Byte => Some(self.read_next()?.unwrap() as u16),
            Displacement::Word => Some(self.read_word()?),
        })
    }

    fn decode_next_op(&mut self) -> Result<Option<Statement>> {
        let next = self.read_next()?;

        match next {
            Some(next) => {
                let opcode = Opcode::try_from(next)?;
                debug!("{:?}", opcode);

                let statement = match opcode {
                    Opcode::MovRegisterMemoryToFromRegister(destination_is_reg, is_word) => {
                        let next = self.read_next()?.unwrap();

                        let (_mode, register, rm) = parse_mod_reg_rm(next, is_word)?;

                        let mut src = register.to_string();

                        let mut dest = match rm {
                            RmField::EffectiveAddressCalculation(effective_addr, disp) => {
                                let disp_val = self.read_displacement(disp)?;
                                effective_addr.to_string_with_displacement(disp_val)
                            }
                            RmField::Register(rm_reg) => rm_reg.to_string(),
                        };

                        if destination_is_reg {
                            std::mem::swap(&mut src, &mut dest)
                        }

                        Statement {
                            opcode: opcode.to_string(),
                            destination: dest,
                            source: src,
                        }
                    }
                    Opcode::MovImmediateToRegister(is_word, reg) => {
                        let data = if is_word {
                            self.read_word()?
                        } else {
                            self.read_next()?.unwrap() as u16
                        };

                        Statement {
                            opcode: opcode.to_string(),
                            destination: reg.to_string(),
                            source: data.to_string(),
                        }
                    }
                    Opcode::AddRegMemWithRegToEither(destination_is_reg, is_word) => {
                        let next = self.read_next()?.unwrap();

                        let (_mode, register, rm) = parse_mod_reg_rm(next, is_word)?;

                        let mut src = register.to_string();

                        let mut dest = match rm {
                            RmField::EffectiveAddressCalculation(effective_addr, disp) => {
                                let disp_val = self.read_displacement(disp)?;
                                effective_addr.to_string_with_displacement(disp_val)
                            }
                            RmField::Register(rm_reg) => rm_reg.to_string(),
                        };

                        if destination_is_reg {
                            std::mem::swap(&mut src, &mut dest)
                        }

                        Statement {
                            opcode: opcode.to_string(),
                            destination: dest,
                            source: src,
                        }
                    }
                    Opcode::AddImmediateToRegOrMem(is_signed, is_word) => {
                        let next = self.read_next()?.unwrap();
                        let (_mode, _, rm) = parse_mod_reg_rm(next, is_word)?;

                        let dest = match rm {
                            RmField::EffectiveAddressCalculation(effective_addr, disp) => {
                                let disp_val = self.read_displacement(disp)?;
                                effective_addr.to_string_with_displacement(disp_val)
                            }
                            RmField::Register(reg) => reg.to_string(),
                        };

                        let data = if !is_signed && is_word {
                            self.read_word()?
                        } else {
                            self.read_next()?.unwrap() as u16
                        };

                        Statement {
                            opcode: opcode.to_string_with_w(is_word),
                            destination: dest,
                            source: data.to_string(),
                        }
                    }
                    Opcode::AddImmediateToAccumulator(is_word) => {
                        let (reg, data) = if is_word {
                            (Register::AX, self.read_word()?)
                        } else {
                            (Register::AL, self.read_next()?.unwrap() as u16)
                        };

                        Statement {
                            opcode: opcode.to_string(),
                            destination: reg.to_string(),
                            source: data.to_string(),
                        }
                    }
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
            opcode: Opcode::MovRegisterMemoryToFromRegister(false, true).to_string(),
            destination: Register::CX.to_string(),
            source: Register::BX.to_string(),
        };

        assert_eq!(expected, statement);
        assert_eq!(expected.to_string(), "mov cx, bx".to_owned());
        Ok(())
    }
}
