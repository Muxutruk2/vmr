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
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum Operation {
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
            Operation::PUSH
            | Operation::PUSH_M
            | Operation::POP
            | Operation::POP_M
            | Operation::JMP
            | Operation::JE
            | Operation::JNE
            | Operation::CALL => Arguments::Reg,

            Operation::CALL_IMM
            | Operation::PUSH_IMM
            | Operation::POP_IMM
            | Operation::JMP_IMM
            | Operation::JNE_IMM
            | Operation::JE_IMM => Arguments::Imm,

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
