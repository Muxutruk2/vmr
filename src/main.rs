use std::{path::Path, usize};
#[macro_use]
extern crate num_derive;
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
                    write!(f, "{byte:02x} ")?;
                }
            }
            None => write!(f, "<EMPTY> ")?,
        }
        write!(
            f,
            "\nInstruction Address: {:02x}\n",
            self.current_instruction
        )?;
        write!(
            f,
            "Instruction Byte: {:02x}\n",
            self.code[self.current_instruction as usize]
        )?;
        write!(f, "Registers\n")?;
        for reg in self.registers.iter() {
            write!(f, "{:02x} ", reg)?;
        }
        if self.equal {
            write!(f, "Equal: Set\n")?;
        } else {
            write!(f, "Equal: Unset\n")?;
        }

        Ok(())
    }
}

pub enum RuntimeError {
    InstructionOOB,
    MemoryOOB,
    InvalidOPCode,
    InvalidRegister,
    OffsetOOB,
    Halted,
}

impl<'a> VirtualMachine<'a> {
    pub fn cycle(&mut self) -> Result<(), RuntimeError> {
        let pc = self.current_instruction as usize;

        if pc >= self.code.len() {
            return Err(RuntimeError::InstructionOOB);
        }

        let op_code = self.code[pc];
        let op = match Operation::from_u8(op_code) {
            Some(o) => Ok(o),
            None => Err(RuntimeError::InvalidOPCode),
        }?;
        let arg_type = op.arg_type();

        // 2. Logging
        println!("PC: {}, Op: {:?}, Args: {:?}", pc, op, arg_type);

        let new_addr: Option<u16> = match op {
            Operation::HALT => return Err(RuntimeError::Halted),
            Operation::NOP => None,
            Operation::MOV => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let r2 = Register::from_u8(self.code[(self.current_instruction + 2) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                // 1 <- 2
                self.registers[r1 as usize] = self.registers[r2 as usize];
                None
            }
            Operation::MOV_IMM => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let imm2 = u16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );
                self.registers[r1 as usize] = imm2;
                None
            }
            Operation::LOAD => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let r2 = Register::from_u8(self.code[(self.current_instruction + 2) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                let address = self.registers[r2 as usize] as usize;
                self.registers[r1 as usize] = self.memory[address];
                None
            }

            Operation::LOAD_REL => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                let offset = i16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 4) as usize]
                        .try_into()
                        .unwrap(),
                );

                let r3 = Register::from_u8(self.code[(self.current_instruction + 4) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                let base_addr = self.registers[r1 as usize] as i32;
                let final_addr = base_addr
                    .checked_add(offset as i32)
                    .ok_or(RuntimeError::OffsetOOB)?;

                self.registers[r3 as usize] = self.memory[final_addr as usize];
                None
            }

            Operation::LOAD_IMM => {
                let addr_imm = u16::from_be_bytes(
                    self.code[(self.current_instruction + 1) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );

                let r2 = Register::from_u8(self.code[(self.current_instruction + 3) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                self.registers[r2 as usize] = self.memory[addr_imm as usize];
                None
            }
            Operation::STORE_R_R => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let r2 = Register::from_u8(self.code[(self.current_instruction + 2) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                self.memory[self.registers[r1 as usize] as usize] = self.registers[r2 as usize];
                None
            }
            Operation::STORE_REL_R => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                let offset = i16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 4) as usize]
                        .try_into()
                        .unwrap(),
                );

                let r3 = Register::from_u8(self.code[(self.current_instruction + 4) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                let base_addr = self.registers[r1 as usize] as i32;
                let final_addr = base_addr
                    .checked_add(offset as i32)
                    .ok_or(RuntimeError::OffsetOOB)?;

                self.memory[final_addr as usize] = self.registers[r3 as usize];
                None
            }
            Operation::STORE_IMM_R => {
                let addr_imm = u16::from_be_bytes(
                    self.code[(self.current_instruction + 1) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );

                let r2 = Register::from_u8(self.code[(self.current_instruction + 3) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                self.memory[addr_imm as usize] = self.registers[r2 as usize];
                None
            }
            Operation::STORE_R_IMM => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let imm_value = u16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 4) as usize]
                        .try_into()
                        .unwrap(),
                );
                self.memory[self.registers[r1 as usize] as usize] = imm_value;
                None
            }
            Operation::STORE_REL_IMM => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

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

                let base_addr = self.registers[r1 as usize] as i32;
                let final_addr = base_addr
                    .checked_add(offset as i32)
                    .ok_or(RuntimeError::OffsetOOB)? as usize;

                self.memory[final_addr] = imm_value;
                None
            }
            Operation::STORE_IMM_IMM => {
                let addr_imm = u16::from_be_bytes(
                    self.code[(self.current_instruction + 1) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );
                let value_imm = u16::from_be_bytes(
                    self.code[(self.current_instruction + 4) as usize
                        ..(self.current_instruction + 6) as usize]
                        .try_into()
                        .unwrap(),
                );

                self.memory[addr_imm as usize] = value_imm;
                None
            }
            Operation::ADD => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let r2 = Register::from_u8(self.code[(self.current_instruction + 2) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                self.registers[r1 as usize] =
                    self.registers[r1 as usize].wrapping_add(self.registers[r2 as usize]);
                None
            }
            Operation::ADD_IMM => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let imm2 = u16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 4) as usize]
                        .try_into()
                        .unwrap(),
                );

                self.registers[r1 as usize] = self.registers[r1 as usize].wrapping_add(imm2);
                None
            }
            Operation::SUB => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let r2 = Register::from_u8(self.code[(self.current_instruction + 2) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                self.registers[r1 as usize] =
                    self.registers[r1 as usize].wrapping_sub(self.registers[r2 as usize]);
                None
            }
            Operation::SUB_IMM => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let imm2 = u16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );

                self.registers[r1 as usize] = self.registers[r1 as usize].wrapping_sub(imm2);
                None
            }
            Operation::AND => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let r2 = Register::from_u8(self.code[(self.current_instruction + 2) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                self.registers[r1 as usize] &= self.registers[r2 as usize];
                None
            }
            Operation::AND_IMM => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let imm2 = u16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );
                self.registers[r1 as usize] &= imm2;
                None
            }
            Operation::OR => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let r2 = Register::from_u8(self.code[(self.current_instruction + 2) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                self.registers[r1 as usize] |= self.registers[r2 as usize];
                None
            }
            Operation::OR_IMM => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let imm2 = u16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );
                self.registers[r1 as usize] |= imm2;
                None
            }
            Operation::XOR => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let r2 = Register::from_u8(self.code[(self.current_instruction + 2) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                self.registers[r1 as usize] ^= self.registers[r2 as usize];
                None
            }
            Operation::XOR_IMM => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let imm2 = u16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );
                self.registers[r1 as usize] ^= imm2;
                None
            }
            Operation::SHL => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let r2 = Register::from_u8(self.code[(self.current_instruction + 2) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                self.registers[r1 as usize] =
                    self.registers[r1 as usize] << self.registers[r2 as usize];
                None
            }
            Operation::SHL_IMM => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let imm2 = u16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 4) as usize]
                        .try_into()
                        .unwrap(),
                );
                self.registers[r1 as usize] = self.registers[r1 as usize] << imm2;
                None
            }
            Operation::SHR => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let r2 = Register::from_u8(self.code[(self.current_instruction + 2) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                self.registers[r1 as usize] =
                    self.registers[r1 as usize] >> self.registers[r2 as usize];
                None
            }
            Operation::SHR_IMM => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let imm2 = u16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );
                self.registers[r1 as usize] = self.registers[r1 as usize] >> imm2;
                None
            }
            Operation::CMP => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let r2 = Register::from_u8(self.code[(self.current_instruction + 2) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                if self.registers[r1 as usize] == self.registers[r2 as usize] {
                    self.equal = true;
                } else {
                    self.equal = false;
                }
                None
            }
            Operation::CMP_IMM => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                let imm2 = u16::from_be_bytes(
                    self.code[(self.current_instruction + 2) as usize
                        ..(self.current_instruction + 4) as usize]
                        .try_into()
                        .unwrap(),
                );

                if self.registers[r1 as usize] == imm2 {
                    self.equal = true;
                } else {
                    self.equal = false;
                }
                None
            }
            Operation::PUSH => todo!(),
            Operation::PUSH_M => todo!(),
            Operation::PUSH_IMM => todo!(),
            Operation::POP => todo!(),
            Operation::POP_M => todo!(),
            Operation::POP_IMM => todo!(),
            Operation::JMP => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;
                Some(self.registers[r1 as usize])
            }
            Operation::JMP_IMM => {
                let addr_imm = u16::from_be_bytes(
                    self.code[(self.current_instruction + 1) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );
                Some(addr_imm)
            }
            Operation::JE => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                if self.equal {
                    Some(self.registers[r1 as usize])
                } else {
                    None
                }
            }
            Operation::JE_IMM => {
                let addr_imm = u16::from_be_bytes(
                    self.code[(self.current_instruction + 1) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );
                if self.equal { Some(addr_imm) } else { None }
            }
            Operation::JNE => {
                let r1 = Register::from_u8(self.code[(self.current_instruction + 1) as usize])
                    .ok_or(RuntimeError::InvalidRegister)?;

                if !self.equal {
                    Some(self.registers[r1 as usize])
                } else {
                    None
                }
            }
            Operation::JNE_IMM => {
                let addr_imm = u16::from_be_bytes(
                    self.code[(self.current_instruction + 1) as usize
                        ..(self.current_instruction + 3) as usize]
                        .try_into()
                        .unwrap(),
                );
                if !self.equal { Some(addr_imm) } else { None }
            }
            Operation::CALL => {
                todo!();
            }
            Operation::CALL_IMM => todo!(),
            Operation::RET => todo!(),
        };

        // 3. Increment PC (Advance to the NEXT instruction)
        // Note: Do this BEFORE execution if instructions are fixed-width,
        // or let execution modify it if it's a Jump.
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
    let a: &Path = Path::new("test.vmr");
    let code: Vec<u8> = std::fs::read(a).unwrap();
    let mut registers: Vec<u16> = Vec::with_capacity(17);
    unsafe {
        registers.set_len(17);
    }
    registers.fill(0);
    let mut registers_slice: &mut [u16; 17] = &mut (registers[0..17].try_into().unwrap());
    let mut vm = VirtualMachine::from_bytes(code, &mut registers_slice);
    while vm.cycle().is_ok() {}
    println!("{vm}");
}
