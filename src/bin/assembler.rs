use clap::Parser;
use libvmr::*;
use log::{debug, error, info, warn};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

pub struct Assembler {}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}

impl Assembler {
    pub fn new() -> Self {
        Self {}
    }

    fn scan_labels(&self, lines: &[&str], objfile: &mut ObjectFile) {
        let mut pc: u16 = 0;

        for line in lines
            .iter()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with(';'))
        {
            let parts: Vec<&str> = line.split_whitespace().collect();

            if let Some(last_word) = parts.last().filter(|w| w.ends_with(':')) {
                let label_name = last_word.strip_suffix(':').unwrap();

                if parts[0] == "EXPORT" {
                    if parts.len() > 2 {
                        warn!(
                            "Malformed export at PC {}: '{}'. Labels should not contain spaces.",
                            pc, line
                        );
                    }
                    objfile.exports.push((label_name.to_string(), pc));
                } else {
                    if parts.len() > 1 {
                        warn!(
                            "Malformed label at PC {}: '{}'. Labels should not contain spaces.",
                            pc, line
                        );
                    }
                    objfile.labels.insert(label_name.to_string(), pc);
                }
                continue;
            }

            if let Some(first_word) = parts.first()
                && let Ok(op) = Operation::from_str(first_word)
            {
                pc += op.arg_type().to_offset() as u16;
            }
        }
    }

    pub fn assemble(&mut self, input: &str, name: &str) -> Result<ObjectFile, String> {
        let mut objfile = ObjectFile::new(name.to_string());

        let lines: Vec<&str> = input.lines().collect();

        self.scan_labels(&lines, &mut objfile);

        for (line_num, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.ends_with(':') {
                continue;
            }

            let parts: Vec<&str> = line.split([' ', ',']).filter(|s| !s.is_empty()).collect();

            let op = Operation::from_str(parts[0])
                .map_err(|_| format!("Line {}: Unknown opcode {}", line_num + 1, parts[0]))?;

            objfile.bytecode.push(op as u8);

            // Encode arguments based on the arg_type
            match op.arg_type() {
                Arguments::Reg => {
                    objfile.bytecode.push(self.parse_reg(parts[1])? as u8);
                }
                Arguments::RegReg => {
                    objfile.bytecode.push(self.parse_reg(parts[1])? as u8);
                    objfile.bytecode.push(self.parse_reg(parts[2])? as u8);
                }
                Arguments::RegImm => {
                    let imm_pos2 = objfile.bytecode.len() as u16 + 1;
                    objfile.bytecode.push(self.parse_reg(parts[1])? as u8);
                    let imm = Self::resolve_value(&mut objfile, parts[2], imm_pos2)?;
                    objfile.bytecode.extend_from_slice(&imm.to_be_bytes()); // Big Endian
                }
                Arguments::Imm => {
                    let imm_pos = objfile.bytecode.len() as u16;
                    let imm = Self::resolve_value(&mut objfile, parts[1], imm_pos)?;
                    objfile.bytecode.extend_from_slice(&imm.to_be_bytes());
                }
                Arguments::None => {}
                Arguments::ImmImm => {
                    let imm_pos1 = objfile.bytecode.len() as u16;
                    let imm_pos2 = objfile.bytecode.len() as u16 + 1;
                    let imm1 = Self::resolve_value(&mut objfile, parts[1], imm_pos1)?;
                    let imm2 = Self::resolve_value(&mut objfile, parts[2], imm_pos2)?;
                    objfile.bytecode.extend_from_slice(&imm1.to_be_bytes());
                    objfile.bytecode.extend_from_slice(&imm2.to_be_bytes());
                }
                Arguments::RegImmReg => {
                    todo!("RegImmReg OpCodes are not supported")
                }
                Arguments::ImmReg => {
                    let imm_pos1 = objfile.bytecode.len() as u16;
                    let imm = Self::resolve_value(&mut objfile, parts[1], imm_pos1)?;
                    objfile.bytecode.extend_from_slice(&imm.to_be_bytes());
                    objfile.bytecode.push(self.parse_reg(parts[2])? as u8);
                }
            }
        }

        Ok(objfile)
    }

    fn parse_reg(&self, s: &str) -> Result<Register, String> {
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
            _ => Err(format!("Invalid register: {}", s)),
        }
    }

    fn resolve_value(objfile: &mut ObjectFile, s: &str, imm_pos: u16) -> Result<u16, String> {
        if let Some(label_name) = s.strip_prefix(".").map(str::trim) {
            if let Some(&local_offset) = objfile.labels.get(label_name) {
                // Local label
                objfile.internal_relocations.push(imm_pos);
                Ok(local_offset)
            } else {
                // Foreign label
                objfile
                    .external_relocations
                    .push((label_name.to_string(), imm_pos));
                Ok(0)
            }
        } else if let Some(hex_str) = s.strip_prefix("0x") {
            u16::from_str_radix(hex_str, 16).map_err(|_| format!("Invalid hex: {}", s))
        } else {
            s.parse::<u16>()
                .map_err(|_| format!("Invalid literal: {}", s))
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
            error!("Error reading input file {:?}: {}", args.input, e);
            std::process::exit(1);
        }
    };

    if args.verbose {
        info!("Assembling {:?}...", args.input);
    }

    let mut assembler = Assembler::new();

    match assembler.assemble(
        &input_content,
        args.input.file_stem().and_then(|s| s.to_str()).unwrap(),
    ) {
        Ok(objfile) => {
            debug!("ObjectFile: {objfile}");
            if let Err(e) = fs::write(&args.output, objfile.to_binary()) {
                error!("Error writing to output file {:?}: {}", args.output, e);
                std::process::exit(1);
            }
            info!("Successfully assembled to {:?}", args.output);
        }
        Err(e) => {
            error!("Assembly Error: {}", e);
            std::process::exit(1);
        }
    }
}
