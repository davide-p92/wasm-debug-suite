use wasmparser::{Parser, Payload};
use std::collections::HashMap;
use anyhow::Result;

pub struct WasmAnalysis {
    pub section_sizes: HashMap<String, usize>,
    pub function_count: usize,
    pub imports: usize,
    pub exports: usize,
    pub instruction_freq: HashMap<String, usize>,
}

impl WasmAnalysis {
    pub fn analyze(bytes: &[u8]) -> Result<Self> {
        let mut section_sizes = HashMap::new();
        let mut function_count = 0;
        let mut imports = 0;
        let mut exports = 0;
        let mut instruction_freq = HashMap::new();

        let parser = Parser::new(0);

        for payload in parser.parse_all(bytes) {
            match payload.unwrap() {
                Payload::Version { .. } => {}
                Payload::TypeSection(types) => {
                    section_sizes.insert("Type".into(), types.range().end - types.range().start);
                }
                Payload::ImportSection(imports_section) => {
                    imports += imports_section.count();
                    section_sizes.insert("Import".into(), imports_section.range().end - imports_section.range().start);
                }
                Payload::FunctionSection(funcs) => {
                    function_count += funcs.count();
                    section_sizes.insert("Function".into(), funcs.range().end - funcs.range().start);
                }
                Payload::ExportSection(exports_section) => {
                    exports += exports_section.count();
                    section_sizes.insert("Export".into(), exports_section.range().end - exports_section.range().start);
                }
                Payload::CodeSectionEntry(code) => {
                    for op in code.get_operators_reader().unwrap() {
                        let op_name = format!("{:?}", op.unwrap());
                        *instruction_freq.entry(op_name).or_insert(0) += 1;
                    }
                }
                Payload::End(_) => break,
                _ => {}
            }
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

        out.push_str(&format!(
            "\nFunctions: {}\nImports: {}\nExports: {}\n",
            self.function_count, self.imports, self.exports
        ));

        out.push_str("\nInstruction Frequency:\n");
        for (instr, count) in &self.instruction_freq {
            out.push_str(&format!("  {}: {}\n", instr, count));
        }

        // ✅ Optimization hints
        out.push_str("\nOptimization Hints:\n");
        if let Some(custom_size) = self.section_sizes.get("Custom") {
            if *custom_size > 500 {
                out.push_str("  - Large custom section detected. Consider stripping debug info.\n");
            }
        }
        if self.function_count > 100 {
            out.push_str("  - High function count. Consider inlining or reducing complexity.\n");
        }
        if self.instruction_freq.get("Call").unwrap_or(&0) > &50 {
            out.push_str("  - Many calls detected. Consider reducing call overhead.\n");
        }

        out
    }

    pub fn analyze_sizes(bytes: &[u8]) -> HashMap<String, usize> {
        let mut sizes = HashMap::new();
        let parser = Parser::new(0);

        for payload in parser.parse_all(bytes) {
            match payload.unwrap() {
                Payload::TypeSection(s) => {
                    sizes.insert("Type".into(), s.range().end - s.range().start);
                }
                Payload::ImportSection(s) => {
                    sizes.insert("Import".into(), s.range().end - s.range().start);
                }
                Payload::FunctionSection(s) => {
                    sizes.insert("Function".into(), s.range().end - s.range().start);
                }
                Payload::CodeSectionEntry(_) => {
                    *sizes.entry("Code".into()).or_insert(0) += 1;
                }
                _ => {}
                       }
        }
        sizes
    }

    pub fn to_report_string(&self) -> String {
        let mut out = String::new();

        out.push_str("WASM Analysis\n");
        out.push_str("================\n\n");

        out.push_str("Sections (bytes):\n");
        // ordinati per nome (o fai per size se preferisci)
        let mut sections: Vec<_> = self.section_sizes.iter().collect();
        sections.sort_by_key(|(k, _)| *k);
        for (name, size) in sections {
            out.push_str(&format!("  - {:<10} {}\n", name, size));
        }

        out.push_str("\nCounts:\n");
        out.push_str(&format!("  • functions: {}\n", self.function_count));
        out.push_str(&format!("  • imports:   {}\n", self.imports));
        out.push_str(&format!("  • exports:   {}\n", self.exports));

        out.push_str("\nTop instructions:\n");
        let mut instr: Vec<_> = self.instruction_freq.iter().collect();
        instr.sort_by_key(|(_, v)| std::cmp::Reverse(**v));
        for (name, count) in instr.into_iter().take(30) {
            out.push_str(&format!("  - {:<40} {}\n", name, count));
        }

        out
    }

    pub fn analyze_report(bytes: &[u8]) -> Result<String> {
        let a = Self::analyze(bytes)?;
        Ok(a.to_report_string())
    }

    pub fn profile_functions(bytes: &[u8]) -> Result<String> {
        let parser = Parser::new(0);
        let mut out = String::new();
        out.push_str("Profile Functions\n");
        out.push_str("=================\n");

        for payload in parser.parse_all(bytes) {
            match payload? {
                Payload::CodeSectionEntry(code) => {
                    let mut count = 0;
                    //for op in code.get_operators_reader().unwrap() {
                    let mut r = code.get_operators_reader()?;
                    while !r.eof() {
                        let _op = r.read()?; // non serve usarlo, basta contare
                        count += 1;
                    }

                    out.push_str(&format!(
                        "Function at offset {}: {} instructions\n",
                        code.range().start,
                        count
                    ));

                    println!("Function at offset {}: {} instructions", code.range().start, count);
                    if count > 500 {
                        out.push_str("  ⚠ Hotspot detected: Consider optimizing this function.");
                        println!("  ⚠ Hotspot detected: Consider optimizing this function.");
                    }
                }
                Payload::End(_) => break,
                _ => {}
            }
        }
        Ok(out)
    }
}
