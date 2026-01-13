use clap::Parser;
use libvmr::{Operation, Register};
use log::{debug, error};
use num_traits::cast::FromPrimitive;
use std::path::PathBuf;

pub type Immediate = u16;
pub type Offset = i16;

#[derive(Debug)]
struct VirtualMachine {
    code: Vec<u8>,
    memory: Vec<u16>,
    equal: bool,
    current_instruction: u16,
    registers: Vec<u16>,
}

impl std::fmt::Display for VirtualMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CODE\n")?;
        for byte in self.code.iter() {
            write!(f, "{byte:02x} ")?;
        }
        write!(f, "\nMEMORY\n")?;
        let last_index = self.memory.iter().rposition(|&x| x != 0);

        match last_index {
            Some(index) => {
                for byte in self.memory[0..index].iter() {
                    write!(f, "{byte:04x} ")?;
                }
            }
            None => write!(f, "<EMPTY> ")?,
        }
        write!(
            f,
            "\nInstruction Address: {:04x}\n",
            self.current_instruction
        )?;
        write!(
            f,
            "Instruction Byte: {:02x}\n",
            self.code[self.current_instruction as usize]
        )?;
        write!(f, "Registers\n")?;
        for reg in self.registers.iter() {
            write!(f, "{:04x} ", reg)?;
        }
        if self.equal {
            write!(f, "Equal: Set\n")?;
        } else {
            write!(f, "Equal: Unset\n")?;
        }

        Ok(())
    }
}
#[derive(Debug)]
pub enum RuntimeError {
    InstructionOOB,
    MemoryOOB,
    InvalidOPCode(u8),
    InvalidRegister,
    InvalidCode,
    OffsetOOB,
    Halted,
    StackOverflow,
}

impl VirtualMachine {
    fn next_reg(&self, offset: u16) -> Result<usize, RuntimeError> {
        let reg_byte = self.next(offset)?;
        Ok(Register::from_u8(reg_byte).ok_or(RuntimeError::InvalidRegister)? as usize)
    }

    fn next<T>(&self, offset: u16) -> Result<T, RuntimeError>
    where
        T: num_traits::FromBytes,
        T::Bytes: for<'a> TryFrom<&'a [u8]>,
    {
        let start_idx = (self.current_instruction as usize)
            .checked_add(offset as usize)
            .ok_or(RuntimeError::InstructionOOB)?;

        let end_idx = start_idx
            .checked_add(size_of::<T>())
            .ok_or(RuntimeError::InstructionOOB)?;

        let bytes = self
            .code
            .get(start_idx..end_idx)
            .ok_or(RuntimeError::InstructionOOB)?;

        let array: T::Bytes = bytes.try_into().map_err(|_| RuntimeError::InstructionOOB)?;

        Ok(T::from_be_bytes(&array))
    }

