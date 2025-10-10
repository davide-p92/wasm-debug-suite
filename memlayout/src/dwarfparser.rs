use gimli::{Dwarf, SectionId, EndianSlice, LittleEndian, Unit, DebuggingInformationEntry, AttributeValue, Expression, EvaluationResult, Location, ValueType, Value,};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use wasmparser::{Parser, Payload};
use crate::memlayout::*;
use crate::errors::*;
use crate::types::*;
use crate::wasmrt::WasmRuntime;

pub struct DwarfParser<'a> {
    dwarf: Dwarf<EndianSlice<'a, LittleEndian>>,
    sections: std::rc::Rc<HashMap<gimli::SectionId, Vec<u8>>>, // Ownership für from_wasm Funktion behalten
    pub vars: RefCell<HashMap<String, u64>>,
}

impl<'a> DwarfParser<'a> {
    pub fn from_wasm(wasm_bytes: &'a[u8]) -> Result<Self, DwarfError> {
        let sections = Rc::new(Self::parse_dwarf_sections(wasm_bytes)?);
            //.map_err(|e| DwarfError::ParseError(e.to_string()))?;
        
        // Wir müssen die sections in der Struct behalten, damit sie lange genug leben
        //let sections_rc = std::rc::Rc::new(sections);
        
        let sections_clone = Rc::clone(&sections);
        
        // Dwarf-Instanz erstellen
        let dwarf = Dwarf::load(move |id| {
            //Ok(sections
            if let Some(s) = sections_clone.get(&id) {
		let leaked: &'static [u8] = Box::leak(s.clone().into_boxed_slice());
		Ok(EndianSlice::new(leaked, LittleEndian))
	    } else {
	       // Err(gimli::Error::MissingUnitDie)
	        // statt Error einfach leere Sektion zurückgeben
                Ok(EndianSlice::new(&[], LittleEndian))
            }
                //.get(&id) // Ok(sections.get(id))
                //.map(|s| EndianSlice::new(s, LittleEndian)))
                //.ok_or(gimli::Error::MissingUnitDie)
        }) // ? wandelt gimli::Error -> DwarfError
        .map_err(DwarfError::GimliError)?;
    
        Ok(Self { 
            dwarf, 
            sections,
            vars: RefCell::new(HashMap::new()),
    })
}

fn section_id_from_name(name: &str) -> Option<gimli::SectionId> {
    match name {
        ".debug_abbrev" => Some(gimli::SectionId::DebugAbbrev),
        ".debug_info" => Some(gimli::SectionId::DebugInfo),
        ".debug_str" => Some(gimli::SectionId::DebugStr),
        ".debug_line" => Some(gimli::SectionId::DebugLine),
        ".debug_loc" => Some(gimli::SectionId::DebugLoc),
        ".debug_ranges" => Some(gimli::SectionId::DebugRanges),
        ".debug_str_offsets" => Some(gimli::SectionId::DebugStrOffsets),
        ".debug_types" => Some(gimli::SectionId::DebugTypes),
        // nach Bedarf weitere Sektionen hinzufügen
        _ => None,
    }
}

fn parse_dwarf_sections(wasm_bytes: &[u8]) -> Result<HashMap<SectionId, Vec<u8>>, DwarfError> {
    let mut sections = HashMap::new();

    let parser = Parser::new(0);
    for payload in parser.parse_all(wasm_bytes) {
    let payload = payload?;//.map_err(|e| DwarfError::ParseError(e.to_string()))?;
        match payload {
            Payload::CustomSection(section) => {
                let name = section.name();

                // Wir wollen nur Debug-Sektionen
                if let Some(id) = Self::section_id_from_name(name) {
                    sections.insert(id, section.data().to_vec());
                }
            }
            _ => {}
        }
    }

    Ok(sections)
}

pub fn get_source_location(&self, addr: u64) -> Option<(String, u64)> {
    let dwarf = &self.dwarf;

    // DWARF Line Section laden
    let mut iter = dwarf.units();
    while let Some(header) = iter.next().ok()? {
        let unit = dwarf.unit(header).ok()?;
        
        let line_program = unit.line_program.as_ref()?;
        let (program, sequences) = line_program.clone().sequences().ok()?;

        for seq in &sequences {
            // Prüfen, ob Adresse in diesem Line-Sequence liegt
            if addr < seq.start || addr >= seq.end {
                continue;
            }
            // Durch Line Rows iterieren
            let mut rows = program.resume_from(seq);
            while let Ok(Some((header, row))) = rows.next_row() {
                if row.address() == addr {
                    if let Some(file) = row.file(header) {
                    // Datei ermitteln
                    match file.path_name() {
                        AttributeValue::String(ref s) => {
                            let file_name = String::from_utf8_lossy(s.slice()).into_owned();
                            println!("File: {}", file_name);
                            
                            // Optional: Verzeichnis ermitteln und voranstellen
                            let full_path = if let Some(dir_attr) = file.directory(header) {
                                if let Ok(dir_bytes) = dwarf.attr_string(&unit, dir_attr) {
                                    let dir = String::from_utf8_lossy(&dir_bytes).into_owned();
                                    if dir.is_empty() {
                                        file_name.clone()
                                    } else {
                                        format!("{}/{}", dir, file_name)
                                    }
                                } else {
                                    file_name.clone()
                                }
                            } else {
                                file_name.clone()
                            };

                            let line = row.line().map(|l| l.get()).unwrap_or(0);
                            return Some((file_name, line));
                            
                        }
                        AttributeValue::DebugStrRef(offset) => {
                            if let Ok(name) = dwarf.attr_string(&unit, gimli::AttributeValue::DebugStrRef(offset)) {
                                let file_name = name.to_string_lossy().into_owned();
                                println!("File (ref): {}", file_name);
                                let line = row.line().map(|l| l.get()).unwrap_or(0);
                                return Some((file_name, line));
                            }
                        }
                        _ => {
                            println!("Unknown file name format in DWARF info");
                        }
                    }
                }
            }
        }
    }
    }
    None
}

pub fn lookup_variable_address(&self, name: &str) -> Option<u64> {
    self.vars.borrow().get(name).cloned()
    }

