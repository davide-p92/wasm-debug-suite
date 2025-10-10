use std::rc::Rc;

mod memlayout;
use memlayout::{MemoryLayout};//, MemoryError};
mod dwarfparser;
use dwarfparser::DwarfParser;
mod errors;
use errors::{DwarfError, MemoryError};
mod wasmrt;
use wasmrt::WasmRuntime;
mod types;
mod memsegments;
mod debugger;
use crate::debugger::WasmDebugger;
mod disasm;
use disasm::ModuleDisasm;

fn main() -> Result<(), anyhow::Error> {
    // 1. WASM-Datei laden
    let wasm_bytes = std::fs::read("app.wasm")?;

    // 2. DWARF-Daten extrahieren
    let dwarf_parser = Rc::new(DwarfParser::from_wasm(&wasm_bytes)?);
        //.map_err(|e| anyhow::anyhow!("DWARF parsing failed: {}", e))?;
    let mut runtime = WasmRuntime::new(&wasm_bytes, Some(dwarf_parser.clone()))?;

    let dwarf_data = dwarf_parser.extract_memory_layout(&runtime)
        .map_err(|e| anyhow::anyhow!("Memory layout extraction failed: {}", e))?;
    let memory = runtime.get_memory_snapshot();
    let layout = MemoryLayout::new(&memory, &dwarf_data);

    // 3. Disassembler initialisieren
    let disasm = ModuleDisasm::from_wasm(&wasm_bytes)?;
    println!(
        "Disassembled {} functions",
        disasm.functions.len()
    );

    let mut dbg = WasmDebugger::new(runtime, dwarf_parser.clone(), disasm);
    dbg.layout = Some(layout); // V damit print <var> funktioniert
    dbg.repl()?;

/*
    // 3. WASM-Instanz erstelle    
    runtime.call_init_functions()?;
    let global_value = runtime.get_global_as_i32("GLOBAL_COUNTER")?;
    println!("Global value: {}", global_value);

    // Globale Variable setzen
    runtime.set_global_value("GLOBAL_COUNTER", wasmer::Value::I32(42))?;

    //Funktion aufrufen
    let results = runtime.call_function("add", &[wasmer::Value::I32(10)])?;
    println!("Function results: {:?}", results);

    // Tabellengröße abfragen
    let table_size = runtime.get_table_size("my_table")?;
    println!("Table size: {}", table_size);

    // Tabellenelement abrufen
    let element = runtime.get_table_element("my_table", 0)?;
    println!("Table element: {:?}", element);

    // Auf zusätzlichen Speicher zugreifen
    let additional_memory = runtime.get_additional_memory("extra_memory")?;
    //println!("Additional memory size: {} B", additional_memory.len());

    // 4. Speicherabbild erfassen

    let memory = runtime.get_memory_snapshot();

    // 5. Memory-Layout erstellen
    let layout = MemoryLayout::new(&memory, &dwarf_data);

    // 6. Variable inspizieren
    let counter_value = layout.read_variable("global_counter")?;
    println!("Counter value: {:?}", counter_value);

    // 7. Visuelle Darstellung generieren
    let visualization = layout.generate_visualization();
    std::fs::write("memory.html", visualization.render_html())?;

    println!("Hello, world!");
*/

    // 5. Debugger starten
    //let mut dbg = WasmDebugger::new(runtime, dwarf_parser, disasm);
    //dbg.repl()?;

    Ok(())
}
