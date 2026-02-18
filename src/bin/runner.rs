#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::indexing_slicing)]
use clap::Parser;
use libvmr::{Operation, Register};
use log::{debug, error};
use num_traits::cast::FromPrimitive;
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

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
        writeln!(f, "CODE")?;
        for byte in self.code.iter() {
            write!(f, "{byte:02x} ")?;
        }
        writeln!(f, "\nInstruction Address: {:04x}", self.current_instruction)?;
        match self.code.get(self.current_instruction as usize) {
            Some(n) => {
                writeln!(f, "Instruction Byte: {:02x}", n)?;
            }
            None => {
                writeln!(f, "Instruction Byte: NOT_FOUND")?;
            }
        }
        writeln!(f, "Registers")?;
        for reg in self.registers.iter() {
            write!(f, "{:04x} ", reg)?;
        }
        if self.equal {
            writeln!(f, "Equal: Set")?;
        } else {
            writeln!(f, "Equal: Unset")?;
        }

        let rsp = self.get_reg(Register::RSP as usize).unwrap_or(0) as isize;
        let memory_len = self.memory.len() as isize;

        writeln!(f, "Stack Dump:")?;

        let range = -5..=5;

        // "0x0000 " 7 characters.
        // 5 blocks * 7 chars = 35 spaces of offset.
        let indent = " ".repeat(5 * 7);
        writeln!(f, "{} ↓RSP", indent)?;

        for offset in range {
            let target_idx = rsp + offset;

            if target_idx >= 0 && target_idx < memory_len {
                // Access valid memory
                let val = self.memory[target_idx as usize];
                write!(f, "{:#06x} ", val)?;
            } else {
                // Out of bounds placeholder
                write!(f, "  __   ")?;
            }
        }

        writeln!(f)?; // Final newline
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

    fn get_mem<T: Into<usize>>(&self, offset: T) -> Result<u16, RuntimeError> {
        self.memory
            .get(offset.into())
            .ok_or(RuntimeError::MemoryOOB)
            .copied()
    }

    fn get_mem_mut<T: Into<usize>>(&mut self, offset: T) -> Result<&mut u16, RuntimeError> {
        self.memory
            .get_mut(offset.into())
            .ok_or(RuntimeError::MemoryOOB)
    }

    fn get_reg<T: Into<usize>>(&self, register: T) -> Result<u16, RuntimeError> {
        self.registers
            .get(register.into())
            .ok_or(RuntimeError::InvalidRegister)
            .copied()
    }
    fn get_reg_mut<T: Into<usize>>(&mut self, register: T) -> Result<&mut u16, RuntimeError> {
        self.registers
            .get_mut(register.into())
            .ok_or(RuntimeError::InvalidRegister)
    }

    fn dump_mem_bin<T: Sized + Write>(&self, f: &mut BufWriter<T>) -> std::io::Result<()> {
        let data: &[u8] = bytemuck::cast_slice(&self.memory);
        f.write_all(data)?;
        Ok(())
    }

    fn dump_mem_readable<T: Sized + Write>(&self, f: &mut BufWriter<T>) -> std::io::Result<()> {
        let last_index = self.memory.iter().rposition(|&x| x != 0);

        match last_index {
            Some(index) => {
                for byte in self.memory.get(0..index).unwrap().iter() {
                    write!(f, "{byte:04x} ")?;
                }
            }
            None => write!(f, "<EMPTY> ")?,
        }

        Ok(())
    }

    pub fn cycle(&mut self) -> Result<(), RuntimeError> {
        let pc = self.current_instruction as usize;

        if pc >= self.code.len() {
            return Err(RuntimeError::InstructionOOB);
        }

        let op_code = *self.code.get(pc).ok_or(RuntimeError::InstructionOOB)?;
        let op = match Operation::from_u8(op_code) {
            Some(o) => Ok(o),
            None => Err(RuntimeError::InvalidOPCode(op_code)),
        }?;
        let arg_type = op.arg_type();

        let end = std::cmp::min(pc.wrapping_add(5), self.code.len());
        debug!(
            "PC: {pc:02x} | Op: {:-13} | Args: {:-20} | Next 6 bytes {:02x?}",
            format!("{:?}", op),
            format!("{:?}", arg_type),
            &self.code.get(pc..end).unwrap()
        );

        let new_addr: Option<u16> = match op {
            Operation::HALT => return Err(RuntimeError::Halted),
            Operation::NOP => None,
            Operation::MOV => {
                let r1 = self.next_reg(1)?;
                let r2 = self.next_reg(2)?;

                // 1 <- 2
                *self.get_reg_mut(r1)? = self.get_reg(r2)?;
                None
            }
            Operation::MOV_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;

                *self.get_reg_mut(r1)? = imm2;
                None
            }
            Operation::LOAD => {
                let r1 = self.next_reg(1)?;
                let r2 = self.next_reg(2)?;

                let address = self.get_reg(r2)?;
                *self.get_reg_mut(r1)? = self.get_mem(address)?;

                None
            }

            Operation::LOAD_REL => {
                let r1 = self.next_reg(1)?;

                let offset: i16 = self.next(2)?;

                let r3 = self.next_reg(4)?;

                let base_addr = self.get_reg(r1)? as i32;
                let final_addr = base_addr
                    .checked_add(offset as i32)
                    .ok_or(RuntimeError::OffsetOOB)? as usize;

                *self.get_reg_mut(r3)? = self.get_mem(final_addr)?;
                None
            }

            Operation::LOAD_IMM => {
                let addr_imm: u16 = self.next(1)?;

                let r2 = self.next_reg(2)?;

                *self.get_reg_mut(r2)? = self.get_mem(addr_imm)?;

                None
            }
            Operation::STORE_R_R => {
                let r1 = self.next_reg(1)?;

                let r2 = self.next_reg(2)?;

                *self.get_mem_mut(self.get_reg(r1)?)? = self.get_reg(r2)?;
                None
            }
            Operation::STORE_REL_R => {
                let r1 = self.next_reg(1)?;

                let offset: i16 = self.next(2)?;

                let r3 = self.next_reg(4)?;

                let base_addr = self.get_reg(r1)? as i32;
                let final_addr: usize = base_addr
                    .checked_add(offset as i32)
                    .ok_or(RuntimeError::OffsetOOB)?
                    as usize;

                *self.get_mem_mut(final_addr)? = self.get_reg(r3)?;
                None
            }
            Operation::STORE_IMM_R => {
                let addr_imm: u16 = self.next(1)?;

                let r2 = self.next_reg(2)?;

                *self.get_mem_mut(addr_imm)? = self.get_reg(r2)?;
                None
            }
            Operation::STORE_R_IMM => {
                let r1 = self.next_reg(1)?;

                let imm_value = self.next(2)?;

                *self.get_mem_mut(self.get_reg(r1)?)? = imm_value;

                None
            }
            Operation::STORE_REL_IMM => {
                let r1 = self.next_reg(1)?;

                let offset: i16 = self.next(2)?;
                let imm_value: u16 = self.next(5)?;

                let base_addr = self.get_reg(r1)? as i32;
                let final_addr = base_addr
                    .checked_add(offset as i32)
                    .ok_or(RuntimeError::OffsetOOB)? as usize;

                *self.get_mem_mut(final_addr)? = imm_value;
                None
            }
            Operation::STORE_IMM_IMM => {
                let addr_imm: u16 = self.next(1)?;
                let imm_value = self.next(3)?;

                *self.get_mem_mut(addr_imm)? = imm_value;
                None
            }
            Operation::ADD => {
                let r1 = self.next_reg(1)?;

                let r2 = self.next_reg(2)?;

                *self.get_reg_mut(r1)? = self.get_reg(r1)?.wrapping_add(self.get_reg(r2)?);
                None
            }
            Operation::ADD_IMM => {
                let r1 = self.next_reg(1)?;

                let imm2: u16 = self.next(2)?;

                *self.get_reg_mut(r1)? = self.get_reg(r1)?.wrapping_add(imm2);
                None
            }
            Operation::SUB => {
                let r1 = self.next_reg(1)?;

                let r2 = self.next_reg(2)?;

                *self.get_reg_mut(r1)? = self.get_reg(r1)?.wrapping_sub(self.get_reg(r2)?);
                None
            }
            Operation::SUB_IMM => {
                let r1 = self.next_reg(1)?;

                let imm2: u16 = self.next(2)?;

                *self.get_reg_mut(r1)? = self.get_reg(r1)?.wrapping_sub(imm2);

                None
            }
            Operation::AND => {
                let r1 = self.next_reg(1)?;

                let r2 = self.next_reg(2)?;

                *self.get_reg_mut(r1)? &= self.get_reg(r2)?;
                None
            }
            Operation::AND_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;
                *self.get_reg_mut(r1)? &= imm2;
                None
            }
            Operation::OR => {
                let r1 = self.next_reg(1)?;

                let r2 = self.next_reg(2)?;

                *self.get_reg_mut(r1)? |= self.get_reg(r2)?;
                None
            }
            Operation::OR_IMM => {
                let r1 = self.next_reg(1)?;

                let imm2: u16 = self.next(2)?;
                *self.get_reg_mut(r1)? |= imm2;
                None
            }
            Operation::XOR => {
                let r1 = self.next_reg(1)?;

                let r2 = self.next_reg(2)?;

                *self.get_reg_mut(r1)? ^= self.get_reg(r2)?;
                None
            }
            Operation::XOR_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;

                *self.get_reg_mut(r1)? ^= imm2;
                None
            }
            Operation::SHL => {
                let r1 = self.next_reg(1)?;
                let r2 = self.next_reg(2)?;

                *self.get_reg_mut(r1)? <<= self.get_reg(r2)?;
                None
            }
            Operation::SHL_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;

                *self.get_reg_mut(r1)? <<= imm2;
                None
            }
            Operation::SHR => {
                let r1 = self.next_reg(1)?;
                let r2 = self.next_reg(2)?;

                *self.get_reg_mut(r1)? >>= self.get_reg(r2)?;
                None
            }
            Operation::SHR_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;

                *self.get_reg_mut(r1)? >>= imm2;
                None
            }
            Operation::CMP => {
                let r1 = self.next_reg(1)?;
                let r2 = self.next_reg(2)?;

                self.equal = self.get_reg(r1)? == self.get_reg(r2)?;
                None
            }
            Operation::CMP_IMM => {
                let r1 = self.next_reg(1)?;
                let imm2: u16 = self.next(2)?;

                debug!("Comparing {} to {}", self.get_reg(r1)?, imm2);

                self.equal = self.get_reg(r1)? == imm2;
                None
            }
            Operation::PUSH => {
                let r1 = self.next_reg(1)?;
                let val = self.get_reg(r1)?;

                // Decrement Stack Pointer
                let new_rsp = self
                    .get_reg(Register::RSP as usize)?
                    .checked_sub(1)
                    .ok_or(RuntimeError::StackOverflow)?;
                *self.get_reg_mut(Register::RSP as usize)? = new_rsp;

                // 2. Store the REGISTER VALUE into memory at RSP
                *self.get_mem_mut(new_rsp)? = val;
                None
            }
            Operation::PUSH_M => {
                let r1 = self.next_reg(1)?;

                *self.get_reg_mut(Register::RSP as usize)? = self
                    .get_reg(Register::RSP as usize)?
                    .checked_sub(1)
                    .ok_or(RuntimeError::StackOverflow)?;

                *self.get_mem_mut(self.get_reg(Register::RSP as usize)?)? =
                    self.get_mem(self.get_reg(r1)?)?;

                None
            }
            Operation::PUSH_IMM => {
                let val_imm = self.next(1)?;

                *self.get_reg_mut(Register::RSP as usize)? = self
                    .get_reg(Register::RSP as usize)?
                    .checked_sub(1)
                    .ok_or(RuntimeError::StackOverflow)?;

                *self.get_mem_mut(self.get_reg(Register::RSP as usize)?)? = val_imm;
                None
            }
            Operation::POP => {
                let r1 = self.next_reg(1)?;

                debug!("POP: DESTINATION REGISTER: {r1}");
                debug!("CURRENT VALUE THERE: {:x}", self.get_reg(r1)?);
                debug!(
                    "POP: RSP POINTS TO {:x}",
                    self.get_reg(Register::RSP as usize)?
                );
                debug!(
                    "POP: RSP MEMORY {:x}",
                    self.get_mem(self.get_reg(Register::RSP as usize)?)?
                );

                *self.get_reg_mut(r1)? = self.get_mem(self.get_reg(Register::RSP as usize)?)?;

                *self.get_reg_mut(Register::RSP as usize)? = self
                    .get_reg(Register::RSP as usize)?
                    .checked_add(1)
                    .ok_or(RuntimeError::StackOverflow)?;

                debug!(
                    "POP: RSP NOW POINTS TO {:x}",
                    self.get_reg(Register::RSP as usize)?
                );

                None
            }
            Operation::POP_M => {
                let r1 = self.next_reg(1)?;

                *self.get_mem_mut(self.get_reg(r1)?)? =
                    self.get_mem(self.get_reg(Register::RSP as usize)?)?;

                *self.get_reg_mut(Register::RSP as usize)? = self
                    .get_reg(Register::RSP as usize)?
                    .checked_add(1)
                    .ok_or(RuntimeError::StackOverflow)?;

                None
            }
            Operation::POP_IMM => {
                let addr_imm: u16 = self.next(1)?;

                *self.get_mem_mut(addr_imm)? =
                    self.get_mem(self.get_reg(Register::RSP as usize)?)?;

                *self.get_reg_mut(Register::RSP as usize)? = self
                    .get_reg(Register::RSP as usize)?
                    .checked_add(1)
                    .ok_or(RuntimeError::StackOverflow)?;

                None
            }
            Operation::JMP => {
                let r1 = self.next_reg(1)?;

                Some(self.get_reg(r1)?)
            }
            Operation::JMP_IMM => {
                let addr_imm = self.next(1)?;
                Some(addr_imm)
            }
            Operation::JE => {
                let r1 = self.next_reg(1)?;

                if self.equal {
                    Some(self.get_reg(r1)?)
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
                    Some(self.get_reg(r1)?)
                } else {
                    None
                }
            }
            Operation::JNE_IMM => {
                let addr_imm = self.next(1)?;
                if !self.equal { Some(addr_imm) } else { None }
            }
            Operation::CALL => {
                let r1 = self.next_reg(1)?;

                let ret_addr = self.next_instruction_addr()?;
                *self.get_reg_mut(Register::RSP as usize)? = self
                    .get_reg(Register::RSP as usize)?
                    .checked_sub(1)
                    .ok_or(RuntimeError::StackOverflow)?;
                *self.get_mem_mut(self.get_reg(Register::RSP as usize)?)? = ret_addr;

                Some(self.get_reg(r1)?)
            }
            Operation::CALL_IMM => {
                let addr_imm = self.next(1)?;

                let ret_addr = self.next_instruction_addr()?;

                *self.get_reg_mut(Register::RSP as usize)? = self
                    .get_reg(Register::RSP as usize)?
                    .checked_sub(1)
                    .ok_or(RuntimeError::StackOverflow)?;

                *self.get_mem_mut(self.get_reg(Register::RSP as usize)?)? = ret_addr;

                Some(addr_imm)
            }
            Operation::RET => {
                let ret_addr = self.get_mem(self.get_reg(Register::RSP as usize)?)?;

                *self.get_reg_mut(Register::RSP as usize)? = self
                    .get_reg(Register::RSP as usize)?
                    .checked_add(1)
                    .ok_or(RuntimeError::StackOverflow)?;

                Some(ret_addr)
            }
        };

        if let Some(addr) = new_addr {
            // If JUMP instruction called
            self.current_instruction = addr;
        } else {
            self.current_instruction = self.next_instruction_addr()?;
        }

        debug!("{self}");

        Ok(())
    }

    fn next_instruction_addr(&self) -> Result<u16, RuntimeError> {
        let op_num = self
            .code
            .get(self.current_instruction as usize)
            .ok_or(RuntimeError::InstructionOOB)?;

        let curr_op = Operation::from_u8(*op_num).ok_or(RuntimeError::InvalidOPCode(*op_num))?;

        let arg_type = curr_op.arg_type();
        let offset = arg_type.to_offset() as u16;
        self.current_instruction
            .checked_add(offset)
            .ok_or(RuntimeError::InstructionOOB)
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
    #[arg(short)]
    dump: bool,
}

