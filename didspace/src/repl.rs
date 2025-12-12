use rustyline::{Editor, history::DefaultHistory};
use rustyline::completion::{Completer, Pair};
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::Helper;
use rustyline::error::ReadlineError;
use rustyline::validate::{Validator, ValidationResult, ValidationContext};
use std::fs;
use crate::utils::highlight_wat;
use crate::hex_dump::wasm_to_hex;
use crate::converter::{wat_to_wasm, wasm_to_wat};
use crate::analysis::WasmAnalysis;

const COMMANDS: &[&str] = &["wat2wasm", "wasm2wat", "hex", "analyze", "help", "exit"];

pub struct CommandCompleter;

// ✅ Implement all required traits here
impl rustyline::Helper for CommandCompleter {}

impl Completer for CommandCompleter {
    type Candidate = Pair;
    fn complete(
        &self, 
        line: &str, 
        pos: usize,
        _ctx: &rustyline::Context<'_>
    ) -> Result<(usize, Vec<Self::Candidate>), rustyline::error::ReadlineError> {
        let candidates: Vec<Self::Candidate> = COMMANDS
            .iter()
            .filter(|cmd| cmd.starts_with(line))
            .map(|cmd| rustyline::completion::Pair {
                display: cmd.to_string(),
                replacement: cmd.to_string(),
            })
            .collect();
        Ok((pos, candidates))
    }
}

impl Hinter for CommandCompleter {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        None
    }
}

impl Validator for CommandCompleter {
    fn validate(&self, _ctx: &mut ValidationContext) -> Result<ValidationResult, ReadlineError> {
        Ok(ValidationResult::Valid(None))
    }
}

impl rustyline::highlight::Highlighter for CommandCompleter {}

pub fn start_repl() -> anyhow::Result<()> where CommandCompleter: Helper {
    println!("Welcome to WASM REPL! Type 'help' for commands, 'exit' to quit.");
    let mut rl = Editor::<CommandCompleter, DefaultHistory>::new()?;

    loop {
        let line = rl.readline("didspace> ");
        rl.set_helper(Some(CommandCompleter));
        match line {
            Ok(input) => {
                let parts: Vec<&str> = input.trim().split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                match parts[0] {
                    "exit" => break,
                    "help" => println!("Commands: wat2wasm, wasm2wat, hex, analyze, exit"),
                    "wat2wasm" => {
                        let wat_code = parts[1..].join(" ");
                        match wat_to_wasm(&wat_code) {
                            Ok(bytes) => println!("WASM bytes: {:?}", bytes),
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                    "wasm2wat" => {
                        if parts.len() < 2 {
                            println!("Usage: wasm2wat <file>");
                            continue;
                        }
                        let file = parts[1];
                        let bytes = fs::read(file)?;
                        match wasm_to_wat(&bytes) {
                            Ok(wat) => println!("{}", highlight_wat(&wat)),
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                    "hex" => {
                        if parts.len() < 2 {
                            println!("Usage: hex <file>");
                            continue;
                        }
                        let file = parts[1];
                        let bytes = fs::read(file)?;
                        println!("{}", wasm_to_hex(&bytes));
                    }
                    "analyze" => {
                        if parts.len() < 2 {
                            println!("Usage: analyze <file>");
                            continue;
                        }
                        let file = parts[1];
                        let bytes = fs::read(file)?;
                        let report = WasmAnalysis::analyze(&bytes).map_err(anyhow::Error::msg)?;
                        println!("{}", report.report());
                    }
                    "profile" => {
                        if parts.len() < 2 {
                            println!("Usage: profile <file>");
                            continue;
                        }
                        let file = parts[1];
                        let bytes = fs::read(file)?;
                        WasmAnalysis::profile_functions(&bytes);
                    }
                        
                    _ => println!("Unknown command: {}", parts[0]),
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}
