use flagset::flags;
use std::slice::IterMut;
use std::{collections::HashMap, fmt::Display};
use strum::EnumString;

use std::fmt::Debug;

/// A vector wrapper indexed by `u16`.
#[derive(Debug, Clone)]
pub struct IndexedVec<T> {
    data: Vec<T>,
}

#[derive(Debug)]
pub struct IndexedVecErr;

impl<T: 'static> IndexedVec<T> {
    /// Creates a new, empty `IndexedVec`.
    #[must_use]
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Creates a new `IndexedVec` with the specified capacity `u16`.
    #[must_use]
    pub fn with_capacity(capacity: u16) -> Self {
        let cap_usize = capacity as usize;

        Self {
            data: Vec::with_capacity(cap_usize),
        }
    }

    /// Returns the number of elements in the vector as a `u16`.
    ///
    /// # Panics
    /// When length is bigger than `u16::MAX`
    #[must_use]
    pub fn len(&self) -> u16 {
        self.data
            .len()
            .try_into()
            .expect("Length exceeded u16::MAX limit")
    }

    /// Returns `true` if the vector contains no elements.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Errors
    /// if `T` is `u16` and the limit (`u16::MAX - 1`) is exceeded.
    pub fn push(&mut self, value: T) -> Result<(), IndexedVecErr> {
        let target_len = self.data.len().checked_add(1).ok_or(IndexedVecErr)?;

        Self::assert_u16_bounds(target_len)?;
        self.data.push(value);
        Ok(())
    }

    /// Returns an iterator over the values.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }

    /// Returns a mutable iterator over the values.
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.data.iter_mut()
    }

    /// Returns the value at `index`
    #[must_use]
    pub fn get(&self, index: u16) -> Option<&T> {
        self.data.get(index as usize)
    }

    pub fn get_mut(&mut self, index: u16) -> Option<&mut T> {
        self.data.get_mut(index as usize)
    }

    /// # Errors
    ///
    /// When extending would overflow
    pub fn extend_from_slice(&mut self, slice: &[T]) -> Result<(), IndexedVecErr>
    where
        T: Clone,
    {
        Self::assert_u16_bounds(self.len() as usize + slice.len())?;
        self.data.extend_from_slice(slice);
        Ok(())
    }

    /// Helper to enforce the inner Vec limit when `T = u16`: maximum allowed size is `u16::MAX - 1`.
    const fn assert_u16_bounds(target_len: usize) -> Result<(), IndexedVecErr> {
        let max_allowed = (u16::MAX - 1) as usize;
        if target_len <= max_allowed {
            return Err(IndexedVecErr);
        }
        Ok(())
    }
}

impl<T: 'static> Default for IndexedVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

// IntoIterator implementation for owned iterations
impl<T> IntoIterator for IndexedVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

// Enables: for item in &indexed_vec
impl<'a, T> IntoIterator for &'a IndexedVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

// Enables: for item in &mut indexed_vec
impl<'a, T> IntoIterator for &'a mut IndexedVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter_mut()
    }
}

