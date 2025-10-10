use std::collections::HashSet;
use std::rc::Rc;
use rustyline::{Editor, history::DefaultHistory};
use crate::wasmrt::WasmRuntime;
use crate::memlayout::MemoryLayout;
use crate::dwarfparser::DwarfParser;
use crate::disasm::ModuleDisasm;

pub struct WasmDebugger<'a> {
    runtime: WasmRuntime<'a>,
    dwarf: Rc<DwarfParser<'a>>,
    pub layout: Option<MemoryLayout>,
    disasm: ModuleDisasm,
    breakpoints: HashSet<String>,
    current_pc: u64, // Aktuelle Instruktionsadresse
    current_func: u32,
    current_instr: usize,
    is_running: bool,
}

impl<'a> WasmDebugger<'a> {
    pub fn new(runtime: WasmRuntime<'a>, dwarf: Rc<DwarfParser<'a>>, disasm: ModuleDisasm) -> Self {
        Self {
            runtime,
            dwarf,
            layout: None,
            disasm,
            breakpoints: HashSet::new(),
            current_pc: 0,
            current_func: 0,
            current_instr: 0,
            is_running: false,
        }
    }

    pub fn repl(&mut self) -> anyhow::Result<()> {
        let mut rl: Editor::<(), DefaultHistory> = Editor::new()?;
        // Optional: rl.load_history("debuggerhistory.txt").ok();
        println!("/ Welcome to wasmdbg - your interactive WebAssembly Debugger!");
        println!("Commands: break <func>, run, step, continue, print <var>, memdump, quit\n!");

        use rustyline::{DefaultEditor, Cmd, Event, EventHandler};
        let mut rl = DefaultEditor::new()?;
        loop {
            let line = rl.readline("(wasmdbg)> ")?;
            let args: Vec<&str> = line.trim().split_whitespace().collect();
            if args.is_empty() {
                continue;
            }

            match args[0] {
                "help" => {
                    println!("Commands: step, continue, break <line>, quit");
                }
                "break"/* if args.len() > 1*/ => {
                    if let Some(func) = args.get(1) {
                        println!("Breakpoint set at function '{}'", func);
                        self.breakpoints.insert(func.to_string());
                    }
                }
                "run" => {
                    println!("Running..");
                    self.is_running = true;
                    self.runtime.call_init_functions()?;
                    println!("Execution started");
                }
                "step" => {
                    println!(">> Stepping one instruction (simulated)");
                    self.debugger_step()?;
                    // TODO: später echte Instr-Steuerung via Wasmer-Interpreter
                }
                "continue" => {
                    println!(">> Continuing execution..");
                    self.is_running = true;
                }
                "print" if args.len() > 1 => {
                    let var_name = args[1];

                    // 1. Erst versuchen, ob die Variable als Global existiert
                    if let Ok(val) = self.runtime.get_global_as_i32(var_name) {
                        println!("{} = {} (global i32)", var_name, val);
                    } else {
                        // 2. Wenn kein Global gefunden, versuche DWARF-basiertes Memory-Layout
                        match self.layout {
                            Some(ref layout) => {
                                match layout.read_variable(var_name) {
                                    Ok(val) => println!("{} = {:?}", var_name, val),
                                    Err(_) => println!("Variable '{}' not found.", var_name),
                                }
                            }
                            None => {
                                println!("No memory layout loaded and '{}' is not a global.", var_name);
                            }
                        }
                    }
                }
                "memdump" => {
                    if args.len() < 2 {
                        println!("Usage: memdump <variable> [length]");
                        continue;
                    }
                    let var = args[1];
                    let len = if args.len() > 2 {
                        args[2].parse::<usize>().unwrap_or(64)
                    } else {
                        64
                    };
                    
                    match self.runtime.resolve_symbol_address(var) {
                        Some(addr) => {
                            println!("📍 {} @ 0x{:X}", var, addr);
                            match self.runtime.dump_memory(addr as usize, len) {
                                Ok(out) => println!("{}", out),
                                Err(e) => println!("Error: {}", e),
                            }
                        }
                        None => println!("Variable '{}' not found", var),
                    }
                }
                "symbols" => {
                    let query = args.get(1).map(|s| s.to_lowercase());
                    println!("📜 Symbol Table: ");
                    let symtab = self.runtime.symbol_table.borrow();
                    if symtab.is_empty() {
                        println!("(no symbols found)");
                    } else {
                        for (name, desc) in symtab.iter() {
                            if query.as_ref().map_or(true, |q| name.to_lowercase().contains(1)) {
                                println!("   {:<30} {}", name, desc);
                            }
                        }
                    }
                }
                "quit" | "exit" => {
                    println!("Exiting wasmdbg.");
                    break;
                }
                _ => {
                    println!("Unknown command. Try: break, run, step, continue, print, memdump, quit");
                }
            }
        }
        // Optional: rl.save_history("debuggerhistory.txt").ok();
        Ok(())
    }

