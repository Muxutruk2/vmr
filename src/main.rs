#[repr(u8)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    JZ = 0xB2,
    JZ_IMM = 0xB3,
    JE = 0xB4,
    JE_IMM = 0xB5,

    // Conditionals: Not Zero / Not Equal
    JNZ = 0xB6,
    JNZ_IMM = 0xB7,
    JNE = 0xB8,
    JNE_IMM = 0xB9,

    // Subroutines
    CALL = 0xD0,     // R
    CALL_IMM = 0xD1, // IMM
    RET = 0xD2,      // ()
}

#[repr(u8)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arguments {
    None = 0x00,
    Reg = 0x01,
    RegReg = 0x02,
    RegImm = 0x03,
    ImmReg = 0x04,
}

fn main() {
    println!("Hello, world!");
}