    pub fn cycle(&mut self) -> Result<(), RuntimeError> {
        let pc = self.current_instruction as usize;

        if pc >= self.code.len() {
            return Err(RuntimeError::InstructionOOB);
        }

        let op_code = self.code[pc];
        let op = match Operation::from_u8(op_code) {
            Some(o) => Ok(o),
            None => Err(RuntimeError::InvalidOPCode(op_code)),
        }?;
        let arg_type = op.arg_type();

        let end = std::cmp::min(pc + 5, self.code.len());
        debug!(
            "PC: {pc:02x} | Op: {:-13} | Args: {:-20} | Next 6 bytes {:02x?}",
            format!("{:?}", op),
            format!("{:?}", arg_type),
            &self.code[pc..end]
        );

        let new_addr: Option<u16> = match op {
            Operation::HALT => return Err(RuntimeError::Halted),
            Operation::NOP => None,
            Operation::MOV => {
                let r1 = self.next_reg(1)?;
                let r2 = self.next_reg(2)?;

                // 1 <- 2
                self.registers[r1] = self.registers[r2];
                None
            }
            Operation::MOV_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;

                self.registers[r1] = imm2;
                None
            }
            Operation::LOAD => {
                let r1 = self.next_reg(1)?;
                let r2 = self.next_reg(2)?;

                let address = self.registers[r2] as usize;
                self.registers[r1] = self.memory[address];
                None
            }

            Operation::LOAD_REL => {
                let r1 = self.next_reg(1)?;

                let offset = i16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 4) as usize]
                        .try_into()
                        .unwrap(),
                );

                let r3 = self.next_reg(4)?;

                let base_addr = self.registers[r1] as i32;
                let final_addr = base_addr
                    .checked_add(offset as i32)
                    .ok_or(RuntimeError::OffsetOOB)?;

                self.registers[r3 as usize] = self.memory[final_addr as usize];
                None
            }

            Operation::LOAD_IMM => {
                let addr_imm: u16 = self.next(1)?;

                let r2 = self.next_reg(2)?;

                self.registers[r2] = self.memory[addr_imm as usize];
                None
            }
            Operation::STORE_R_R => {
                let r1 = self.next_reg(1)?;
                let r2 = self.next_reg(2)?;

                self.memory[self.registers[r1] as usize] = self.registers[r2];
                None
            }
            Operation::STORE_REL_R => {
                let r1 = self.next_reg(1)?;

                let offset: i16 = self.next(2)?;

                let r3 = self.next_reg(4)?;

                let base_addr = self.registers[r1] as i32;
                let final_addr = base_addr
                    .checked_add(offset as i32)
                    .ok_or(RuntimeError::OffsetOOB)?;

                self.memory[final_addr as usize] = self.registers[r3 as usize];
                None
            }
            Operation::STORE_IMM_R => {
                let addr_imm: u16 = self.next(1)?;

                let r2 = self.next_reg(2)?;

                self.memory[addr_imm as usize] = self.registers[r2];
                None
            }
            Operation::STORE_R_IMM => {
                let r1 = self.next_reg(1)?;

                let imm_value = u16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 4) as usize]
                        .try_into()
                        .unwrap(),
                );
                self.memory[self.registers[r1] as usize] = imm_value;
                None
            }
            Operation::STORE_REL_IMM => {
                let r1 = self.next_reg(1)?;

                let offset = i16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 4) as usize]
                        .try_into()
                        .unwrap(),
                );
                let imm_value = u16::from_be_bytes(
                    self.code[(self.current_instruction + 5) as usize
                        ..(self.current_instruction + 7) as usize]
                        .try_into()
                        .unwrap(),
                );

                let base_addr = self.registers[r1] as i32;
                let final_addr = base_addr
                    .checked_add(offset as i32)
                    .ok_or(RuntimeError::OffsetOOB)? as usize;

                self.memory[final_addr] = imm_value;
                None
            }
            Operation::STORE_IMM_IMM => {
                let addr_imm: u16 = self.next(1)?;
                let value_imm = self.next(3)?;

                self.memory[addr_imm as usize] = value_imm;
                None
            }
            Operation::ADD => {
                let r1 = self.next_reg(1)?;

                let r2 = self.next_reg(2)?;

                self.registers[r1] = self.registers[r1].wrapping_add(self.registers[r2]);
                None
            }
            Operation::ADD_IMM => {
                let r1 = self.next_reg(1)?;

                let imm2: u16 = self.next(2)?;

                self.registers[r1] = self.registers[r1].wrapping_add(imm2);
                None
            }
            Operation::SUB => {
                let r1 = self.next_reg(1)?;

                let r2 = self.next_reg(2)?;

                self.registers[r1] = self.registers[r1].wrapping_sub(self.registers[r2]);
                None
            }
            Operation::SUB_IMM => {
                let r1 = self.next_reg(1)?;

                let imm2: u16 = self.next(2)?;

                self.registers[r1] = self.registers[r1].wrapping_sub(imm2);
                None
            }
            Operation::AND => {
                let r1 = self.next_reg(1)?;

                let r2 = self.next_reg(2)?;

                self.registers[r1] &= self.registers[r2];
                None
            }
            Operation::AND_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;
                self.registers[r1] &= imm2;
                None
            }
            Operation::OR => {
                let r1 = self.next_reg(1)?;

                let r2 = self.next_reg(2)?;

                self.registers[r1] |= self.registers[r2];
                None
            }
            Operation::OR_IMM => {
                let r1 = self.next_reg(1)?;

                let imm2: u16 = self.next(2)?;
                self.registers[r1] |= imm2;
                None
            }
            Operation::XOR => {
                let r1 = self.next_reg(1)?;

                let r2 = self.next_reg(2)?;

                self.registers[r1] ^= self.registers[r2];
                None
            }
            Operation::XOR_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;

                self.registers[r1] ^= imm2;
                None
            }
            Operation::SHL => {
                let r1 = self.next_reg(1)?;
                let r2 = self.next_reg(2)?;

                self.registers[r1] = self.registers[r1] << self.registers[r2];
                None
            }
            Operation::SHL_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;

                self.registers[r1] = self.registers[r1] << imm2;
                None
            }
            Operation::SHR => {
                let r1 = self.next_reg(1)?;
                let r2 = self.next_reg(2)?;

                self.registers[r1] = self.registers[r1] >> self.registers[r2];
                None
            }
            Operation::SHR_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;

                self.registers[r1] = self.registers[r1] >> imm2;
                None
            }
            Operation::CMP => {
                let r1 = self.next_reg(1)?;
                let r2 = self.next_reg(2)?;

                if self.registers[r1] == self.registers[r2] {
                    self.equal = true;
                } else {
                    self.equal = false;
                }
                None
            }
            Operation::CMP_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;

                if self.registers[r1] == imm2 {
                    self.equal = true;
                } else {
                    self.equal = false;
                }
                None
            }
            Operation::PUSH => {
                let r1 = self.next_reg(1)?;

                self.registers[Register::RSP as usize] = self.registers[Register::RSP as usize]
                    .checked_sub(1)
                    .ok_or(RuntimeError::StackOverflow)?;
                self.memory[self.registers[Register::RSP as usize] as usize] = self.registers[r1];

                None
            }
            Operation::PUSH_M => {
                let r1 = self.next_reg(1)?;

                self.registers[Register::RSP as usize] -= 1;
                self.memory[self.registers[Register::RSP as usize] as usize] =
                    self.memory[self.registers[r1] as usize];

                None
            }
            Operation::PUSH_IMM => {
                let val_imm = self.next(1)?;
                self.registers[Register::RSP as usize] -= 1;
                self.memory[self.registers[Register::RSP as usize] as usize] = val_imm;
                None
            }
            Operation::POP => {
                let r1 = self.next_reg(1)?;

                self.registers[r1] = self.memory[self.registers[Register::RSP as usize] as usize];
                self.registers[Register::RSP as usize] += 1;

                None
            }
            Operation::POP_M => {
                let r1 = self.next_reg(1)?;

                self.memory[self.registers[r1] as usize] =
                    self.memory[self.registers[Register::RSP as usize] as usize];
                self.registers[Register::RSP as usize] += 1;

                None
            }
            Operation::POP_IMM => {
                let addr_imm: u16 = self.next(1)?;

                self.memory[addr_imm as usize] =
                    self.memory[self.registers[Register::RSP as usize] as usize];
                self.registers[Register::RSP as usize] += 1;

                None
            }
            Operation::JMP => {
                let r1 = self.next_reg(1)?;

                Some(self.registers[r1])
            }
            Operation::JMP_IMM => {
                let addr_imm = self.next(1)?;
                Some(addr_imm)
            }
            Operation::JE => {
                let r1 = self.next_reg(1)?;

                if self.equal {
                    Some(self.registers[r1])
                } else {
                    None
                }
            }
            Operation::JE_IMM => {
                let addr_imm = self.next(1)?;
                if self.equal { Some(addr_imm) } else { None }
            }
            Operation::JNE => {
                let r1 = self.next_reg(1)?;

                if !self.equal {
                    Some(self.registers[r1])
                } else {
                    None
                }
            }
            Operation::JNE_IMM => {
                let addr_imm = self.next(1)?;
                if !self.equal { Some(addr_imm) } else { None }
            }
            Operation::CALL => {
                todo!();
            }
            Operation::CALL_IMM => todo!(),
            Operation::RET => todo!(),
        };

        if let Some(addr) = new_addr {
            // If JUMP instruction called
            self.current_instruction = addr;
        } else {
            let offset = arg_type.to_offset() as u16;
            self.current_instruction += offset;
        }

        Ok(())
    }

    pub fn from_bytes(code: Vec<u8>, registers: Vec<u16>) -> Self {
        VirtualMachine {
            code: code.into_iter().skip(3).collect(),
            equal: false,
            memory: vec![0; 0xFFFF],
            current_instruction: 0,
            registers,
        }
    }
}

#[derive(clap::Parser)]
struct Args {
    input_file: PathBuf,
}

fn main() {
    env_logger::builder()
        .format_timestamp(None)
        .parse_default_env()
        .init();

    let args = Args::parse();

    let code: Vec<u8> = std::fs::read(args.input_file).unwrap();
    let mut registers: Vec<u16> = Vec::with_capacity(17);

    for _ in 0..=17 {
        registers.push(0);
    }

    registers[Register::RSP as usize] = 0xFFFF;

    let mut vm = VirtualMachine::from_bytes(code, registers);
    loop {
        match vm.cycle() {
            Ok(_) => {}
            Err(e) => {
                match e {
                    RuntimeError::Halted => {
                        eprintln!("Program halted.");
                    }
                    _ => {
                        error!("RUNTIME ERROR: {e:02x?}")
                    }
                }
                break;
            }
        };
    }
    println!("{vm}");
}
