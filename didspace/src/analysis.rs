
// src/analysis.rs
use wasmparser::{Parser, Payload};
use std::collections::HashMap;

pub struct WasmAnalysis {
    pub section_sizes: HashMap<String, usize>,
    pub function_count: usize,
    pub imports: usize,
    pub exports: usize,
    pub instruction_freq: HashMap<String, usize>,
}

impl WasmAnalysis {
    pub fn analyze(bytes: &[u8]) -> Result<Self, String> {
        let mut section_sizes = HashMap::new();
        let mut function_count = 0;
        let mut imports = 0;
        let mut exports = 0;
        let mut instruction_freq = HashMap::new();

        let mut parser = Parser::new(0);
        let mut offset = 0;

        while let Ok(chunk) = parser.parse(&bytes[offset..], true) {
            match chunk {
                wasmparser::Chunk::Parsed { payload, .. } => {
                    match payload {

                        Payload::Version { .. } => {}
                        Payload::TypeSection(types) => {
                            section_sizes.insert("Type".to_string(), types.range().end - types.range().start);
                        }
                        Payload::ImportSection(imports_section) => {
                            imports += imports_section.count();
                            section_sizes.insert("Import".to_string(), imports_section.range().end - imports_section.range().start);
                        }
                        Payload::FunctionSection(funcs) => {
                            function_count += funcs.count();
                            section_sizes.insert("Function".to_string(), funcs.range().end - funcs.range().start);
                        }
                        Payload::ExportSection(exports_section) => {
                            exports += exports_section.count();
                            section_sizes.insert("Export".to_string(), exports_section.range().end - exports_section.range().start);
                        }
                        Payload::CodeSectionEntry(code) => {
                            for op in code.get_operators_reader().unwrap() {
                                let op_name = format!("{:?}", op.unwrap());
                                *instruction_freq.entry(op_name).or_insert(0) += 1;
                            }
                        }
                        Payload::End(_) => break,
                    }
                }
                wasmparser::Chunk::NeedMoreData(_) => break,
            }
            offset += chunk.consumed();
        }

        Ok(Self {
            section_sizes,
            function_count: function_count as usize,
            imports: imports as usize,
            exports: exports as usize,
            instruction_freq,
        })
    }

    pub fn report(&self) -> String {
        let mut out = String::new();
        out.push_str("=== WASM Analysis Report ===\n\n");

        out.push_str("Sections:\n");
        for (section, size) in &self.section_sizes {
            out.push_str(&format!("  - {}: {} bytes\n", section, size));
        }

        out.push_str(&format!("\nFunctions: {}\nImports: {}\nExports: {}\n", 
            self.function_count, self.imports, self.exports));

        out.push_str("\nInstruction Frequency:\n");
        for (instr, count) in &self.instruction_freq {
            out.push_str(&format!("  {}: {}\n", instr, count));
        }

        out
    }
}