    pub fn extract_memory_layout(&self, runtime: &WasmRuntime) -> Result<DwarfData, DwarfError> { // Statt Result<MemoryLayout, DwarfError>
        let mut variables = Vec::new();
        let mut type_registry = HashMap::new();
        // 1. Parse CUs
        // Iteriere über alle CUs
        let mut units = self.dwarf.units();
        while let Some(unit_header) = units.next().map_err(DwarfError::GimliError)? {
            // Unit aus UnitHeader erstellen
            let unit = self.dwarf.unit(unit_header).map_err(DwarfError::GimliError)?;

            // Abkürzungen für diese Unit laden
            /*let abbrevs = unit.abbreviations(&self.dwarf.debug_abbrev)
                .map_err(DwarfError::GimliError)?;*/

            // Einträge mit den Abkürzungen laden
            let mut entries = unit.entries();

            // DFS durch alle Debug Informationen
            while let Ok(Some((depth, entry))) = entries.next_dfs().map_err(DwarfError::GimliError) {
                self.process_die(entry, runtime, &unit, &mut variables, &mut type_registry);
            }
        }
        
        Ok(DwarfData { 
            variables, 
            types: type_registry 
        })
    }
    
    fn process_die(
        &self,
        die: &DebuggingInformationEntry<EndianSlice<LittleEndian>>,
        runtime: &WasmRuntime,
        unit: &Unit<EndianSlice<LittleEndian>>,
        variables: &mut Vec<VariableInfo>,
        types: &mut HashMap<String, TypeInfo>,
    ) -> Result<(), DwarfError> {
        match die.tag() {
            gimli::DW_TAG_variable => self.process_variable(runtime, die, unit, variables)?,
            gimli::DW_TAG_structure_type => self.process_struct(die, unit, types)?,
            gimli::DW_TAG_base_type => self.process_base_type(die, unit, types)?,
            // Weitere Tags verarbeiten (function, array, etc.)
            // 
            _ => {}

        }
        Ok(())
    }

