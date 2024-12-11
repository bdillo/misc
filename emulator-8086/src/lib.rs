pub mod modrm;
pub mod opcodes;
pub mod reg;

use std::{
    fmt::{self, Display},
    io::{Cursor, Read, Seek},
    str::FromStr,
};

use log::{debug, info};
use modrm::{parse_mod_reg_rm, parse_mod_rm, Displacement, EffectiveAddress, Mode, Rm};
use opcodes::{NextFieldType, OpcodeContext, OpcodeMnemonic};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type DestinationIsReg = bool;
type IsWord = bool;
type IsSigned = bool;

#[derive(Debug)]
pub enum DissassemblerError {
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
struct AsmStatement<T: Display> {
    // TODO: make this better, handle all the string formatting for the instruction here
    opcode: OpcodeMnemonic,
    destination: T,
    source: T,
}

impl<T> fmt::Display for AsmStatement<T>
where
    T: Display,
{
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
        Ok(self.read_next()?.unwrap())
    }

    /// Read word (u16)
    fn read_word(&mut self) -> Result<u16> {
        let low = self.read_next()?.unwrap() as u16;
        let high = (self.read_next()?.unwrap() as u16) << 8;
        Ok(low + high)
    }

    /// Read based off displacement (0, 1, 2 bytes)
    fn read_displacement(&mut self, disp: Displacement) -> Result<Option<u16>> {
        Ok(match disp {
            Displacement::None => None,
            Displacement::Byte => Some(self.read_next()?.unwrap() as u16),
            Displacement::Word => Some(self.read_word()?),
        })
    }

    /// Peeks the next byte, but returns the offset to where it originally was
    fn peek_next(&mut self) -> Result<u8> {
        let val = self.read_expecting()?;
        self.instructions_bin.seek_relative(-1)?;
        Ok(val)
    }

    /// Sometimes we need to read more bytes depending on our effective address calculation, so put this here
    fn rm_to_string(&mut self, rm: Rm) -> Result<String> {
        Ok(match rm {
            Rm::EffectiveAddressCalculation(effective_addr, disp) => {
                let disp_val = self.read_displacement(disp)?;
                effective_addr.to_string_with_displacement(disp_val)
            }
            Rm::Register(rm_reg) => rm_reg.to_string(),
        })
    }

    fn decode_next_op(&mut self) -> Result<Option<AsmStatement<String>>> {
        let opcode_byte = self.read_next()?;

        match opcode_byte {
            Some(opcode) => {
                let mut opcode_ctx = OpcodeContext::try_from(opcode)?;
                debug!("opcode: {:?}", opcode_ctx);

                // if we need the next byte, just peek it so we can get our mnemonic. We'll read this byte again but
                // this just makes the logic a bit simpler here
                // TODO: check if all of the opcodes where we have NeedsNextByte would result in ModOpcodeContRm, then
                // we wouldn't need to peek
                if matches!(
                    opcode_ctx.mnemonic(),
                    opcodes::OpcodeMnemonic::NeedsNextByte
                ) {
                    let next = self.peek_next()?;
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

                // need to reorganize this
                match opcode_ctx.next_field() {
                    NextFieldType::ModRegRm => {
                        let mod_reg_rm = self.read_expecting()?;
                        let (_mode, reg, rm) = parse_mod_reg_rm(
                            mod_reg_rm,
                            opcode_ctx.w().expect("W bit not found!"),
                        )?;

                        let mut dest = self.rm_to_string(rm)?;
                        let mut src = reg.to_string();

                        if opcode_ctx.d().expect("Need direction set!") {
                            // destination is reg
                            std::mem::swap(&mut dest, &mut src);
                        }

                        Ok(Some(AsmStatement {
                            opcode: *opcode_ctx.mnemonic(),
                            destination: dest,
                            source: src,
                        }))
                    }
                    NextFieldType::ModOpcodeContRm => {
                        let mod_op_rm = self.read_expecting()?;
                        let (mode, rm) =
                            parse_mod_rm(mod_op_rm, opcode_ctx.w().expect("W bit not found!"))?;

                        let w = opcode_ctx.w().expect("Expected w!");

                        let mut dest = self.rm_to_string(rm)?;

                        // handle ambiguous size encoding here - this might need to be cleaned up
                        if let Mode::Memory(_) = mode {
                            dest.insert_str(0, if w { "word " } else { "byte " });
                        }

                        // if we have an s field, the size of data depends on s and w (2 bytes if sw == 01)
                        let src = if let Some(s) = opcode_ctx.s() {
                            if !s && w {
                                self.read_word()?
                            } else {
                                self.read_expecting()? as u16
                            }
                        // otherwise we just go off the w bit
                        } else if w {
                            self.read_word()?
                        } else {
                            self.read_expecting()? as u16
                        }
                        .to_string();

                        Ok(Some(AsmStatement {
                            opcode: *opcode_ctx.mnemonic(),
                            destination: dest,
                            source: src,
                        }))
                    }
                    NextFieldType::Data => {
                        // TODO: clean up
                        let reg = opcode_ctx.reg().expect("Expected reg!");
                        let data = if opcode_ctx.w().expect("Expected w!") {
                            self.read_word()?
                        } else {
                            self.read_expecting()? as u16
                        };

                        Ok(Some(AsmStatement {
                            opcode: *opcode_ctx.mnemonic(),
                            destination: reg.to_string(),
                            source: data.to_string(),
                        }))
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
    use opcodes::OpcodeMnemonic;
    use reg::Register;

    use super::*;

    #[test]
    fn test_basic_mov() -> Result<()> {
        let instructions: [u8; 2] = [0b10001001, 0b11011001];
        let mut d = Disassembler::new(&instructions);
        let statement = d.decode_next_op()?.unwrap();

        let expected = AsmStatement {
            opcode: OpcodeMnemonic::Mov,
            destination: Register::CX.to_string(),
            source: Register::BX.to_string(),
        };

        assert_eq!(expected, statement);
        assert_eq!(expected.to_string(), "mov cx, bx".to_owned());
        Ok(())
    }
}
