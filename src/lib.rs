use flagset::flags;
use std::{collections::HashMap, fmt::Display};
use strum::EnumString;

#[macro_use]
extern crate num_derive;

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum Register {
    R0,
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
    RSP,
}

pub type Immediate = u16;
pub type Offset = i16;

#[repr(u8)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(EnumString, Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum Operation {
    // Misc [0]
    HALT = 0x00,
    NOP = 0x01,

    // Move [1]
    MOV = 0x10,     // R, R
    MOV_IMM = 0x11, // R, IMM

    // Load/Store [2-3]
    // Load: Destination is always a Register
    LOAD = 0x20,     // R, [R]
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

    DIV_MOD = 0x60, // R1 / R -> R0 % R1

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
    // Jumps (Address from Immediate)
    JMP = 0xB0, // Unconditional
    JZ = 0xB1,  // Jump if Zero
    JNZ = 0xB2, // Jump if not Zero
    JA = 0xB3,  // Jump if Above
    JB = 0xB4,  // Jump if Below
    JAE = 0xB5, // Jump if Above or Equal
    JBE = 0xB6, // Jump if Below or Equal
    JN = 0xB7,  // Jump if Negative
    JNP = 0xB8, // Jump if Not Negative
    JO = 0xB9,  // Jump if Overflow
    JNO = 0xBA, // Jump if Not Overflow
    JC = 0xBB,  // Jump if Carry
    JNC = 0xBC, // Jump if Not Carry

    // Subroutines
    CALL = 0xD0, // Call function at immediate address
    RET = 0xD2,  // ()
    //
    SYSCALL = 0xFF,
}