    // 2. Extract vars & locs
    fn process_variable(
        &self,
        runtime: &WasmRuntime,
        die: &DebuggingInformationEntry<EndianSlice<LittleEndian>>,
        unit: &Unit<EndianSlice<LittleEndian>>,
        variables: &mut Vec<VariableInfo>,
    ) -> Result<(), DwarfError> {
        let name_attr = die.attr(gimli::DW_AT_name)?;
        let name = if let Some(name_attr) = name_attr { //attrs.find(gimli::DW_AT_name) {
            //if let Some(name) = name_attr.string_value(&debug_str) {
                //let name_str = String::from_utf8_lossy(name.slice()).to_string();
                let name_str = name_attr.string_value(&self.dwarf.debug_str)
                    .ok_or_else(|| DwarfError::MissingInfo("Name string value not found".to_string()))?;
                /*let name = */name_str.to_string()
                    .map_err(|e| DwarfError::ParseError(e.to_string()))?
            } else {
               return Ok(());
            };
        //}
        //let name = Some(name) = name_attr {
            //name.string_value(&self.dwarf.debug_str)?.to_string()?
            /*let name_str = name.string_value(&self.dwarf.debug_str)
                .ok_or_else(|| DwarfError::MissingInfo("Name string value not found".to_string()))?;
            let name = name_str.to_string()
                .map_err(|e| DwarfError::ParseError(e.to_string()))?;
        } else {
            return Ok(());
        };*/

        // 3. Map to mem segments
        // Addresse aus Location-Expression extrahieren
        let address = match die.attr(gimli::DW_AT_location)? {
            Some(attr) => { 
                if let AttributeValue::Exprloc(expr) = attr.value() {
                    self.evaluate_location(runtime, &expr, unit.encoding())?
                } else {
                    0
                }
            }
            None => 0,
        };

        // Typinformation extrahieren
        let (type_name, size) = if let Some(type_ref) = die.attr(gimli::DW_AT_type)? {
            self.resolve_type(type_ref.value(), unit)?
        } else {
            ("<unknown>".to_string(), 0)
        };

        variables.push(VariableInfo {
            name: name.to_string(),
            address: address,
            type_name: type_name,
            size: size,
        });

        Ok(())
    }

