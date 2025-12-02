// new file: src/disasm.rs
// src/disasm.rs
use std::fmt::Write;
use std::rc::Rc;
use std::collections::HashMap;

use wasmparser::{Parser, Payload, Operator};
use anyhow::Result;
use crate::DwarfParser;

pub type Instr = (usize, String);

#[derive(Debug)]
pub struct FunctionDisasm {
    pub func_index: u32,
    pub name: String,
    pub instrs: Vec<Instr>, // (offset, textual op)
}

#[derive(Debug)]
pub struct ModuleDisasm<'a> {
    pub functions: Vec<FunctionDisasm>,
    pub dwarf: Option<Rc<DwarfParser<'a>>>,
}

impl<'a> ModuleDisasm<'a> {
    pub fn from_wasm(bytes: &[u8], dwarf: Option<Rc<DwarfParser<'a>>>) -> Result<Self> {
        let mut parser = Parser::new(0);
        let mut functions = Vec::new();
        let mut func_index: u32 = 0;
        let mut function_imports: u32 = 0;

        while let Ok(payload) = parser.parse(bytes, true) {
            match payload {
                wasmparser::Chunk::NeedMoreData(_) => break,
                wasmparser::Chunk::Parsed { payload, .. } => match payload {
                    Payload::ImportSection(imports) => {
                        for import in imports {
                            let imp = import?;
                            if let wasmparser::TypeRef::Func(_) = imp.ty {
                                function_imports += 1;
                                println!(
                                    "Imported function: {}::{}",
                                    imp.module, imp.name
                                );
                            }
                        }
                    }
                    Payload::CodeSectionEntry(body) => {
                        let idx = function_imports + func_index;
                        let mut reader = body.get_operators_reader()?;
                        let mut instrs = Vec::new();
                        while !reader.eof() {
                            let pos = reader.original_position();
                            let op = reader.read()?;
                            instrs.push((pos, format!("{:?}", op)));
                        }
                        let func_name = if let Some(ref dwarf_rc) = dwarf {
                            dwarf_rc.get_function_name(idx)
                            .unwrap_or_else(|| format!("func_{}", idx))
                        } else {
                            format!("func_{}", idx)
                        };
                        functions.push(FunctionDisasm { 
                            func_index: idx, 
                            name: func_name, 
                            instrs });
                        func_index += 1;
                    }
                    Payload::End(_) => break,
                    _ => {}
                },
            }
        }

        Ok(Self { functions, dwarf })
    }

    pub fn disassemble_function(&self, func_name: &str) -> Option<&FunctionDisasm> {
        self.functions.iter().find(|f| f.name == func_name)
    }

    // Suche Funktionsindex per Name (case-insensitive)
    pub fn find_function_index_by_name(&self, func_name: &str) -> Option<usize> {
        let needle = func_name.to_ascii_lowercase();
        self.functions
            .iter()
            .position(|f| f.name.to_ascii_lowercase() == needle)
    }

/*    pub fn print_function(&self, func_name: &str) -> anyhow::Result<String> {
        if let Some(func) = self.disassemble_function(func_name) {
            let mut out = String::new();
            writeln!(
                &mut out,
                "Disassembly of function '{}' ({} instructions):",
                func.name,
                func.instrs.len()
            )?;
            for (offset, op) in &func.instrs {
                if let Some(ref dwarf_rc) = &self.dwarf {
                    if let Some((file, line)) = dwarf_rc.get_source_location(*offset as u64) {
                        writeln!(&mut out, "  0x{:08x}: {:<20} ;; {}:{}", offset, op, file, line)?;
                    } else {
                        writeln!(&mut out, "  0x{:08x}: {}", offset, op)?;
                    }
                }
            }
            Ok(out)
        } else {
            Err(anyhow::anyhow!("Function '{}' not found in disassembly", func_name))
        }
    }

    pub fn get_instr(&self, func_index: u32, instr_idx: usize) -> Option<(usize, String)> {
        self.functions
            .get(func_index as usize)?
            .instrs
            .get(instr_idx)
            .cloned()
    }*/

    // Disassemblierung als String ausgeben; optional mit Quellzeilen (DWARF)
    pub fn print_function(&self, func_name: &str) -> Result<String> {
        let mut out = String::new();

        let idx = match self.find_function_index_by_name(func_name) {
            Some(i) => i,
            None => {
                // evtl. Zahl? (z.B. „disass 12“)
                if let Ok(numeric) = func_name.parse::<usize>() {
                    if numeric < self.functions.len() {
                        numeric
                    } else {
                        anyhow::bail!("Funktion '{}' nicht gefunden", func_name);
                    }
                } else {
                    anyhow::bail!("Funktion '{}' nicht gefunden", func_name);
                }
            }
        };

        let f = &self.functions[idx];
        use std::fmt::Write as _;

        writeln!(&mut out, "Disassembly of {} (index {}):", f.name, f.func_index)?;

        for (offset, op) in &f.instrs {
            write!(&mut out, "  0x{:04X}: {}", offset, op)?;

            if let Some(ref d) = self.dwarf {
                if let Some((file, line)) = d.get_source_location(*offset as u64) {
                    write!(&mut out, "    ; {}:{}", file, line)?;
                }
            }

            writeln!(&mut out)?;
        }

        Ok(out)
    }

    // Zugriff, falls du im Step-Mode per Index eine Instr holst
    pub fn get_instr(&self, func_index: usize, instr_index: usize) -> Option<(usize, String)> {
        self.functions.get(func_index).and_then(|f| f.instrs.get(instr_index)).cloned()
    }

    pub fn func_len(&self, func_index: u32) -> Option<usize> {
        self.functions
            .get(func_index as usize)
            .map(|f| f.instrs.len())
    }
}

