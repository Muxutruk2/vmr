use clap::Parser;
use libvmr::*;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct Assembler {
    labels: HashMap<String, u16>,
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            labels: HashMap::new(),
        }
    }

    /// First Pass: Determine the address of every label
    fn scan_labels(&mut self, lines: &[&str]) {
        let mut pc: u16 = 0;
        for line in lines {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }

            if line.ends_with(':') {
                let label = line[..line.len() - 1].to_string();
                self.labels.insert(label, pc);
            } else {
                // Parse the opcode to determine how many bytes this instruction takes
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(op) = self.parse_op(parts[0]) {
                    pc += op.arg_type().to_offset() as u16;
                }
            }
        }
    }

    /// Second Pass: Generate bytecode
    pub fn assemble(&mut self, input: &str) -> Result<Vec<u8>, String> {
        let lines: Vec<&str> = input.lines().collect();
        self.scan_labels(&lines);

        let mut bytecode = Vec::new();
        bytecode.extend_from_slice(&[0x76, 0x6D, 0x72]);

        for (line_num, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.ends_with(':') {
                continue;
            }

            let parts: Vec<&str> = line
                .split(|c: char| c == ' ' || c == ',')
                .filter(|s| !s.is_empty())
                .collect();

            let op = self.parse_op(parts[0]).ok_or(format!(
                "Line {}: Unknown opcode {}",
                line_num + 1,
                parts[0]
            ))?;

            bytecode.push(op as u8);

            // Encode arguments based on the arg_type
            match op.arg_type() {
                Arguments::Reg => {
                    bytecode.push(self.parse_reg(parts[1])? as u8);
                }
                Arguments::RegReg => {
                    bytecode.push(self.parse_reg(parts[1])? as u8);
                    bytecode.push(self.parse_reg(parts[2])? as u8);
                }
                Arguments::RegImm => {
                    bytecode.push(self.parse_reg(parts[1])? as u8);
                    let imm = self.resolve_value(parts[2])?;
                    bytecode.extend_from_slice(&imm.to_be_bytes()); // Big Endian
                }
                Arguments::Imm => {
                    let imm = self.resolve_value(parts[1])?;
                    bytecode.extend_from_slice(&imm.to_be_bytes());
                }
                Arguments::None => {}
                Arguments::ImmImm => {
                    let imm1 = self.resolve_value(parts[1])?;
                    let imm2 = self.resolve_value(parts[2])?;
                    bytecode.extend_from_slice(&imm1.to_be_bytes());
                    bytecode.extend_from_slice(&imm2.to_be_bytes());
                }
                Arguments::RegImmReg => {
                    todo!("RegImmReg OpCodes are not supported")
                }
                Arguments::ImmReg => {
                    let imm = self.resolve_value(parts[1])?;
                    bytecode.extend_from_slice(&imm.to_be_bytes());
                    bytecode.push(self.parse_reg(parts[2])? as u8);
                }
            }
        }

        Ok(bytecode)
    }

    fn parse_op(&self, s: &str) -> Option<Operation> {
        match s.to_uppercase().as_str() {
            "HALT" => Some(Operation::HALT),
            "NOP" => Some(Operation::NOP),
            "MOV" => Some(Operation::MOV),
            "MOV_IMM" => Some(Operation::MOV_IMM),
            "LOAD" => Some(Operation::LOAD),
            "LOAD_REL" => Some(Operation::LOAD_REL),
            "LOAD_IMM" => Some(Operation::LOAD_IMM),
            "STORE_R_R" => Some(Operation::STORE_R_R),
            "STORE_REL_R" => Some(Operation::STORE_REL_R),
            "STORE_IMM_R" => Some(Operation::STORE_IMM_R),
            "STORE_R_IMM" => Some(Operation::STORE_R_IMM),
            "STORE_REL_IMM" => Some(Operation::STORE_REL_IMM),
            "STORE_IMM_IMM" => Some(Operation::STORE_IMM_IMM),
            "ADD" => Some(Operation::ADD),
            "ADD_IMM" => Some(Operation::ADD_IMM),
            "SUB" => Some(Operation::SUB),
            "SUB_IMM" => Some(Operation::SUB_IMM),
            "AND" => Some(Operation::AND),
            "AND_IMM" => Some(Operation::AND_IMM),
            "OR" => Some(Operation::OR),
            "OR_IMM" => Some(Operation::OR_IMM),
            "XOR" => Some(Operation::XOR),
            "XOR_IMM" => Some(Operation::XOR_IMM),
            "SHL" => Some(Operation::SHL),
            "SHL_IMM" => Some(Operation::SHL_IMM),
            "SHR" => Some(Operation::SHR),
            "SHR_IMM" => Some(Operation::SHR_IMM),
            "CMP" => Some(Operation::CMP),
            "CMP_IMM" => Some(Operation::CMP_IMM),
            "PUSH" => Some(Operation::PUSH),
            "PUSH_M" => Some(Operation::PUSH_M),
            "PUSH_IMM" => Some(Operation::PUSH_IMM),
            "POP" => Some(Operation::POP),
            "POP_M" => Some(Operation::POP_M),
            "POP_IMM" => Some(Operation::POP_IMM),
            "JMP" => Some(Operation::JMP),
            "JMP_IMM" => Some(Operation::JMP_IMM),
            "JE" => Some(Operation::JE),
            "JE_IMM" => Some(Operation::JE_IMM),
            "JNE" => Some(Operation::JNE),
            "JNE_IMM" => Some(Operation::JNE_IMM),
            "CALL" => Some(Operation::CALL),
            "CALL_IMM" => Some(Operation::CALL_IMM),
            "RET" => Some(Operation::RET),
            _ => None,
        }
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

    fn resolve_value(&self, s: &str) -> Result<u16, String> {
        // Check if it's a label
        if let Some(&addr) = self.labels.get(s) {
            return Ok(addr);
        }
        // Otherwise try to parse as hex or dec
        if s.starts_with("0x") {
            u16::from_str_radix(&s[2..], 16).map_err(|_| format!("Invalid hex: {}", s))
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

    let input_content = match fs::read_to_string(&args.input) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading input file {:?}: {}", args.input, e);
            std::process::exit(1);
        }
    };

    if args.verbose {
        println!("Assembling {:?}...", args.input);
    }

    let mut assembler = Assembler::new();

    match assembler.assemble(&input_content) {
        Ok(binary) => {
            if let Err(e) = fs::write(&args.output, binary) {
                eprintln!("Error writing to output file {:?}: {}", args.output, e);
                std::process::exit(1);
            }
            println!("Successfully assembled to {:?}", args.output);
        }
        Err(e) => {
            eprintln!("Assembly Error: {}", e);
            std::process::exit(1);
        }
    }
}