    fn process_struct(
        &self,
        die: &DebuggingInformationEntry<EndianSlice<LittleEndian>>,
        unit: &Unit<EndianSlice<LittleEndian>>,
        types: &mut HashMap<String, TypeInfo>,
    ) -> Result<(), DwarfError> {
        // Implementierung für Strukturtypen
        // Namen der Struktur extrahieren
        let name_attr = die.attr(gimli::DW_AT_name)?
            .ok_or_else(|| DwarfError::MissingInfo("Struct name not found".to_string()))?;
        
        let name_str = name_attr.string_value(&self.dwarf.debug_str)
            .ok_or_else(|| DwarfError::MissingInfo("Name string value not found".to_string()))?;
        let name = name_str.to_string()
            .map_err(|e| DwarfError::ParseError(e.to_string()))?;

        // Größe der Struktur extrahieren
        let size = die.attr(gimli::DW_AT_byte_size)?
            .and_then(|attr| attr.udata_value())
            .unwrap_or(0);

        // Felder der Struktur sammeln
        let mut fields = HashMap::new();
        //let mut entries = die.units().next()
        let mut units = self.dwarf.units();

        while let Some(unit_header) = units.next().map_err(DwarfError::GimliError)? {

            //.expect("Soll zumindest eine CU haben")
            //.expect("And soll es ok parsen");
            let unit = self.dwarf.unit(unit_header)
            	.map_err(DwarfError::GimliError)?;

	    /*let abbrevs = unit.abbreviations(&self.dwarf.debug_abbrev)
	        .map_err(DwarfError::GimliError)?;*/

	    // Hole die erste Entry aus der CU
	    let mut cursor = unit.entries();
	    if let Some((_, entry)) = cursor.next_dfs()
	        .map_err(DwarfError::GimliError)? {

	        let mut attrs = entry.attrs();
	    
	        while let Some(attr) = attrs.next().unwrap() {
	            if entry.tag() == gimli::DW_TAG_member {
	                let field_name_attr = entry.attr(gimli::DW_AT_name)?;
	                let field_name = if let Some(name) = field_name_attr {
	                    let name_str = name.string_value(&self.dwarf.debug_str)
	                        .ok_or_else(|| DwarfError::MissingInfo("String value not found".to_string()))?;
	                    name_str.to_string()
	                        .map_err(|e| DwarfError::ParseError(e.to_string()))?
	                } else {
	                    continue; // Überspringen von Feldern ohne Namen
	                };

	                // Feld-Offset extrahieren
	                let offset = entry.attr(gimli::DW_AT_data_member_location)?
	                    .and_then(|attr| attr.udata_value())
	                    .unwrap_or(0);

	                // Feld-Typ extrahieren
	                let (type_name, field_size) = if let Some(type_ref) = entry.attr(gimli::DW_AT_type)? {
	                    self.resolve_type(type_ref.value(), &unit)?
	                } else {
	                    ("<unknown>".to_string(), 0)
	                };

	                // FieldInfo erstellen und hinzufügen
	                let field_info = FieldInfo {
	                    name: field_name.to_string(),
	                    offset,
	                    type_info: types.get(&type_name).cloned().unwrap_or(TypeInfo::Unknown),
	                };

	                fields.insert(field_info.name.clone(), field_info);
	            }
	        }
	    }
        }
        // TypeInfo für die Struktur erstellen und zur Registry hinzufügen
        let type_info = TypeInfo::Struct {
            size: size,
            fields: fields,
            name: name.to_string().clone(),
        };

        types.insert(name.to_string(), type_info);

        Ok(())

    }

    fn process_base_type(
        &self,
        die: &DebuggingInformationEntry<EndianSlice<LittleEndian>>,
        unit: &Unit<EndianSlice<LittleEndian>>,
        types: &mut HashMap<String, TypeInfo>,
    ) -> Result<(), DwarfError> {
        // Implementierung für Basistypen
        // Namen des Basistyps extrahieren
        let name_attr = die.attr(gimli::DW_AT_name)?
            .ok_or_else(|| DwarfError::MissingInfo("Base type name not found".to_string()))?;
        
        let name = name_attr
       	    .string_value(&self.dwarf.debug_str)
       	    .ok_or_else(|| DwarfError::MissingInfo("Name string value not found".to_string()))?
       	    .to_string()
       	    .map_err(|e| DwarfError::ParseError(e.to_string()))?;
        
        let name_str = match name_attr.string_value(&self.dwarf.debug_str) {
            Some(s) => s.to_string()
                .map_err(|e| DwarfError::ParseError(e.to_string()))?,
            None => return Ok(()),
        };

        // Größe des Basistyps extrahieren
        let size = die.attr(gimli::DW_AT_byte_size)?
            .and_then(|attr| attr.udata_value())
            .unwrap_or(0);

        // Encoding (Vorzeichenbehaftet/Vorzeichenlos) extrahieren
        let encoding = die.attr(gimli::DW_AT_encoding)?
            .and_then(|attr| attr.udata_value())
            .map(|val| gimli::DwAte(val as u8))
            .unwrap_or(gimli::DW_ATE_signed); // Fallback

        // Art des primitiven Typs bestimmen
        let kind = match encoding {
            gimli::DW_ATE_signed | gimli::DW_ATE_signed_char => PrimitiveKind::Int { signed: true },
            gimli::DW_ATE_unsigned | gimli::DW_ATE_unsigned_char => PrimitiveKind::Int { signed: false },
            gimli::DW_ATE_float => PrimitiveKind::Float,
            gimli::DW_ATE_boolean => PrimitiveKind::Bool,
            _ => PrimitiveKind::Int { signed: true }, // Default
        };

        // TypeInfo für den Basistyp erstellen und zur Registry hinzufügen
        let type_info = TypeInfo::Primitive { size, kind };
        types.insert(name.to_string().clone(), type_info);

        Ok(())
    }

