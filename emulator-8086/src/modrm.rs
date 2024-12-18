use std::fmt;

use crate::{reg::Register, DissassemblerError, IsWord};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplacementLen {
    None,
    Byte,
    Word,
}

// TODO: is this needed?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplacementValue {
    None,
    Byte(u8),
    Word(u16),
}

impl fmt::Display for DisplacementValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                // should this be 0 here?
                DisplacementValue::None => "0".to_owned(),
                DisplacementValue::Byte(b) => b.to_string(),
                DisplacementValue::Word(w) => w.to_string(),
            }
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Memory(DisplacementLen),
    Register,
}

impl TryFrom<u8> for Mode {
    type Error = DissassemblerError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        let masked = value & 0b11000000;
        let masked = masked >> 6;

        let mode = match masked {
            0b00 => Self::Memory(DisplacementLen::None),
            0b01 => Self::Memory(DisplacementLen::Byte),
            0b10 => Self::Memory(DisplacementLen::Word),
            0b11 => Self::Register,
            _ => return Err(DissassemblerError::InvalidMode),
        };

        Ok(mode)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum EffectiveAddress {
    DirectAddress,
    SingleReg(Register),
    DoubleReg(Register, Register),
}

impl EffectiveAddress {
    pub fn from_with_mode(value: u8, mode: Mode) -> Result<Self> {
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
                if displacement == DisplacementLen::None {
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
    pub fn to_string_with_displacement(&self, disp: &DisplacementValue) -> String {
        let mut s = String::new();
        match self {
            Self::DirectAddress => {
                s.push_str(&format!("[{}]", disp.to_string()));
            }
            Self::SingleReg(reg) => {
                s.push_str(&format!("[{}", reg));
                match disp {
                    DisplacementValue::None => (),
                    DisplacementValue::Byte(v) => s.push_str(&format!(" + {}", v)),
                    DisplacementValue::Word(v) => s.push_str(&format!(" + {}", v)),
                }
                s.push(']');
            }
            Self::DoubleReg(first, second) => {
                s.push_str(&format!("[{} + {}", first, second));
                match disp {
                    DisplacementValue::None => (),
                    DisplacementValue::Byte(v) => s.push_str(&format!(" + {}", v)),
                    DisplacementValue::Word(v) => s.push_str(&format!(" + {}", v)),
                }
                s.push(']');
            }
        };
        s
    }
}

#[derive(Debug)]
pub enum Rm {
    EffectiveAddressCalculation(EffectiveAddress, DisplacementLen),
    Register(Register),
}

/// Read a mod reg rm byte
pub fn parse_mod_reg_rm(value: u8, is_word: IsWord) -> Result<(Mode, Register, Rm)> {
    let shifted = value >> 3;
    let register = Register::try_from_with_w(shifted, is_word)?;

    let (mode, rm) = parse_mod_rm(value, is_word)?;

    Ok((mode, register, rm))
}

/// Read a mod rm byte, ignoring bytes 5-3 (no reg)
pub fn parse_mod_rm(value: u8, is_word: IsWord) -> Result<(Mode, Rm)> {
    // parse mode - this can have a displacement
    // if mode == register mode, then rm field is a register, otherwise it's an effective address calculation
    let mode = Mode::try_from(value)?;

    let rm = match mode {
        Mode::Memory(mut disp) => {
            let effective_addr = EffectiveAddress::from_with_mode(value, mode)?;

            // if mode == 00 (memory, no disp) and rm == 110 (direct address), add 16 bit disp
            if disp == DisplacementLen::None && effective_addr == EffectiveAddress::DirectAddress {
                disp = DisplacementLen::Word
            }

            Rm::EffectiveAddressCalculation(effective_addr, disp)
        }
        Mode::Register => Rm::Register(Register::try_from_with_w(value, is_word)?),
    };

    Ok((mode, rm))
}
