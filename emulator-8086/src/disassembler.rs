use std::{
    io::{Cursor, Read, Seek},
    str::FromStr,
};

use crate::opcodes::{NextFieldType, OpcodeContext, OpcodeMnemonic};
use crate::operation::Operation;
use crate::{
    modrm::{parse_mod_reg_rm, parse_mod_rm, DisplacementLen, DisplacementValue, Rm},
    operation::Operand,
};
use log::{debug, info};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
// type DestinationIsReg = bool;
// type IsWord = bool;
// type IsSigned = bool;

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

    /// Main loop
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

    /// Read next byte - returns None if no more instructions
    fn read_next(&mut self) -> Result<Option<u8>> {
        let mut next = [0u8; 1];
        let read_len = self.instructions_bin.read(&mut next)?;

        if read_len == 0 {
            return Ok(None);
        }

        debug!("read {:08b}", next[0]);

        Ok(Some(next[0]))
    }

    /// Read expecting to panic if we can't read the next byte
    fn read_expecting(&mut self) -> Result<u8> {
        Ok(self.read_next()?.expect("Failed to read next byte!"))
    }

    /// Read word (u16)
    fn read_word(&mut self) -> Result<u16> {
        let low = self.read_expecting()? as u16;
        let high = (self.read_expecting()? as u16) << 8;
        Ok(low + high)
    }

    /// Read based off displacement (0, 1, 2 bytes)
    fn read_displacement(&mut self, disp: DisplacementLen) -> Result<DisplacementValue> {
        Ok(match disp {
            DisplacementLen::None => DisplacementValue::None,
            DisplacementLen::Byte => DisplacementValue::Byte(self.read_expecting()?),
            DisplacementLen::Word => DisplacementValue::Word(self.read_word()?),
        })
    }

    /// Peeks the next byte, but returns the offset to where it originally was
    fn peek(&mut self) -> Result<u8> {
        let val = self.read_expecting()?;
        self.instructions_bin.seek_relative(-1)?;
        Ok(val)
    }

    /// Convert RM field to an operand - need this here as we may have to read more bytes depending on the displacement
    fn rm_to_operand(&mut self, rm: Rm) -> Result<Operand> {
        Ok(match rm {
            Rm::EffectiveAddressCalculation(effective_address, displacement_len) => {
                let disp_val = self.read_displacement(displacement_len)?;
                Operand::EffectiveAddress(effective_address, disp_val)
            }
            Rm::Register(register) => Operand::Register(register),
        })
    }

    fn decode_next_op(&mut self) -> Result<Option<Operation>> {
        let opcode_byte = self.read_next()?;

        match opcode_byte {
            Some(opcode) => {
                let mut opcode_ctx = OpcodeContext::try_from(opcode)?;
                debug!("opcode: {:?}", opcode_ctx);

                // if we need the next byte, just peek it so we can get our mnemonic. We'll read this byte again but
                // this just makes the logic a bit simpler here
                // TODO: check if all of the opcodes where we have NeedsNextByte would result in ModOpcodeContRm, then
                // we wouldn't need to peek
                if matches!(opcode_ctx.mnemonic(), OpcodeMnemonic::NeedsNextByte) {
                    let next = self.peek()?;
                    opcode_ctx.with_next_byte(next);
                    debug!("updated opcode: {:?}", opcode_ctx);
                }

                // don't actually need to do this - if an opcode has mod/reg/rm, it doesn't have data (where would it go?)
                // all opcodes w/ mod _ rm (no reg) must have data
                // disp comes from mod field - if there's a mod field there is the possibility of displacement
                // how about jmp statements?
                // kinda the same just need to add new fields to support
                // so actually the previous example (from below in next_field part) should work just fine

                // TODO: figure out when we need to specify byte/word in the asm op
                // when is it needed? looks like when we have ambiguous codings from effective address calculation
                // so when mode is memory mode?

                match opcode_ctx.next_field() {
                    NextFieldType::ModRegRm => {
                        let mod_reg_rm = self.read_expecting()?;
                        let (_mode, reg, rm) = parse_mod_reg_rm(
                            mod_reg_rm,
                            opcode_ctx.w().expect("W bit not found!"),
                        )?;

                        let mut dest = self.rm_to_operand(rm)?;
                        let mut src = Operand::Register(reg);

                        if opcode_ctx.d().expect("Need direction set!") {
                            // destination is reg
                            std::mem::swap(&mut dest, &mut src);
                        }

                        Ok(Some(Operation::new(
                            *opcode_ctx.mnemonic(),
                            dest,
                            Some(src),
                        )))
                    }
                    NextFieldType::ModOpcodeContRm => {
                        let mod_op_rm = self.read_expecting()?;
                        let (_mode, rm) =
                            parse_mod_rm(mod_op_rm, opcode_ctx.w().expect("W bit not found!"))?;

                        let w = opcode_ctx.w().expect("Expected w!");

                        let dest = self.rm_to_operand(rm)?;

                        // handle ambiguous size encoding here - this might need to be cleaned up
                        // if let Mode::Memory(_) = mode {
                        //     dest.insert_str(0, if w { "word " } else { "byte " });
                        // }

                        // TODO: not all of these have data! need to add has_data back into opcode ctx
                        // if we have an s field, the size of data depends on s and w (2 bytes if sw == 01)
                        let src = if let Some(s) = opcode_ctx.s() {
                            if !s && w {
                                Operand::DataWord(self.read_word()?)
                            } else {
                                Operand::DataByte(self.read_expecting()?)
                            }
                        // otherwise we just go off the w bit
                        } else if w {
                            Operand::DataWord(self.read_word()?)
                        } else {
                            Operand::DataByte(self.read_expecting()?)
                        };

                        Ok(Some(Operation::new(
                            *opcode_ctx.mnemonic(),
                            dest,
                            Some(src),
                        )))
                    }
                    NextFieldType::Data => {
                        // TODO: clean up
                        let reg = Operand::Register(opcode_ctx.reg().expect("Expected reg!"));
                        let data = if opcode_ctx.w().expect("Expected w!") {
                            Operand::DataWord(self.read_word()?)
                        } else {
                            Operand::DataByte(self.read_expecting()?)
                        };

                        Ok(Some(Operation::new(
                            *opcode_ctx.mnemonic(),
                            reg,
                            Some(data),
                        )))
                    }
                    NextFieldType::IpInc8 => {
                        let jump_offset = Operand::SignedJump(self.read_expecting()? as i8);

                        // TODO: fix this up for formatting instructions that don't have src/dest, this is just a hack for now
                        Ok(Some(Operation::new(
                            *opcode_ctx.mnemonic(),
                            jump_offset,
                            None,
                        )))
                    }
                    _ => todo!(),
                }
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::reg::Register;

    use super::*;

    #[test]
    fn test_basic_mov() -> Result<()> {
        let instructions: [u8; 2] = [0b10001001, 0b11011001];
        let mut d = Disassembler::new(&instructions);
        let statement = d.decode_next_op()?.unwrap();

        let expected = Operation::new(
            OpcodeMnemonic::Mov,
            Operand::Register(Register::CX),
            Some(Operand::Register(Register::BX)),
        );

        assert_eq!(expected, statement);
        assert_eq!(expected.to_string(), "mov cx, bx".to_owned());
        Ok(())
    }
}