    fn evaluate_location(
        &self,
        runtime: &WasmRuntime,
        expr: &gimli::Expression<EndianSlice<LittleEndian>>,
        encoding: gimli::Encoding
    ) -> Result<u64, DwarfError> {
        let mut eval = expr.evaluation(encoding);
        let mut result = eval.evaluate().map_err(DwarfError::GimliError)?;

        //while let Some(operation) = eval.next()? {
        // Evaluation fortsetzen, bis sie abgeschlossen ist
        loop {
            match result {
                gimli::EvaluationResult::Complete => break,
                gimli::EvaluationResult::RequiresRegister { register, base_type } => {
                    // Registerwert aus dem Ausführungskontext holen
                    let register_value = self.get_register_value(runtime, register, base_type)
                        .map_err(|e| DwarfError::RegisterUnavailable(register, e.to_string()))?;
                    result = eval.resume_with_register(gimli::Value::Generic(register_value))
                        .map_err(DwarfError::GimliError)?; 
                }
                gimli::EvaluationResult::RequiresFrameBase => {
                    // Frame-Base-Adresse aus dem Ausführungskontext holen
                    let frame_base = self.get_frame_base(runtime)
                        .map_err(|e| DwarfError::FrameBaseUnavailable(e.to_string()))?;
                    result = eval.resume_with_frame_base(frame_base)
                        .map_err(DwarfError::GimliError)?;
                }
                gimli::EvaluationResult::RequiresRelocatedAddress(offset) => {
                    // Relozierte Adresse berechnen
                    let relocated_address = self.relocate_address(runtime, offset)
                        .map_err(|e| DwarfError::RelocationError(offset, e.to_string()))?;
                    result = eval.resume_with_relocated_address(relocated_address)
                        .map_err(DwarfError::GimliError)?;
                }
                
                gimli::EvaluationResult::RequiresMemory { address, size, .. } => {
                    // Speicherinhalt lesen
                    let start = address as usize;
                    let end = start + size as usize;
                    let memory_contents = self.read_memory(runtime, address, size as u64)?;
                    if end > memory_contents.len() {
                        return Err(DwarfError::MemoryReadError(address, size.into(), "out of bounds".to_string()));
                    }
                    let bytes = &memory_contents[start..end];

                    // Die gelesenen Bytes in u64 packen
                    let mut buf = [0u8; 8];
                    buf[..bytes.len()].copy_from_slice(bytes);

                    // Wert bauen (DWARF behandelt das als generischen Wert)
                    let value = Value::Generic(u64::from_le_bytes(buf));

                    // Evaluation mit dem gelesenen Wert fortsetzen
                    result = eval.resume_with_memory(value)?;

                    /*let memory_contents = self.read_memory(address, size as u64)
                        .map_err(|e| DwarfError::MemoryReadError(address, size.into(), e.to_string()))?;
                    result = eval.resume_with_memory(memory_contents)
                        .map_err(DwarfError::GimliError)?;*/
                }
                
                gimli::EvaluationResult::RequiresEntryValue (value_type) => {
                    // Benötigten Wert beschaffen, eine Referenz auf die Expression Übergeben
                    let value = self.evaluate_entry_value(&value_type)
                        .map_err(|e| DwarfError::EntryValueEvaluationError(e.to_string()))?;
                    result = eval.resume_with_entry_value(gimli::Value::Generic(value))
                        .map_err(DwarfError::GimliError)?;
                }
                
                gimli::EvaluationResult::RequiresTls(_) => {
                    // TLS-Wert beschaffen
                    let tls_value = self.get_tls_value(runtime)
                        .map_err(|e| DwarfError::TlsUnavailable(e.to_string()))?;
                    result = eval.resume_with_tls(tls_value)
                        .map_err(DwarfError::GimliError)?;
                }            
                    // Weitere Fälle nach Bedarf behandeln
                other => {
                    return Err(DwarfError::UnsupportedExpression(
                        format!("Unsupported evaluation result: {:?}", other)
                    ));
                }
            }
        }
        // Ergebnis auswerten
        let pieces = eval.result();
        for piece in pieces {
            /*let addr = */if let gimli::read::Location::Address { address } = piece.location { 
                return Ok(address);
            } else {
                return Err(DwarfError::UnsupportedLocation(format!("Unsupported location: {:?}", piece.location)));
            };
        }
        Err(DwarfError::InvalidLocation)
    }

