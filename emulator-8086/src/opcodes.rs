use core::{fmt, panic};

use crate::{reg::Register, DestinationIsReg, DissassemblerError, IsWord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpcodeMnemonic {
    Mov,
    Add,
    Sub,
    NeedsNextByte,
}

impl fmt::Display for OpcodeMnemonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Mov => "mov",
                Self::Add => "add",
                Self::Sub => "sub",
                Self::NeedsNextByte => todo!(),
            }
        )
    }
}

impl OpcodeMnemonic {
    /// For when the opcode mnemonic needs bytes 5-3 from the mod rm field
    pub fn with_mod_rm(opcode_val: u8, mod_rm: u8) -> Self {
        let masked = mod_rm & 0b00111000;
        let shifted = masked >> 3;

        match shifted {
            0b000 => match opcode_val {
                0b11000110..=0b11000111 => OpcodeMnemonic::Mov,
                0b10000000..=0b10000011 => OpcodeMnemonic::Add,
                _ => panic!("unsupported vals {:b} {:b}", opcode_val, mod_rm),
            },
            0b101 => match opcode_val {
                0b10000000..=0b10000011 => OpcodeMnemonic::Sub,
                _ => panic!("unsupported vals {:b} {:b}", opcode_val, mod_rm),
            },
            _ => panic!("unsupported vals {:b} {:b}", opcode_val, mod_rm),
        }
    }
}

#[derive(Debug)]
pub enum NextFieldType {
    ModRegRm,
    ModOpcodeContRm,
    Data,
    Addr,
    None,
}

#[derive(Debug)]
pub enum AmbiguousOperandEncoding {
    Byte,
    Word,
}

impl fmt::Display for AmbiguousOperandEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AmbiguousOperandEncoding::Byte => "byte",
                AmbiguousOperandEncoding::Word => "word",
            }
        )
    }
}

#[derive(Debug)]
pub struct OpcodeContext {
    first_byte_raw: u8,
    mnemonic: OpcodeMnemonic,
    next_field: NextFieldType,
    d: Option<DestinationIsReg>,
    w: Option<IsWord>,
    s: Option<bool>,
    reg: Option<Register>,
    has_data: bool,
}

impl TryFrom<u8> for OpcodeContext {
    type Error = DissassemblerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            // mov register/memory to/from register
            0b10001000..=0b10001011 => OpcodeContext {
                first_byte_raw: value,
                mnemonic: OpcodeMnemonic::Mov,
                next_field: NextFieldType::ModRegRm,
                d: Some((value & 0b10) != 0),
                w: Some((value & 0b1) != 0),
                s: None,
                reg: None,
                has_data: false,
            },
            // mov immediate to register/memory
            0b11000110..=0b11000111 => OpcodeContext {
                first_byte_raw: value,
                mnemonic: OpcodeMnemonic::Mov,
                // TODO: fix
                next_field: NextFieldType::ModRegRm,
                d: None,
                w: Some((value & 0b1) != 0),
                s: None,
                reg: None,
                has_data: true,
            },
            // mov immediate to register
            0b10110000..=0b10111111 => {
                let w = (value & 0b00001000) != 0;
                OpcodeContext {
                    first_byte_raw: value,
                    mnemonic: OpcodeMnemonic::Mov,
                    // TODO: fix, overlaps with has_data
                    next_field: NextFieldType::Data,
                    d: None,
                    w: Some(w),
                    s: None,
                    reg: Some(Register::try_from_with_w(value, w)?),
                    has_data: true,
                }
            }
            // add reg/memory with register to either
            0b00000000..=0b00000011 => OpcodeContext {
                first_byte_raw: value,
                mnemonic: OpcodeMnemonic::Add,
                next_field: NextFieldType::ModRegRm,
                d: Some((value & 0b10) != 0),
                w: Some((value & 0b1) != 0),
                s: None,
                reg: None,
                has_data: false,
            },
            // add, adc immediate to register/memory
            0b10000000..=0b10000011 => OpcodeContext {
                first_byte_raw: value,
                mnemonic: OpcodeMnemonic::NeedsNextByte,
                next_field: NextFieldType::ModOpcodeContRm,
                d: None,
                w: Some((value & 0b1) != 0),
                s: Some((value & 0b10) != 0),
                reg: None,
                has_data: true,
            },
            // add, immediate to accumulator
            0b00000100..=0b00000101 => {
                let w_val = (value & 0b1) != 0;
                let reg = Register::accumulator_from_w(w_val);
                OpcodeContext {
                    first_byte_raw: value,
                    mnemonic: OpcodeMnemonic::Add,
                    next_field: NextFieldType::Data,
                    d: None,
                    w: Some(w_val),
                    s: None,
                    reg: Some(reg),
                    has_data: true,
                }
            }
            // sub, reg/memory and register to either
            0b00101000..=0b00101011 => OpcodeContext {
                first_byte_raw: value,
                mnemonic: OpcodeMnemonic::Sub,
                next_field: NextFieldType::ModRegRm,
                d: Some((value & 0b10) != 0),
                w: Some((value & 0b1) != 0),
                s: None,
                reg: None,
                has_data: false,
            },
            // sub, immediate from accumulator
            0b00101100..=0b00101101 => {
                let w_val = (value & 0b1) != 0;
                let reg = Register::accumulator_from_w(w_val);
                OpcodeContext {
                    first_byte_raw: value,
                    mnemonic: OpcodeMnemonic::Sub,
                    next_field: NextFieldType::Data,
                    d: None,
                    w: Some(w_val),
                    s: None,
                    reg: Some(reg),
                    has_data: true,
                }
            }
            _ => return Err(DissassemblerError::InvalidOpcode(value)),
        })
    }
}

impl OpcodeContext {
    pub fn mnemonic(&self) -> &OpcodeMnemonic {
        &self.mnemonic
    }

    pub fn next_field(&self) -> &NextFieldType {
        &self.next_field
    }

    pub fn d(&self) -> Option<bool> {
        self.d
    }

    pub fn w(&self) -> Option<bool> {
        self.w
    }

    pub fn s(&self) -> Option<bool> {
        self.s
    }

    pub fn reg(&self) -> &Option<Register> {
        &self.reg
    }

    pub fn has_data(&self) -> bool {
        self.has_data
    }

    pub fn with_next_byte(&mut self, next_byte: u8) {
        let mnemonic = OpcodeMnemonic::with_mod_rm(self.first_byte_raw, next_byte);
        self.mnemonic = mnemonic;
    }
}
