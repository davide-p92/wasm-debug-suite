use wasmer::{Store, Module, Engine, Instance, Memory, MemoryType, Table, Global};//, Universal};
                                          
use wasmer_compiler::EngineBuilder;

//use wasmer::compiler::{Singlepass, Cranelift, LLVM};
//use wasmer_engine_universal::Universal;
//use wasmer::engine::Universal;
use wasmer_types::Type;
//use wasmer::CompilerConfig;

#[cfg(feature = "cranelift")]
use wasmer_compiler_cranelift::Cranelift;
#[cfg(feature = "singlepass")]
use wasmer_compiler_singlepass::Singlepass;
#[cfg(feature = "llvm")]
use wasmer_compiler_llvm::LLVM;

//use wasmer::sys::Cranelift;
use std::collections::HashMap;
use std::fmt::Write as _; // für write!()
use std::rc::Rc;

use wasmer_types::lib::std::cell::RefCell;
use crate::MemoryLayout;
use crate::DwarfParser;
use crate::types::VariableValue;
use crate::errors::*;

pub struct WasmRuntime<'a> {
    store: RefCell<Store>,
    instance: Instance,
    memory: Memory,
    exported_functions: HashMap<String, wasmer::Function>,
    exported_globals: HashMap<String, Global>,
    exported_tables: HashMap<String, Table>,
    exported_memories: HashMap<String, Memory>,
    fake_pc: usize,
    pub dwarf: Option<Rc<DwarfParser<'a>>>,
}

impl<'a> WasmRuntime<'a> {
    fn make_engine() -> Engine {
        #[cfg(feature = "cranelift")]
        {
            let compiler = Cranelift::default();
            let raw_engine = EngineBuilder::new(compiler).engine();
            return Engine::from(raw_engine);
        }

        #[cfg(feature = "singlepass")]
        {
            let compiler = Singlepass::default();
            let raw_engine = EngineBuilder::new(compiler).engine();
            return Engine::from(raw_engine);
        }

        // Fallback: wenn kein Feature gesetzt → Cranelift
        #[cfg(feature = "llvm")]
        {
            let compiler = LLVM::default();
            let raw_engine = EngineBuilder::new(compiler).engine();
            return Engine::from(raw_engine);
        }
        panic!("Kein Backend aktiviert — benutze --features cranelift | singlepass | llvm");
    }

    pub fn init_store() -> Store {
        let engine = Self::make_engine();
        Store::new(engine)
    }