    fn evaluate_entry_value(&self, expr: &gimli::Expression<EndianSlice<LittleEndian>>) -> Result<u64, Box<dyn std::error::Error>> {
        // Implementierung zur Auswertung des DWARF-Ausdrucks
        // am Einstiegspunkt der aktuellen Funktion
        // Dies erfordert Zugriff auf Registerwerte, Speicherinhalt, etc.
        unimplemented!()
    }



    fn resolve_type(
        &self,
        attr_value: AttributeValue<EndianSlice<LittleEndian>>,
        unit: &Unit<EndianSlice<LittleEndian>>,
    ) -> Result<(String, u64), DwarfError> {
        if let AttributeValue::UnitRef(offset) = attr_value {
            let type_die = unit.entry(offset)?;
            let name = type_die.attr(gimli::DW_AT_name)?
                .and_then(|attr| attr.string_value(&self.dwarf.debug_str))
                .map(|s| s.to_string().unwrap_or_else(|_| "<unnamed>"))
                .unwrap_or_else(|| "<unnamed>");
            
            let size = type_die.attr(gimli::DW_AT_byte_size)?
                .and_then(|attr| attr.udata_value())
                .unwrap_or(0);

            Ok((name.to_string(), size))
        } else {
            Err(DwarfError::TypeResolutionFailed(format!(
                        "Expected UnitRef attribute value, got {:?}",
                        attr_value)))
        }
    }

    // Hilfsmethoden, die Sie implementieren müssten

    fn get_register_value(&self, runtime: &WasmRuntime, register: gimli::Register, base_type: gimli::UnitOffset) -> Result<u64, Box<dyn std::error::Error>> {
        // Registerwert aus dem Ausführungskontext holen
        // Diese Implementierung hängt von Ihrer spezifischen Laufzeitumgebung ab
        // Wasmer kennt keine CPU-Register, deshalb Dummy-Wert.
        // Später könntest du hier Debug-Infos oder spezielle globale Variablen abfragen.
        Ok(0)
    }

    fn get_frame_base(&self, runtime: &WasmRuntime,) -> Result<u64, Box<dyn std::error::Error>> {
        // Frame-Base-Adresse berechnen
        // In WebAssembly gibt es keinen klassischen Stack-Frame wie in nativen Prozessen.
        // Du kannst hier ggf. 0 oder die Basis einer globalen Variable zurückgeben.
        Ok(0)
    }

    fn relocate_address(&self, runtime: &WasmRuntime, offset: u64) -> Result<u64, Box<dyn std::error::Error>> {
        // Adresse relozieren
        // Falls du ein Mapping von DWARF-Adressen → Wasm-Memory brauchst, kannst du das hier implementieren.
        Ok(offset)
    }

    fn read_memory(
    	&self, 
    	runtime: &WasmRuntime, 
    	address: u64, 
    	size: u64
    ) -> Result<Vec<u8>, DwarfError> {
        // Speicherinhalt lesen
        let bytes = runtime.read_memory(address, size as usize)?;
        Ok(bytes)
    }

    fn get_value(&self, value_type: gimli::ValueType) -> Result<u64, Box<dyn std::error::Error>> {
        // Benötigten Wert beschaffen
        Ok(0)
    }

    fn get_tls_value(&self, runtime: &WasmRuntime) -> Result<u64, Box<dyn std::error::Error>> {
        // TLS-Wert beschaffen
        // WebAssembly kennt standardmäßig kein TLS.
        Ok(0)
    }
}
