// src/integration.rs
use crate::dwarfparser::DwarfParser;
use crate::wasmrt::WasmRuntime;
use crate::memory::MemoryLayout;
use crate::errors::RuntimeError;
use std::collections::HashMap;

pub struct Debugger {
    runtime: WasmRuntime,
    dwarf_parser: Option<DwarfParser>,
    memory_layout: Option<MemoryLayout>,
    symbol_table: HashMap<String, SymbolInfo>,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub symbol_type: SymbolType,
    pub dwarf_info: Option<DwarfSymbolInfo>,
}

#[derive(Debug, Clone)]
pub struct DwarfSymbolInfo {
    pub file: String,
    pub line: u32,
    pub type_name: String,
}

#[derive(Debug, Clone)]
pub enum SymbolType {
    Function,
    Global,
    Table,
    Memory,
}

impl Debugger {
    pub fn new(wasm_bytes: &[u8]) -> Result<Self, RuntimeError> {
        let runtime = WasmRuntime::new(wasm_bytes)?;
        
        // Versuche, DWARF-Informationen zu extrahieren
        let dwarf_parser = DwarfParser::from_wasm(wasm_bytes).ok();
        
        // Erstelle initiale Symboltabelle aus Exporten
        let symbol_table = Self::build_symbol_table(&runtime);
        
        Ok(Self {
            runtime,
            dwarf_parser,
            memory_layout: None,
            symbol_table,
        })
    }
    
    fn build_symbol_table(runtime: &WasmRuntime) -> HashMap<String, SymbolInfo> {
        let mut symbols = HashMap::new();
        
        // Funktionen zur Symboltabelle hinzufügen
        for (name, func) in &runtime.exported_functions {
            symbols.insert(name.clone(), SymbolInfo {
                name: name.clone(),
                address: 0, // Wird später mit DWARF-Info gefüllt
                size: 0,
                symbol_type: SymbolType::Function,
                dwarf_info: None,
            });
        }
        
        // Globals zur Symboltabelle hinzufügen
        for (name, global) in &runtime.exported_globals {
            symbols.insert(name.clone(), SymbolInfo {
                name: name.clone(),
                address: 0,
                size: 0,
                symbol_type: SymbolType::Global,
                dwarf_info: None,
            });
        }
        
        // Weitere Exporttypen hinzufügen...
        
        symbols
    }
    
    pub fn load_dwarf_info(&mut self) -> Result<(), RuntimeError> {
        if let Some(parser) = &self.dwarf_parser {
            // DWARF-Informationen extrahieren
            let dwarf_data = parser.extract_memory_layout()?;
            
            // Symboltabelle mit DWARF-Informationen anreichern
            for variable in &dwarf_data.variables {
                if let Some(symbol) = self.symbol_table.get_mut(&variable.name) {
                    symbol.address = variable.address;
                    symbol.size = variable.size;
                    // Weitere DWARF-Informationen hinzufügen
                }
            }
            
            // MemoryLayout erstellen
            let memory_snapshot = self.runtime.get_memory_snapshot();
            self.memory_layout = Some(MemoryLayout::new(&memory_snapshot, dwarf_data));
            
            Ok(())
        } else {
            Err(RuntimeError::DwarfInfoUnavailable)
        }
    }
    
    pub fn get_symbol_info(&self, name: &str) -> Option<&SymbolInfo> {
        self.symbol_table.get(name)
    }
    
    pub fn resolve_address(&self, address: u64) -> Option<&SymbolInfo> {
        self.symbol_table.values().find(|symbol| {
            symbol.address <= address && address < symbol.address + symbol.size
        })
    }
    
    pub fn call_function(&mut self, name: &str, args: &[i32]) -> Result<Vec<i32>, RuntimeError> {
        // Konvertiere i32-Argumente zu wasmer::Value
        let wasmer_args: Vec<wasmer::Value> = args.iter()
            .map(|&arg| wasmer::Value::I32(arg))
            .collect();
        
        self.runtime.call_function(name, &wasmer_args)
            .map(|results| {
                results.into_iter()
                    .filter_map(|val| if let wasmer::Value::I32(i) = val { Some(i) } else { None })
                    .collect()
            })
    }
    
    pub fn read_global(&mut self, name: &str) -> Result<i32, RuntimeError> {
        self.runtime.get_global_as_i32(name)
    }
    
    pub fn set_global(&mut self, name: &str, value: i32) -> Result<(), RuntimeError> {
        self.runtime.set_global_value(name, wasmer::Value::I32(value))
    }
    
    pub fn read_memory(&self, address: u64, size: usize) -> Result<Vec<u8>, RuntimeError> {
        if let Some(layout) = &self.memory_layout {
            layout.read_bytes(address, size)
        } else {
            self.runtime.read_memory(address, size)
        }
    }
    
    pub fn write_memory(&mut self, address: u64, data: &[u8]) -> Result<(), RuntimeError> {
        self.runtime.write_memory(address, data)
    }
}