    pub fn new(wasm_bytes: &[u8], dwarf: Option<Rc<DwarfParser<'a>>>) -> Result<Self, anyhow::Error> {
        // Store mit Cranelift Compiler erstellen
        //let engine = Universal::new(compiler).engine();
        let mut store = Self::init_store();// statt Store::default();

        // Modul aus Bytes erstellen
        let module = Module::new(&store, wasm_bytes)
            .map_err(|e| RuntimeError::InstanceCreation(e.to_string()))?;

        // Importe definieren
        let import_object = wasmer::imports! {};

        let instance = Instance::new(&mut store, &module, &import_object)
            .map_err(|e| RuntimeError::InstanceCreation(e.to_string()))?;
        /*let memory = instance.exports.get_memory("memory")
            .map_err(|e| RuntimeError::MemoryNotFound(format!("export 'memory' not found: {}", e)))?;
*/
        // 👉 Versuch: zuerst nach "memory" suchen
        let memory = instance
            .exports
            .get_memory("memory")
            .map(|m| m.clone()) // <- Referenz in besitzten Memory umwandeln
            .or_else(|_| {
                // Wenn kein "memory"-Export existiert, nimm die erste gefundene Memory
                instance.exports.iter()
                    .find_map(|(_, e)| {
                        if let wasmer::Extern::Memory(m) = e {
                            Some(m.clone())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| RuntimeError::MemoryNotFound("no exported memory found".into()))
            })?;
        // Exportierte Funktionen sammlen
        let mut exported_functions = HashMap::new();
        let mut exported_globals = HashMap::new();
        let mut exported_tables = HashMap::new();
        let mut exported_memories = HashMap::new();
        let mut default_memory: Option<wasmer::Memory> = None;

        for (name, export) in instance.exports.iter() {
            match export {
                wasmer::Extern::Function(func) => {
                    exported_functions.insert(name.to_string(), func.clone());
                }
                //wasmer::Extern::Function(func) => {
	        // Vereinfacht: Wir nehmen an, dass alle Funktionen () -> () sind
	            //if let Ok(typed_func) = func.typed::<(), ()>(&store) {
	                 //exported_functions.insert(name.to_string(), typed_func);
	         
                wasmer::Extern::Global(global) => {
                     exported_globals.insert(name.to_string(), global.clone());
                }
                wasmer::Extern::Table(table) => {
                     exported_tables.insert(name.to_string(), table.clone());
                }
                wasmer::Extern::Memory(mem) => {
                     if default_memory.is_none() {
                         default_memory = Some(mem.clone());
                     }
                     exported_memories.insert(name.to_string(), mem.clone());
                }
                wasmer::Extern::Tag(_) => {
                    eprintln!("Tag export '{}' not supported yet!", name);
                }
            	
            }

        }
    
        let memory = default_memory.ok_or_else(|| RuntimeError::MemoryNotFound("default memory".to_string()))?;

        Ok(Self { 
            store: store.into(), 
            instance, 
            memory: memory,
            exported_functions,
            exported_globals,
            exported_tables,
            exported_memories,
            fake_pc: 0,
            dwarf,
        })
    }

    pub fn call_init_functions(&mut self) -> Result<(), RuntimeError> {
        // Versuche, bekannte Initialisierungsfunktionen aufzurufen
        let init_functions = ["_start", "__wasm_call_ctors", "init", "initialize"];

        for func_name in init_functions.iter() {
            if self.exported_functions.contains_key(*func_name) {
                match self.call_function(*func_name, &[]) {
                    Ok(_) => println!("Successfully called init function: {}", func_name),
                    Err(e) => eprintln!("Failed to call init function {}: {}", func_name, e),
                }
            }
        }
        
        Ok(())
    }

    // Führt einen einzelnen Schritt in der WASM-Instanz aus.
    // Sucht nach einer Funktion `_step`, `step`, oder `main`
    // und führt sie aus, falls vorhanden.
    pub fn step_instruction(&mut self) -> anyhow::Result<()> {
        // Versuche, Instanz und Store zu bekommen

        // 1️⃣ Funktionsobjekt erst *außerhalb* des mutable borrow finden
        let func_opt: Option<(String, wasmer::Function)> = {
            //let mut store_ref = self.store.borrow_mut();
            let exports = self.instance.exports.clone();

            // Prüfen, ob Speicher vorhanden ist
            for (name, export) in exports.iter() {
                println!("Export: {}", name);
            }
            // Liste möglicher "step"-Funktionen (z. B. Debug-/Main-Einstiegspunkte)
            let candidates = ["_step", "step", "main"];
            let mut found = None;

            for name in candidates {
                if let Ok(func) = exports.get_function(name) {
                    //if let Some(func) = export.into_function() {
                    println!(">> Stepping into function '{}'", name);
                    // Kein Argument, kein Rückgabewert – universell sicher
                    //println!("✅ Executed '{}' ({} results)", name, results.len());
                    found = Some((name.to_string(), func.clone())); // Nur Wert zurückgeben, kein Ok((())
                    break;
                }
            }
            found
        };

        // 2️⃣ Wenn Funktion vorhanden: Store mutabel leihen, aufrufen
        if let Some((func_name, func)) = func_opt {
            {
                let mut store_ref = self.store.borrow_mut();
                match func.call(&mut *store_ref, &[]) {
                    Ok(_) => {
                        //let mem = self.get_memory_snapshot();
                        println!("'{}' executed successfully.", func_name);
                        // Speicherabbild zeigen
                        //println!("Memory (first 32 B): {:?}", &mem[..32.min(mem.len())]);
                        return Ok(());
                    }
                    Err(e) => eprintln!("Execution error: {}", e),
                }
            } // store_ref wird hier automatisch gedroppt
            let mem = self.get_memory_snapshot();
            println!("Memory (first 32 bytes): {:?}", &mem[..32.min(mem.len())]);
        } else {
            println!("No step-like function found. Simulating...");
            self.fake_pc = self.fake_pc.wrapping_add(1);
        }

        Ok(())
    }

    // Gibt einen formatierten Dump des linearen Speichers aus.
    // Optional mit Startadresse (`offset`) und Länge (`len`).
    pub fn dump_memory(&self, offset: usize, len: usize) -> anyhow::Result<String> {
        let store_ref = self.store.borrow();
        let view = self.memory.view(&*store_ref);
        let total_size: usize = view.data_size().try_into().unwrap();

        if offset >= total_size.try_into().unwrap() {
            return Err(anyhow::anyhow!("Offset 0x{:X} lays outside of memory!", offset));
        }

        let end = usize::min(offset + len, total_size);
        let mut out = String::new();

        writeln!(
            &mut out,
            "Memory dump [0x{:04X}..0x{:04X}] ({} bytes total, showing {}):",
            offset,
            end,
            total_size,
            end - offset
        )?;

        // Buffer for reading operation
        let mut buf = vec![0u8; 1];

        // In 16-Byte-Zeilen formatieren
        for base in (offset..end).step_by(16) {
            write!(&mut out, "0x{:04X}: ", base)?;
            for i in 0..16 {
                let addr = base + i;
                if addr < end {
                    view.read(addr as u64, &mut buf)?;
                    write!(&mut out, "{:02X} ", buf[0])?;
                } else {
                    write!(&mut out, "   ")?;
                }
            }

            // ASCII-Spalte
            write!(&mut out, " |")?;
            for i in 0..16 {
                let addr = base + i;
                if addr < end {
                    view.read(addr as u64, &mut buf)?;
                    let b = buf[0];
                    let c = if b.is_ascii_graphic() { b as char } else { '.' };
                    write!(&mut out, "{}", c)?;
                }
            }
            writeln!(&mut out, "|")?;
        }

        Ok(out)
    }
    
    pub fn read_memory(&self, address: u64, length: usize) -> Result<Vec<u8>, RuntimeError> {
        //let addr = address as usize;
        //let end = addr + length;
       
        let store_ref = self.store.borrow();
        // Größe des Speichers über den View ermitteln
        let view = self.memory.view(&*store_ref);
        //let memory_size = view.size().0 * 65536;
        let mut buffer = vec![0; length];
        view.read(address, &mut buffer)
            .map_err(|_| RuntimeError::InvalidMemoryAccess);
        Ok(buffer)
        /*if end > memory_size {
            return Err(RuntimeError::InvalidMemoryAccess);
        }
        
        let mut result = Vec::with_capacity(length);
        
        for i in addr..end {
            let cell = view.get(i)
                .ok_or(RuntimeError::InvalidMemoryAccess)?;
            result.push(cell.get());
        }
        
        Ok(result)*/
    }
    
    pub fn write_memory(&mut self, address: u64, data: &[u8]) -> Result<(), RuntimeError> {
        //let addr = address as usize;
        //let end = addr + data.len();
       
        let store_ref = self.store.borrow();
        let view = self.memory.view(&*store_ref);
        //let memory_size = view.size().0 * 65536;

        view.write(address, data)
            .map_err(|_| RuntimeError::InvalidMemoryAccess);
        //if end > memory_size {
            //return Err(RuntimeError::InvalidMemoryAccess);
        //}

        // Effizienteres Schreiben mit write
        /*unsafe {
            view.unchecked_write(addr, data)
                .map_err(|_| RuntimeError::InvalidMemoryAccess)?;
        } */

        Ok(())
    }
   
    pub fn resolve_symbol_address(&self, name: &str) -> Option<u64> {
        // 1️⃣ DWARF-Suche (die eleganteste Variante)
        if let Some(ref dwarf_rc) = self.dwarf {
            if let Some(addr) = dwarf_rc.lookup_variable_address(name) {
                return Some(addr);
            }
        }

        // 2️⃣ Fallback: bekannte globale Exporte
        if let Ok(global) = self.instance.exports.get_global(name) {
            let mut store_ref = self.store.borrow_mut();
            if let wasmer::Value::I32(val) = global.get(&mut *store_ref) {
                return Some(val as u64);
            }
        }

        // 3️⃣ Fallback: nichts gefunden
        None
    }

    pub fn call_function(&mut self, name: &str, params: &[wasmer::Value]) -> Result<Box<[wasmer::Value]>, RuntimeError> {
        let func = self.exported_functions.get(name)
            .ok_or_else(|| RuntimeError::FunctionNotFound(name.to_string()))?;

        let mut store_ref = self.store.borrow_mut();
        func.call(&mut *store_ref, &params)
            .map_err(|e| RuntimeError::FunctionCall(name.to_string(), e.to_string()))
    }

    pub fn get_memory_snapshot(&self) -> Vec<u8> {
    let store_ref = self.store.borrow();
    self.memory.view(&*store_ref).copy_to_vec().unwrap_or_else(|_| vec![])
}

    // Globals
    pub fn get_global_value(&mut self, name: &str) -> Result<wasmer::Value, RuntimeError> {
        let global = self.exported_globals.get(name)
            .ok_or_else(|| RuntimeError::GlobalNotFound(name.to_string()))?;
        
        //global.set(&mut self.store, value)
            //.map_err(|e| RuntimeError::GlobalSetFailed(name.to_string(), e.to_string()))
        let mut store_ref = self.store.borrow_mut();
        Ok(global.get(&mut *store_ref))
    }

    pub fn set_global_value(&self, name: &str, value: wasmer::Value) -> Result<(), RuntimeError> {
        let global = self.exported_globals.get(name)
            .ok_or_else(|| RuntimeError::GlobalNotFound(name.to_string()))?;

        let mut store_ref = self.store.borrow_mut();
        global.set(&mut *store_ref, value)
            .map_err(|e| RuntimeError::GlobalSetFailed(name.to_string(), e.to_string()))
    }

    // Tables
    pub fn get_table_size(&self, name: &str) -> Result<u32, RuntimeError> {
        let table = self.exported_tables.get(name)
            .ok_or_else(|| RuntimeError::TableNotFound(name.to_string()))?;
    
        let store_ref = self.store.borrow();
        Ok(table.size(&*store_ref))
    }

    pub fn get_table_element(&self, name: &str, index: u32) -> Result<Option<wasmer::Value>, RuntimeError> {
        let table = self.exported_tables.get(name)
            .ok_or_else(|| RuntimeError::TableNotFound(name.to_string()))?;

        // wasmer-API erwartet &mut impl AsStoreMut
        let mut store_ref = self.store.borrow_mut();
        Ok(table.get(&mut *store_ref, index))
        /*table.get(&self.store, index)
            .ok_or_else(|| RuntimeError::TableAccessFailed(
                 name.to_string(), 
                 format!("Element at index {} not found!", index)
            ))*/
    }

    // Memories (zusätzliche Memories)
    pub fn get_additional_memory(&self, name: &str) -> Result<&Memory, RuntimeError> {
        self.exported_memories.get(name)
            .ok_or_else(|| RuntimeError::MemoryNotFound(name.to_string()))
    }

    // Hilfsmethoden für häufige Operationen
    pub fn get_global_as_i32(&mut self, name: &str) -> Result<i32, RuntimeError> {
        let value = self.get_global_value(name)?;
        if let wasmer::Value::I32(val) = value {
            Ok(val)
        } else {
            Err(RuntimeError::TypeMismatch {
                expected: "i32".to_string(),
                found: format!("{:?}", value),
            })
        }
    }

    pub fn get_global_as_i64(&mut self, name: &str) -> Result<i64, RuntimeError> {
        let value = self.get_global_value(name)?;
        if let wasmer::Value::I64(val) = value {
            Ok(val)
        } else {
            Err(RuntimeError::TypeMismatch {
                expected: "i64".to_string(),
                found: format!("{:?}", value),
            })
        }
    }
    
/*    pub fn get_global_value(&self, name: &str) -> Option<i32> {
        // Vereinfachte Implementierung für Globals
        // In einer echten Implementierung müssten Sie den Typ des Globals berücksichtigen
        self.instance.exports.get_global(name)
            .ok()
            .and_then(|global| {
                let mut store = self.store.borrow_mut(); // Mutable borrow
                let value = global.get(&mut *store);
                if let wasmer::Value::I32(val) = value {
                    Some(val)
                } else {
                    None
                }
            })
        }
    }*/
}
/*
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Failed to create WASM instance: {0}")]
    InstanceCreation(String),

    #[error("Memory not found in WASM module")]
    MemoryNotFound,

    #[error("Function '{0}' not found")]
    FunctionNotFound(String),

    #[error("Failed to call function '{0}': {1}")]
    FunctionCall(String, String),

    #[error("Invalid memory access")]
    InvalidMemoryAccess,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unknown runtime error: {0}")]
    Unknown(String),
}
*/


fn inspect_variable(
    runtime: &WasmRuntime,
    layout: &MemoryLayout,
    var_name: &str,
) -> Result<VariableValue, anyhow::Error> {
    let var_info = layout.read_variable(var_name)?;
    let memory = runtime.get_memory_snapshot();

    // Aktualisiertes Layout mit aktuellen Speicherdaten
    let current_layout = MemoryLayout::new(&memory, &layout.dwarf_data.clone());

    Ok(current_layout.read_variable(var_name)?)
}