fn main() {
    env_logger::builder()
        .format_timestamp(None)
        .parse_default_env()
        .init();

    let args = Args::parse();

    let code: Vec<u8> = std::fs::read(args.input_file).unwrap();
    let mut registers: Vec<u16> = Vec::with_capacity(17);

    registers.extend(std::iter::repeat_n(0, 17));

    *registers.get_mut(Register::RSP as usize).unwrap() = 0xFFFF;

    let mut vm = VirtualMachine::from_bytes(code, registers);
    loop {
        match vm.cycle() {
            Ok(_) => {}
            Err(e) => {
                match e {
                    RuntimeError::Halted => {
                        debug!("{vm}");
                        debug!("Program halted.");
                    }
                    _ => {
                        error!("{vm}");
                        error!("RUNTIME ERROR: {e:02x?}")
                    }
                }
                break;
            }
        };
    }

    let milisecond = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time travel")
        .as_millis();

    if args.dump {
        if let Err(e) = fs::create_dir_all("./dumps") {
            error!("Failed to create dumps directory: {e}");
        } else {
            let readable_path_str = format!("./dumps/readable_{milisecond}.txt");
            let bin_path_str = format!("./dumps/raw_{milisecond}.bin");
            let readable_path = Path::new(&readable_path_str);
            let bin_path = Path::new(&bin_path_str);

            let try_create = || -> Result<(File, File), std::io::Error> {
                let f1 = File::create(readable_path)?;
                let f2 = File::create(bin_path)?;
                Ok((f1, f2))
            };

            match try_create() {
                Ok((readable_file, bin_file)) => {
                    let mut readable_writer = BufWriter::new(readable_file);
                    let mut bin_writer = BufWriter::new(bin_file);
                    vm.dump_mem_bin(&mut bin_writer)
                        .expect("Could not write binary dump");
                    vm.dump_mem_readable(&mut readable_writer)
                        .expect("Could not write binary dump");
                    eprintln!(
                        "Memory dumped at {} and {}",
                        readable_path.display(),
                        bin_path.display()
                    );
                }
                Err(e) => {
                    error!("Skipping dump: Could not create files: {e}");
                }
            }
        }
    }
}