    fn step(&mut self) -> anyhow::Result<()> {
        // Pointer vorlegen
        let func = self.current_func;
        let instr_idx = self.current_instr;
        if let Some((offset, op)) = self.disasm.get_instr(func, instr_idx) {
            // Operatortext zeigen
            println!(">> Step: func {} instr#{} @{} => {:?}", func, instr_idx, offset, op);
            // Source Mapping zeigen, wenn möglich
            // (Moduleniveau Code oder fiktive Adresse)
            // 1. Aktuelle Position ermitteln (DWARF Line)
            if let Some((file, line)) = self.dwarf.get_source_location(offset as u64) {
                println!("At {}:{}", file, line);
            }
            self.current_instr += 1;
        }else {
            println!("End of function or unknown function. Trying to advance to next function.");
            self.current_func += 1;
            self.current_instr = 0;
        }

/*
        // 2. Eine Instruktion ausführen
        self.runtime.step_instruction()?;
        self.current_pc += 1;

        // 3. Neue Position anzeigen
        if let Some((file, line)) = self.dwarf.get_source_location(self.current_pc) {
            println!("> Stepped to {}: {}", file, line);
        } else {
            println!("> Stepped to unknown address {:#x}", self.current_pc);
        }
*/
        Ok(())
    }

    pub fn show_current_location(&self, offset: u64) {
        if let Some((file, line)) = self.dwarf.get_source_location(offset) {
            println!("At {}:{} (offset 0x{:x})", file, line, offset);
        } else {
            println!("At unknown location (offset 0x{:x})", offset);
        }
    }

    // Führt einen echten Schritt in der Laufzeit aus UND zeigt danach
    // den aktuellen Quellcode-Ort + Instruktion an.
    pub fn debugger_step(&mut self) -> anyhow::Result<()> {
        println!(">> Executing one WASM instruction...");

        // 1️⃣ Wirklich eine Instruktion in der Laufzeit ausführen
        if let Err(e) = self.runtime.step_instruction() {
            println!("Runtime step failed: {}", e);
        }

        // 2️⃣ Danach die aktuelle Disassemblierung / Source Map anzeigen
        let func = self.current_func;
        let instr_idx = self.current_instr;

        if let Some((offset, op)) = self.disasm.get_instr(func, instr_idx) {
            println!(">> Step: func {} instr#{} @0x{:x} => {:?}", func, instr_idx, offset, op);

            // DWARF-Quelle ermitteln
            if let Some((file, line)) = self.dwarf.get_source_location(offset as u64) {
                println!("At {}:{}", file, line);
            } else {
                println!("(no DWARF source mapping for offset 0x{:x})", offset);
            }

            // Instruktionszeiger weiter
            self.current_instr += 1;
        } else {
            println!("End of function or unknown instruction. Moving to next function...");
            self.current_func += 1;
            self.current_instr = 0;
        }

        // 3️⃣ Speicherzustand optional anzeigen
        let snapshot = self.runtime.get_memory_snapshot();
        println!(
            "Memory (first 32 bytes): {:?}",
            &snapshot[..snapshot.len().min(32)]
        );

        Ok(())
    }

    fn continue_exec(&mut self) -> anyhow::Result<()> {
        println!("Running until breakpoint or end...");
        Ok(())
    }
}

