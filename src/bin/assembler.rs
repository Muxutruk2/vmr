use clap::Parser;
use libvmr::{Arguments, IndexedVecErr, ObjectFile, Operation, Register};
use log::{debug, error, info, warn};
use std::fs;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::FromStr;

pub struct Assembler {}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
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
    ParseError(String),
    ResolveError(ResolveError),
}

impl From<String> for AssembleError {
    fn from(err: String) -> Self {
        Self::ParseError(err)
    }
}
impl From<IndexedVecErr> for AssembleError {
    fn from(_: IndexedVecErr) -> Self {
        Self::BytecodeTooLarge
    }
}
fn parse_reg(s: &str) -> Result<Register, String> {
    match s.to_uppercase().as_str() {
        "R0" => Ok(Register::R0),
        "R1" => Ok(Register::R1),
        "R2" => Ok(Register::R2),
        "R3" => Ok(Register::R3),
        "R4" => Ok(Register::R4),
        "R5" => Ok(Register::R5),
        "R6" => Ok(Register::R6),
        "R7" => Ok(Register::R7),
        "R8" => Ok(Register::R8),
        "R9" => Ok(Register::R9),
        "R10" => Ok(Register::R10),
        "R11" => Ok(Register::R11),
        "R12" => Ok(Register::R12),
        "R13" => Ok(Register::R13),
        "R14" => Ok(Register::R14),
        "R15" => Ok(Register::R15),
        "RSP" => Ok(Register::RSP),
        _ => Err(format!("Invalid register: {s}")),
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

impl Assembler {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// # Errors
    ///
    /// When incorrect format
    pub fn assemble(&mut self, input: &str, name: &str) -> Result<ObjectFile, AssembleError> {
        let mut objfile = ObjectFile::new(name.to_string());

        let lines: Vec<&str> = input.lines().collect();

        scan_labels(&lines, &mut objfile);

        for (line_num, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.ends_with(':') {
                continue;
            }

            let parts: Vec<&str> = line.split([' ', ',']).filter(|s| !s.is_empty()).collect();

            let raw_op = parts
                .first()
                .ok_or(AssembleError::MissingOpcode { line: line_num + 1 })?;

            let op = Operation::from_str(raw_op).map_err(|_err| AssembleError::UnknownOpcode {
                line: line_num + 1,
                opcode: (*raw_op).to_string(),
            })?;

            objfile.bytecode.push(op as u8)?;

            let get_part = |idx: usize| -> Result<&str, AssembleError> {
                parts
                    .get(idx)
                    .copied()
                    .ok_or(AssembleError::MissingArgument {
                        line: line_num + 1,
                        index: idx,
                    })
            };

            match op.arg_type() {
                Arguments::Reg => {
                    objfile.bytecode.push(parse_reg(get_part(1)?)? as u8)?;
                }
                Arguments::RegReg => {
                    objfile.bytecode.push(parse_reg(get_part(1)?)? as u8)?;
                    objfile.bytecode.push(parse_reg(get_part(2)?)? as u8)?;
                }
                Arguments::RegImm => {
                    let imm_pos2 = objfile.bytecode.len() + 1;
                    objfile.bytecode.push(parse_reg(get_part(1)?)? as u8)?;
                    let imm = Self::resolve_value(&mut objfile, get_part(2)?, imm_pos2)?;
                    objfile.bytecode.extend_from_slice(&imm.to_be_bytes())?;
                }
                Arguments::Imm => {
                    let imm_pos = objfile.bytecode.len();
                    let imm = Self::resolve_value(&mut objfile, get_part(1)?, imm_pos)?;
                    objfile.bytecode.extend_from_slice(&imm.to_be_bytes())?;
                }
                Arguments::None => {}
                Arguments::ImmImm => {
                    let imm_pos1 = objfile.bytecode.len();
                    let imm_pos2 = objfile.bytecode.len() + 1;
                    let imm1 = Self::resolve_value(&mut objfile, get_part(1)?, imm_pos1)?;
                    let imm2 = Self::resolve_value(&mut objfile, get_part(2)?, imm_pos2)?;
                    objfile.bytecode.extend_from_slice(&imm1.to_be_bytes())?;
                    objfile.bytecode.extend_from_slice(&imm2.to_be_bytes())?;
                }
                Arguments::RegImmReg => {
                    objfile.bytecode.push(parse_reg(get_part(1)?)? as u8)?;
                    let imm_pos1 = objfile.bytecode.len();
                    let imm1 = Self::resolve_value(&mut objfile, get_part(2)?, imm_pos1)?;
                    objfile.bytecode.extend_from_slice(&imm1.to_be_bytes())?;
                    objfile.bytecode.push(parse_reg(get_part(3)?)? as u8)?;
                }
                Arguments::ImmReg => {
                    let imm_pos1 = objfile.bytecode.len();
                    let imm = Self::resolve_value(&mut objfile, get_part(1)?, imm_pos1)?;
                    objfile.bytecode.extend_from_slice(&imm.to_be_bytes())?;
                    objfile.bytecode.push(parse_reg(get_part(2)?)? as u8)?;
                }
                Arguments::RegImmImm => {
                    objfile.bytecode.push(parse_reg(get_part(1)?)? as u8)?;
                    let imm_pos1 = objfile.bytecode.len();
                    let imm1 = Self::resolve_value(&mut objfile, get_part(2)?, imm_pos1)?;
                    objfile.bytecode.extend_from_slice(&imm1.to_be_bytes())?;
                    let imm_pos2 = objfile.bytecode.len();
                    let imm2 = Self::resolve_value(&mut objfile, get_part(3)?, imm_pos2)?;
                    objfile.bytecode.extend_from_slice(&imm2.to_be_bytes())?;
                }
            }
        }

        Ok(objfile)
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

    let mut assembler = Assembler::new();

    match assembler.assemble(
        &input_content,
        args.input
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("Invalid filename"),
    ) {
        Ok(objfile) => {
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
