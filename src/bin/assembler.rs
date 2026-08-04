use clap::Parser;
use libvmr::{Arguments, IndexedVecErr, ObjectFile, Operation, Register};
use log::{debug, error, info, warn};
use std::fs;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::FromStr;

pub struct Assembler {
    objfile: ObjectFile,
}

fn scan_labels(lines: &[&str], objfile: &mut ObjectFile) {
    let mut pc: u16 = 0;

    for (line, line_num) in lines
        .iter()
        .map(|l| l.trim())
        .zip(1..)
        .filter(|(l, _)| !l.is_empty() && !l.starts_with(';'))
    {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if let Some(last_word) = parts.last().filter(|w| w.ends_with(':')) {
            let label_name = last_word.strip_suffix(':').expect("Label without suffix");

            if *parts.first().expect("No first part") == "EXPORT" {
                if parts.len() > 2 {
                    warn!(
                        "Malformed export at line {line_num}: '{line}'. Labels should not contain spaces.",
                    );
                }
                objfile
                    .exports
                    .push((label_name.to_string(), pc))
                    .expect("Too many exports");
            } else {
                if parts.len() > 1 {
                    warn!(
                        "Malformed export at line {line_num}: '{line}'. Labels should not contain spaces.",
                    );
                }
                let previous = objfile.labels.insert(label_name.to_string(), pc).is_some();
                if previous {
                    warn!("Duplicate label '{label_name}' at line {line_num}: '{line}'");
                }
            }
            continue;
        }

        if let Some(first_word) = parts.first()
            && let Ok(op) = Operation::from_str(first_word)
        {
            pc += u16::from(op.arg_type().to_offset());
        }
    }
}

#[derive(Debug)]
pub enum AssembleError {
    MissingOpcode { line: usize },
    UnknownOpcode { line: usize, opcode: String },
    MissingArgument { line: usize, index: usize },
    BytecodeTooLarge,
    ParseError(ParseError),
    ResolveError(ResolveError),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    InvalidRegister,
}

impl From<IndexedVecErr> for AssembleError {
    fn from(_: IndexedVecErr) -> Self {
        Self::BytecodeTooLarge
    }
}

fn parse_reg(s: &str) -> Option<Register> {
    match s.to_uppercase().as_str() {
        "R0" => Some(Register::R0),
        "R1" => Some(Register::R1),
        "R2" => Some(Register::R2),
        "R3" => Some(Register::R3),
        "R4" => Some(Register::R4),
        "R5" => Some(Register::R5),
        "R6" => Some(Register::R6),
        "R7" => Some(Register::R7),
        "R8" => Some(Register::R8),
        "R9" => Some(Register::R9),
        "R10" => Some(Register::R10),
        "R11" => Some(Register::R11),
        "R12" => Some(Register::R12),
        "R13" => Some(Register::R13),
        "R14" => Some(Register::R14),
        "R15" => Some(Register::R15),
        "RSP" => Some(Register::RSP),
        _ => None,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveError {
    InternalRelocationOverflow,
    ExternalRelocationOverflow,
    LiteralError(ParseIntError),
}

impl From<ParseIntError> for ResolveError {
    fn from(value: ParseIntError) -> Self {
        Self::LiteralError(value)
    }
}

impl From<ResolveError> for AssembleError {
    fn from(value: ResolveError) -> Self {
        Self::ResolveError(value)
    }
}

fn get_part<'a>(parts: &[&'a str], idx: usize, line_num: usize) -> Result<&'a str, AssembleError> {
    parts
        .get(idx)
        .copied()
        .ok_or(AssembleError::MissingArgument {
            line: line_num + 1,
            index: idx,
        })
}

