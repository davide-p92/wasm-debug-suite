// new file: src/disasm.rs
// src/disasm.rs
use std::collections::HashMap;
use wasmparser::{Parser, Payload, Operator};
use anyhow::Result;

pub type Instr = (usize, String);

#[derive(Debug)]
pub struct FunctionDisasm {
    pub func_index: u32,
    pub instrs: Vec<Instr>, // (offset, textual op)
}

#[derive(Debug)]
pub struct ModuleDisasm {
    pub functions: Vec<FunctionDisasm>,
}

impl ModuleDisasm {
    pub fn from_wasm(bytes: &[u8]) -> Result<Self> {
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
                        functions.push(FunctionDisasm { func_index: idx, instrs });
                        func_index += 1;
                    }
                    Payload::End(_) => break,
                    _ => {}
                },
            }
        }

        Ok(Self { functions })
    }

    pub fn get_instr(&self, func_index: u32, instr_idx: usize) -> Option<(usize, String)> {
        self.functions
            .get(func_index as usize)?
            .instrs
            .get(instr_idx)
            .cloned()
    }

    pub fn func_len(&self, func_index: u32) -> Option<usize> {
        self.functions
            .get(func_index as usize)
            .map(|f| f.instrs.len())
    }
}

