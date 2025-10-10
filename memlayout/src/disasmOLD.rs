// new file: src/disasm.rs
use std::collections::HashMap;
use wasmparser::{Operator, Parser, Payload, Chunk, TypeRef, FuncValidator, Validator, WasmFeatures};
use anyhow::Result;

pub type Instr = (usize, wasmparser::Operator<'static>); // offset in function, operator (owned)

pub struct FunctionDisasm {
    pub func_index: u32,
    pub instrs: Vec<Instr>, // (offset_in_func, operator)
}

pub struct ModuleDisasm {
    pub functions: Vec<Vec<(usize, String)>>, // func_index -> disasm
}

impl ModuleDisasm {
    pub fn from_wasm(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut parser = Parser::new(0);
        let mut func_bodies = Vec::new();
        let mut func_index = 0u32;
        let mut function_imports = 0u32;

        // first pass: count imports to map code section function indices
        //for payload in parser.parse_all(bytes) {
        loop {
            match parser.parse(bytes, true)? {
                Chunk::NeedMoreData(_) => break,
                Chunk::Parsed(payload) => match payload {
                    Payload::ImportSection(s) => {
                        for import in s {
                            let imp = import?;
                            if matches!(imp.ty, TypeRef::Func(_)) {
                                function_imports += 1;
                                println!("Imported function: {}::{:?}", imp.module, imp.name);
                            }
                        }
                    }
                    Payload::CodeSectionEntry(body) => {
                        let mut reader = body.get_operators_reader()?;
                        let mut instrs = Vec::new();
                        while !reader.eof() {
                            let pos = reader.original_position();
                            let op = reader.read()?;
                            instrs.push((pos, format!("{:?}", op)));
                        }
                        functions.push(instrs);
                    }
                    Payload::End(_) => break,
                    _ => continue,
                },
            }
        }

        // second pass: parse code section
        let mut parser = Parser::new(0);
        let mut module_disasm = ModuleDisasm { functions: Vec::new() };
        while let wasmparser::Chunk::Parsed { consumed: _, payload: _ } = parser.parse(bytes, true)? {
            match payload {
                Payload::CodeSectionStart { count, .. } => {
                    // reader through payloads
                }
                Payload::CodeSectionEntry(body) => {
                    // body is FunctionBody
                    let idx = function_imports + func_index;
                    let mut reader = body.get_operators_reader()?;
                    let mut instrs = Vec::new();
                    while !reader.eof() {
                        let pos = reader.original_position();
                        let op = reader.read()?;
                        // To store operator, we must convert to owned — easiest: format to string for display,
                        // but here we keep operator by mapping to a string. (Operators are not 'static)
                        // We'll store text representation plus offset
                        instrs.push((pos, format!("{:?}", op)));
                    }
                    module_disasm.functions.insert(idx.try_into().unwrap(), FunctionDisasm { func_index: idx, instrs });
                    func_index += 1;
                }
                Payload::End(_) => break,
                _ => continue,
            }
        }
        Ok(module_disasm)
    }

    pub fn get_instr(&self, func_index: u32, instr_idx: usize) -> Option<(usize, String)> {
        self.functions
            .get(func_index as usize)?
            .get(instr_idx)
            .cloned()
    }

    pub fn func_len(&self, func_index: u32) -> Option<usize> {
        self.functions.get(func_index as usize).map(|f| f.instrs.len())
    }
}

