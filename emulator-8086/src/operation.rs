use std::fmt;

use crate::{
    modrm::{DisplacementValue, EffectiveAddress},
    opcodes::OpcodeMnemonic,
    reg::Register,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Operand {
    EffectiveAddress(EffectiveAddress, DisplacementValue),
    Register(Register),
    DataByte(u8),
    DataWord(u16),
    SignedJump(i8),
}

// TODO: move all the string formatting stuff here
impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Operand::EffectiveAddress(effective_address, displacement_value) => {
                    effective_address.to_string_with_displacement(displacement_value)
                }
                Operand::Register(register) => register.to_string(),
                Operand::DataByte(b) => b.to_string(),
                Operand::DataWord(w) => w.to_string(),
                Operand::SignedJump(j) => j.to_string(),
            }
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Operation {
    // TODO: not sure if dest/src naming make the most sense
    opcode: OpcodeMnemonic,
    dest: Operand,
    src: Option<Operand>,
}

impl Operation {
    pub fn new(opcode: OpcodeMnemonic, dest: Operand, src: Option<Operand>) -> Self {
        Self { opcode, dest, src }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut op = self.opcode.to_string();

        op.push(' ');
        let dest_operand = self.dest.to_string();
        op.push_str(&dest_operand);

        if let Some(src_operand) = &self.src {
            op.push_str(", ");
            op.push_str(&src_operand.to_string());
        }

        // TODO: handle this ambiguous encoding, copied from disassem loop
        // if we have an instruction that references memory but the register doesn't imply the size, we need to include byte/word
        // handle ambiguous size encoding here - this might need to be cleaned up
        // if let Mode::Memory(_) = mode {
        //     dest.insert_str(0, if w { "word " } else { "byte " });
        // }
        write!(f, "{}", op)
    }
}
