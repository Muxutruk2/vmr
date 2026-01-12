use std::{fmt::Display, path::Path, usize};
#[macro_use]
extern crate num_derive;
use bytemuck::Pod;
use log::{debug, error};
use num_traits::FromPrimitive;

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum Register {
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    R16,
    RSP,
}

pub type Immediate = u16;
pub type Offset = i16;

#[repr(u8)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
enum Operation {
    // Misc [0]
    HALT = 0x00,
    NOP = 0x01,

    // Move [1]
    MOV = 0x10,     // R, R
    MOV_IMM = 0x11, // R, IMM

    // Load/Store [2-3]
    // Load: Destination is always a Register
    LOAD = 0x20,     // [R], R
    LOAD_REL = 0x21, // [R + OFFSET], R
    LOAD_IMM = 0x22, // [IMM], R

    // Store: Register Source
    STORE_R_R = 0x30,   // [R], R
    STORE_REL_R = 0x31, // [R + OFFSET], R
    STORE_IMM_R = 0x32, // [IMM], R

    // Store: Immediate Source
    STORE_R_IMM = 0x33,   // [R], IMM
    STORE_REL_IMM = 0x34, // [R + OFFSET], IMM
    STORE_IMM_IMM = 0x35, // [IMM], IMM

    // Arithmetic & Logic [4-6]
    ADD = 0x40,     // R, R
    ADD_IMM = 0x41, // R, IMM
    SUB = 0x42,     // R, R
    SUB_IMM = 0x43, // R, IMM

    AND = 0x50,     // R, R
    AND_IMM = 0x51, // R, IMM
    OR = 0x52,      // R, R
    OR_IMM = 0x53,  // R, IMM
    XOR = 0x54,     // R, R
    XOR_IMM = 0x55, // R, IMM

    // Shifts [7]
    SHL = 0x70,     // R, R
    SHL_IMM = 0x71, // R, IMM
    SHR = 0x72,     // R, R
    SHR_IMM = 0x73, // R, IMM

    // Comparison [8]
    CMP = 0x80,     // R, R
    CMP_IMM = 0x81, // R, IMM

    // Stack Operations [9-A]
    PUSH = 0x90,     // R
    PUSH_M = 0x91,   // [R]
    PUSH_IMM = 0x92, // IMM

    POP = 0xA0,     // R
    POP_M = 0xA1,   // [R]
    POP_IMM = 0xA2, // IMM -- Pop into specific address

    // Control Flow [B-D]
    // Jumps (Address from Register or Immediate)
    JMP = 0xB0,
    JMP_IMM = 0xB1,

    // Conditionals: Zero / Equal
    JE = 0xB2,
    JE_IMM = 0xB3,

    // Conditionals: Not Zero / Not Equal
    JNE = 0xB6,
    JNE_IMM = 0xB7,

    // Subroutines
    CALL = 0xD0,     // R
    CALL_IMM = 0xD1, // IMM
    RET = 0xD2,      // ()
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Operation {
    #[allow(dead_code)]
    pub fn arg_type(&self) -> Arguments {
        match self {
            // Misc: 0 Operands
            Operation::HALT | Operation::NOP | Operation::RET => Arguments::None,

            // Single Operand: Register or Immediate
            Operation::PUSH
            | Operation::PUSH_M
            | Operation::PUSH_IMM
            | Operation::POP
            | Operation::POP_M
            | Operation::POP_IMM
            | Operation::JMP
            | Operation::JMP_IMM
            | Operation::JE
            | Operation::JE_IMM
            | Operation::JNE
            | Operation::JNE_IMM
            | Operation::CALL
            | Operation::CALL_IMM => Arguments::Reg,

            // Two Operands: Register, Register
            Operation::MOV
            | Operation::LOAD
            | Operation::STORE_R_R
            | Operation::ADD
            | Operation::SUB
            | Operation::AND
            | Operation::OR
            | Operation::XOR
            | Operation::SHL
            | Operation::SHR
            | Operation::CMP => Arguments::RegReg,

            // Two Operands: Register, Immediate
            Operation::MOV_IMM
            | Operation::STORE_R_IMM
            | Operation::ADD_IMM
            | Operation::SUB_IMM
            | Operation::AND_IMM
            | Operation::OR_IMM
            | Operation::XOR_IMM
            | Operation::SHL_IMM
            | Operation::SHR_IMM
            | Operation::CMP_IMM => Arguments::RegImm,

            // Two Operands: Immediate, Register
            Operation::LOAD_IMM | Operation::STORE_IMM_R => Arguments::ImmReg,

            // Two Operands: Immediate, Immediate
            Operation::STORE_IMM_IMM => Arguments::ImmImm,

            // Three Operands: Register, Immediate, Register (Relative Addressing)
            Operation::LOAD_REL | Operation::STORE_REL_R | Operation::STORE_REL_IMM => {
                Arguments::RegImmReg
            }
        }
    }
}

#[repr(u8)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arguments {
    None = 0x00,
    Reg = 0x01,
    RegReg = 0x02,
    RegImm = 0x03,
    ImmImm = 0x04,
    RegImmReg = 0x05,
    ImmReg = 0x06,
}

impl Arguments {
    fn to_offset(&self) -> u8 {
        match self {
            Arguments::None => 1,
            Arguments::Reg => 2,
            Arguments::RegReg => 3,
            Arguments::RegImm => 4,
            Arguments::ImmImm => 5,
            Arguments::RegImmReg => 5,
            Arguments::ImmReg => 4,
        }
    }
}

#[derive(Debug)]
struct VirtualMachine<'a> {
    code: Vec<u8>,
    memory: Vec<u16>,
    equal: bool,
    current_instruction: u16,
    registers: &'a mut [u16; 17],
}

impl<'a> std::fmt::Display for VirtualMachine<'a> {
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

impl<'a> VirtualMachine<'a> {
    fn next_reg(&self, offset: u16) -> Result<usize, RuntimeError> {
        let reg_byte = self.next(offset)?;
        Ok(Register::from_u8(reg_byte).ok_or(RuntimeError::InvalidRegister)? as usize)
    }

    fn next<T: Pod>(&self, offset: u16) -> Result<T, RuntimeError> {
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

        Ok(bytemuck::pod_read_unaligned(bytes))
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

        // 3. Increment PC (Advance to the NEXT instruction)
        let offset = arg_type.to_offset() as u16;
        self.current_instruction += offset;

        if let Some(addr) = new_addr {
            self.current_instruction = addr;
        }

        Ok(())
    }

    pub fn from_bytes(code: Vec<u8>, registers: &'a mut [u16; 17]) -> Self {
        VirtualMachine {
            code: code.into_iter().skip(3).collect(),
            equal: false,
            memory: vec![0; 0xFFFF],
            current_instruction: 0,
            registers,
        }
    }
}

fn main() {
    env_logger::builder()
        .format_timestamp(None)
        .parse_default_env()
        .init();

    let a: &Path = Path::new("test.vmr");
    let code: Vec<u8> = std::fs::read(a).unwrap();
    let mut registers: Vec<u16> = Vec::with_capacity(17);

    for _ in 0..=17 {
        registers.push(0);
    }
    registers[Register::RSP as usize] = 0xFFFF;

    let mut registers_slice: &mut [u16; 17] = &mut (registers[0..17].try_into().unwrap());
    let mut vm = VirtualMachine::from_bytes(code, &mut registers_slice);
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