impl<T> TryFrom<Vec<T>> for IndexedVec<T> {
    type Error = ();

    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        if value.len() < u16::MAX.into() {
            return Err(());
        }
        Ok(Self { data: value })
    }
}

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
#[allow(non_camel_case_types, reason = "Opcodes are uppercase")]
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
    JNN = 0xB8, // Jump if Not Negative
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
    #[must_use]
    pub const fn arg_type(&self) -> Arguments {
        match self {
            // Misc: 0 Operands
            Self::HALT | Self::NOP | Self::RET | Self::SYSCALL => Arguments::None,

            // Single Operand: Register or Immediate
            Self::DIV_MOD | Self::PUSH | Self::PUSH_M | Self::POP | Self::POP_M => Arguments::Reg,

            Self::PUSH_IMM
            | Self::POP_IMM
            | Self::JMP
            | Self::JZ
            | Self::JNZ
            | Self::JA
            | Self::JB
            | Self::JAE
            | Self::JBE
            | Self::JN
            | Self::JNN
            | Self::JO
            | Self::JNO
            | Self::JC
            | Self::JNC
            | Self::CALL => Arguments::Imm,

            // Two Operands: Register, Register
            Self::MOV
            | Self::LOAD
            | Self::STORE_R_R
            | Self::ADD
            | Self::SUB
            | Self::AND
            | Self::OR
            | Self::XOR
            | Self::SHL
            | Self::SHR
            | Self::CMP => Arguments::RegReg,

            // Two Operands: Register, Immediate
            Self::MOV_IMM
            | Self::STORE_R_IMM
            | Self::ADD_IMM
            | Self::SUB_IMM
            | Self::AND_IMM
            | Self::OR_IMM
            | Self::XOR_IMM
            | Self::SHL_IMM
            | Self::SHR_IMM
            | Self::CMP_IMM => Arguments::RegImm,

            // Two Operands: Immediate, Register
            Self::LOAD_IMM | Self::STORE_IMM_R => Arguments::ImmReg,

            // Two Operands: Immediate, Immediate
            Self::STORE_IMM_IMM => Arguments::ImmImm,

            // Three Operands: Register, Immediate, Register (Relative Addressing)
            Self::LOAD_REL | Self::STORE_REL_R => Arguments::RegImmReg,

            Self::STORE_REL_IMM => Arguments::RegImmImm,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arguments {
    None = 0x00,
    Reg = 0x01,
    RegReg = 0x02,
    RegImm = 0x03,
    ImmImm = 0x04,
    RegImmReg = 0x05,
    ImmReg = 0x06,
    Imm = 0x07,
    RegImmImm = 0x08,
}

impl Arguments {
    #[must_use]
    pub const fn to_offset(&self) -> u8 {
        match self {
            Self::None => 1,
            Self::Reg => 2,
            Self::Imm | Self::RegReg => 3,
            Self::RegImm | Self::ImmReg => 4,
            Self::ImmImm | Self::RegImmReg => 5,
            Self::RegImmImm => 6,
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
    pub internal_relocations: IndexedVec<u16>,
    pub external_relocations: IndexedVec<(String, u16)>,
    pub labels: HashMap<String, u16>,
    pub exports: IndexedVec<(String, u16)>,
    pub bytecode: IndexedVec<u8>,
    pub base_address: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ObjectParseErr {
    InvalidMagic,
    UnexpectedEof,
    StringParseError,
    ExportsOverflow,
    ExternalRelocationsOverflow,
    InternalRelocationsOverflow,
    BytecodeTooLarge,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StringParseError {
    UTF8Error(std::str::Utf8Error),
    BadData,
}

impl ObjectFile {
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            name,
            internal_relocations: IndexedVec::new(),
            external_relocations: IndexedVec::new(),
            exports: IndexedVec::new(),
            labels: HashMap::new(),
            bytecode: IndexedVec::new(),
            base_address: 0,
        }
    }

    #[must_use]
    pub fn to_binary(&self) -> Vec<u8> {
        let mut out = Vec::new();

        out.extend_from_slice(b"vmro");

        out.extend_from_slice(&self.exports.len().to_be_bytes());

        for (name, offset) in &self.exports {
            Self::write_string(&mut out, name);
            out.extend_from_slice(&offset.to_be_bytes());
        }

        out.extend_from_slice(&self.external_relocations.len().to_be_bytes());
        for (label, address) in &self.external_relocations {
            Self::write_string(&mut out, label);
            out.extend_from_slice(&address.to_be_bytes());
        }

        out.extend_from_slice(&self.internal_relocations.len().to_be_bytes());
        for address in &self.internal_relocations {
            out.extend_from_slice(&address.to_be_bytes());
        }

        out.extend_from_slice(&self.bytecode.data[..]);

        out
    }

    /// Parse object file
    ///
    /// # Errors
    ///
    /// With incorrect format
    pub fn from_binary(data: &[u8], name: &str, base_address: u16) -> Result<Self, ObjectParseErr> {
        if data.get(0..4) != Some(b"vmro") {
            return Err(ObjectParseErr::InvalidMagic);
        }
        let mut cursor = 4;

        let read_u16 = |data: &[u8], cursor: &mut usize| -> Result<u16, ObjectParseErr> {
            let bytes = data
                .get(*cursor..*cursor + 2)
                .ok_or(ObjectParseErr::UnexpectedEof)?;
            *cursor += 2;
            Ok(u16::from_be_bytes([
                *bytes.first().ok_or(ObjectParseErr::UnexpectedEof)?,
                *bytes.get(1).ok_or(ObjectParseErr::UnexpectedEof)?,
            ]))
        };

        let export_count = read_u16(data, &mut cursor)?;
        let mut exports = IndexedVec::with_capacity(export_count);
        for _ in 0..export_count {
            let slice = data.get(cursor..).ok_or(ObjectParseErr::UnexpectedEof)?;
            let (label, bytes_read) =
                Self::read_string(slice).map_err(|_err| ObjectParseErr::StringParseError)?;
            cursor += bytes_read;

            let offset = read_u16(data, &mut cursor)?;
            exports
                .push((label.clone(), offset))
                .map_err(|_err| ObjectParseErr::ExportsOverflow)?;
        }

        let external_relocation_count = read_u16(data, &mut cursor)?;
        let mut external_relocations = IndexedVec::with_capacity(external_relocation_count);
        for _ in 0..external_relocation_count {
            let slice = data.get(cursor..).ok_or(ObjectParseErr::UnexpectedEof)?;
            let (label, bytes_read) =
                Self::read_string(slice).map_err(|_err| ObjectParseErr::StringParseError)?;
            cursor += bytes_read;

            let offset = read_u16(data, &mut cursor)?;
            external_relocations
                .push((label.clone(), offset))
                .map_err(|_err| ObjectParseErr::ExternalRelocationsOverflow)?;
        }

        let internal_relocation_count = read_u16(data, &mut cursor)?;
        let mut internal_relocations = IndexedVec::with_capacity(internal_relocation_count);
        for _ in 0..internal_relocation_count {
            let offset = read_u16(data, &mut cursor)?;
            internal_relocations
                .push(offset)
                .map_err(|_err| ObjectParseErr::InternalRelocationsOverflow)?;
        }

        let bytecode_vec = data
            .get(cursor..)
            .ok_or(ObjectParseErr::UnexpectedEof)?
            .to_vec();
        let bytecode = bytecode_vec
            .try_into()
            .map_err(|_err| ObjectParseErr::BytecodeTooLarge)?;

        Ok(Self {
            name: name.to_string(),
            exports,
            labels: HashMap::new(),
            bytecode,
            internal_relocations,
            external_relocations,
            base_address,
        })
    }

    fn write_string(buffer: &mut Vec<u8>, s: &str) {
        buffer.extend_from_slice(
            &u16::try_from(s.len())
                .expect("String too long")
                .to_be_bytes(),
        );
        buffer.extend_from_slice(s.as_bytes());
    }
    fn read_string(data: &[u8]) -> Result<(String, usize), StringParseError> {
        let len = u16::from_be_bytes([
            *data.first().expect("Could not get string length"),
            *data.get(1).expect("Could not get string length"),
        ]) as usize;
        let raw = data.get(2..2 + len).ok_or(StringParseError::BadData)?;
        let s = String::from_utf8(raw.to_vec())
            .map_err(|e| StringParseError::UTF8Error(e.utf8_error()))?;
        Ok((s, len + 2))
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