impl Assembler {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            objfile: ObjectFile::new(name.to_string()),
        }
    }

    #[must_use]
    pub const fn get_object(&self) -> &ObjectFile {
        &self.objfile
    }

    fn push_reg(
        &mut self,
        parts: &[&str],
        idx: usize,
        line_num: usize,
    ) -> Result<(), AssembleError> {
        self.objfile.bytecode.push(
            parse_reg(get_part(parts, idx, line_num)?)
                .ok_or(AssembleError::ParseError(ParseError::InvalidRegister))? as u8,
        )?;
        Ok(())
    }

    /// Resolves an immediate operand at `parts[idx]` and pushes its big-endian `u16` bytes.
    fn push_imm(
        &mut self,
        parts: &[&str],
        idx: usize,
        line_num: usize,
    ) -> Result<(), AssembleError> {
        let imm_pos = self.objfile.bytecode.len();

        let raw_val = get_part(parts, idx, line_num)?;
        let val = Self::resolve_value(&mut self.objfile, raw_val, imm_pos)?;
        self.objfile
            .bytecode
            .extend_from_slice(&val.to_be_bytes())?;
        Ok(())
    }

    /// # Errors
    ///
    /// When incorrect format
    pub fn assemble(&mut self, input: &str) -> Result<(), AssembleError> {
        let lines: Vec<&str> = input.lines().collect();

        scan_labels(&lines, &mut self.objfile);

        for (line_num, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.ends_with(':') {
                continue;
            }

            self.assemble_line(line, line_num)?;
        }

        Ok(())
    }

    /// Parses and emits bytecode for a single instruction line.
    fn assemble_line(&mut self, line: &str, line_num: usize) -> Result<(), AssembleError> {
        let parts: Vec<&str> = line.split([' ', ',']).filter(|s| !s.is_empty()).collect();

        let raw_op = parts
            .first()
            .ok_or(AssembleError::MissingOpcode { line: line_num + 1 })?;

        let op = Operation::from_str(raw_op).map_err(|_err| AssembleError::UnknownOpcode {
            line: line_num + 1,
            opcode: (*raw_op).to_string(),
        })?;

        self.objfile.bytecode.push(op as u8)?;

        match op.arg_type() {
            Arguments::None => {}
            Arguments::Reg => {
                self.push_reg(&parts, 1, line_num)?;
            }
            Arguments::RegReg => {
                self.push_reg(&parts, 1, line_num)?;
                self.push_reg(&parts, 2, line_num)?;
            }
            Arguments::RegImm => {
                self.push_reg(&parts, 1, line_num)?;
                self.push_imm(&parts, 2, line_num)?;
            }
            Arguments::Imm => {
                self.push_imm(&parts, 1, line_num)?;
            }
            Arguments::ImmImm => {
                self.push_imm(&parts, 1, line_num)?;
                self.push_imm(&parts, 2, line_num)?;
            }
            Arguments::RegImmReg => {
                self.push_reg(&parts, 1, line_num)?;
                self.push_imm(&parts, 2, line_num)?;
                self.push_reg(&parts, 3, line_num)?;
            }
            Arguments::ImmReg => {
                self.push_imm(&parts, 1, line_num)?;
                self.push_reg(&parts, 2, line_num)?;
            }
            Arguments::RegImmImm => {
                self.push_reg(&parts, 1, line_num)?;
                self.push_imm(&parts, 2, line_num)?;
                self.push_imm(&parts, 3, line_num)?;
            }
        }

        Ok(())
    }

    fn resolve_value(objfile: &mut ObjectFile, s: &str, imm_pos: u16) -> Result<u16, ResolveError> {
        if let Some(label_name) = s.strip_prefix(".").map(str::trim) {
            if let Some(&local_offset) = objfile.labels.get(label_name) {
                // Local label
                objfile
                    .internal_relocations
                    .push(imm_pos)
                    .map_err(|_err| ResolveError::InternalRelocationOverflow)?;
                Ok(local_offset)
            } else {
                // Foreign label
                objfile
                    .external_relocations
                    .push((label_name.to_string(), imm_pos))
                    .map_err(|_err| ResolveError::ExternalRelocationOverflow)?;

                Ok(0)
            }
        } else if let Some(hex_str) = s.strip_prefix("0x") {
            Ok(u16::from_str_radix(hex_str, 16)?)
        } else {
            Ok(s.parse::<u16>()?)
        }
    }
}

#[derive(Parser, Debug)]
#[command(about = "Assembler for VMR")]
struct Args {
    #[arg(value_name = "INPUT_FILE")]
    input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT_FILE", default_value = "out.bin")]
    output: PathBuf,

    /// Enable verbose logging during assembly
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    env_logger::builder().format_timestamp(None).init();

    let input_content = match fs::read_to_string(&args.input) {
        Ok(content) => content,
        Err(e) => {
            error!("Error reading input file {}: {}", args.input.display(), e);
            std::process::exit(1);
        }
    };

    if args.verbose {
        info!("Assembling {}...", args.input.display());
    }

    let name = args
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("Invalid filename");

    let mut assembler = Assembler::new(name);

    match assembler.assemble(&input_content) {
        Ok(()) => {
            let objfile = assembler.get_object();
            debug!("ObjectFile: {objfile}");
            if let Err(e) = fs::write(&args.output, objfile.to_binary()) {
                error!(
                    "Error writing to output file {}: {e}",
                    args.output.display()
                );
                std::process::exit(1);
            }
            info!("Successfully assembled to {}", args.output.display());
        }
        Err(e) => {
            error!("Assembly Error: {e:?}");
            std::process::exit(1);
        }
    }
}