impl Operation {
    #[allow(dead_code)]
    pub fn arg_type(&self) -> Arguments {
        match self {
            // Misc: 0 Operands
            Operation::HALT | Operation::NOP | Operation::RET | Operation::SYSCALL => {
                Arguments::None
            }

            // Single Operand: Register or Immediate
            Operation::DIV_MOD
            | Operation::PUSH
            | Operation::PUSH_M
            | Operation::POP
            | Operation::POP_M => Arguments::Reg,

            Operation::PUSH_IMM
            | Operation::POP_IMM
            | Operation::JMP
            | Operation::JZ
            | Operation::JNZ
            | Operation::JA
            | Operation::JB
            | Operation::JAE
            | Operation::JBE
            | Operation::JN
            | Operation::JNP
            | Operation::JO
            | Operation::JNO
            | Operation::JC
            | Operation::JNC
            | Operation::CALL => Arguments::Imm,

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
pub enum Arguments {
    None = 0x00,
    Reg = 0x01,
    RegReg = 0x02,
    RegImm = 0x03,
    ImmImm = 0x04,
    RegImmReg = 0x05,
    ImmReg = 0x06,
    Imm,
}

impl Arguments {
    pub fn to_offset(&self) -> u8 {
        match self {
            Arguments::None => 1,
            Arguments::Reg => 2,
            Arguments::Imm => 3,
            Arguments::RegReg => 3,
            Arguments::RegImm => 4,
            Arguments::ImmImm => 5,
            Arguments::RegImmReg => 5,
            Arguments::ImmReg => 4,
        }
    }
}

flags! {
    pub enum Flags: u16 {
        Equals = 0b0001,
        Negative = 0b0010,
        Overflow = 0b0100,
        Carry = 0b1000,
    }
}

#[derive(Debug)]
pub struct ObjectFile {
    pub name: String,
    pub internal_relocations: Vec<u16>,
    pub external_relocations: Vec<(String, u16)>,
    pub labels: HashMap<String, u16>,
    pub exports: Vec<(String, u16)>,
    pub bytecode: Vec<u8>,
    pub base_address: u16,
}

impl ObjectFile {
    pub fn new(name: String) -> Self {
        Self {
            name,
            internal_relocations: Vec::new(),
            external_relocations: Vec::new(),
            exports: Vec::new(),
            labels: HashMap::new(),
            bytecode: Vec::new(),
            base_address: 0,
        }
    }

    pub fn to_binary(&self) -> Vec<u8> {
        let mut out = Vec::new();

        out.extend_from_slice(b"vmro");

        out.extend_from_slice(&(self.exports.len() as u16).to_be_bytes());

        for (name, offset) in &self.exports {
            Self::write_string(&mut out, name);
            out.extend_from_slice(&offset.to_be_bytes());
        }

        out.extend_from_slice(&(self.external_relocations.len() as u16).to_be_bytes());
        for (label, address) in &self.external_relocations {
            Self::write_string(&mut out, label);
            out.extend_from_slice(&address.to_be_bytes());
        }

        out.extend_from_slice(&(self.internal_relocations.len() as u16).to_be_bytes());
        for address in &self.internal_relocations {
            out.extend_from_slice(&address.to_be_bytes());
        }

        out.extend_from_slice(&self.bytecode);

        out
    }

    pub fn from_binary(data: &[u8], name: &str, base_address: u16) -> Self {
        let mut cursor = 4; // Skip "vmro"

        // Parse Exports
        let export_count = u16::from_be_bytes([data[cursor], data[cursor + 1]]);
        let mut exports = Vec::with_capacity(export_count as usize);
        cursor += 2;
        for _ in 0..export_count {
            let (label, bytes_read) = Self::read_string(&data[cursor..]);
            cursor += bytes_read;
            let offset = u16::from_be_bytes([data[cursor], data[cursor + 1]]);
            cursor += 2;

            exports.push((label.clone(), offset));
        }

        // External Relocations
        let external_relocation_count = u16::from_be_bytes([data[cursor], data[cursor + 1]]);
        let mut external_relocations = Vec::with_capacity(external_relocation_count as usize);
        cursor += 2;
        for _ in 0..external_relocation_count {
            let (label, bytes_read) = Self::read_string(&data[cursor..]);
            cursor += bytes_read;
            let offset = u16::from_be_bytes([data[cursor], data[cursor + 1]]);
            cursor += 2;

            external_relocations.push((label.clone(), offset));
        }

        // Local Relocations
        let internal_relocation_count = u16::from_be_bytes([data[cursor], data[cursor + 1]]);
        cursor += 2;
        let mut internal_relocations: Vec<u16> =
            Vec::with_capacity(internal_relocation_count as usize);
        for _ in 0..internal_relocation_count {
            let offset = u16::from_be_bytes([data[cursor], data[cursor + 1]]);
            cursor += 2;
            internal_relocations.push(offset);
        }

        let bytecode = data[cursor..].to_vec();

        ObjectFile {
            name: name.to_string(),
            exports,
            labels: HashMap::new(),
            bytecode,
            internal_relocations,
            external_relocations,
            base_address,
        }
    }

    fn write_string(buffer: &mut Vec<u8>, s: &str) {
        buffer.extend_from_slice(&(s.len() as u16).to_be_bytes());
        buffer.extend_from_slice(s.as_bytes());
    }
    fn read_string(data: &[u8]) -> (String, usize) {
        let len = u16::from_be_bytes([data[0], data[1]]) as usize;
        let s = String::from_utf8_lossy(&data[2..2 + len]).to_string();
        (s, len + 2)
    }
}

impl Display for ObjectFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{{")?;
        writeln!(f, "\t\"name\": \"{}\",", self.name)?;
        writeln!(f, "\t\"exports\": {:02x?},", self.exports)?;
        writeln!(f, "\t\"internal\": {:02x?},", self.internal_relocations)?;
        writeln!(f, "\t\"external\": {:02x?},", self.external_relocations)?;
        writeln!(f, "\t\"labels\": {:02x?},", self.labels)?;
        writeln!(f, "\t\"base_address\": {:02x?}", self.base_address)?;
        // write!(f, "\t\"bytecode\": \"")?;
        // for byte in self.bytecode.iter() {
        //     write!(f, "{byte:02x} ")?;
        // }
        // writeln!(f, "\"")?;
        writeln!(f, "}}")?;
        Ok(())
    }
}
