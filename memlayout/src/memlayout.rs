use byteorder::{ByteOrder, LittleEndian};
use std::collections::HashMap;
use crate::types::{DwarfData, FieldInfo, PrimitiveKind, TypedValue, TypeInfo, VariableInfo, VariableValue};
use crate::memsegments::*;
use crate::errors::*;

pub struct MemoryLayout {
    segments: Vec<MemorySegment>,
    memory: Vec<u8>,
    type_registry: HashMap<String, TypeInfo>,
    pub dwarf_data: DwarfData, // Neues Feld hinzugefügt
}

impl MemoryLayout {
    pub fn new(
        wasm_memory: &[u8],
        dwarf_data: &DwarfData
    ) -> Self {
        // Transform DWARF into memory segments
        let segments = dwarf_data.variables.iter().map(|var| {
            MemorySegment {
                start: var.address,
                size: var.size,
                name: var.name.clone(),
                segment_type: SegmentType::GlobalVariable,
                signed: false,
            }
        }).collect();

        Self {
            segments,
            memory: wasm_memory.to_vec(),
            type_registry: dwarf_data.types.clone(),
            dwarf_data: dwarf_data.clone(), // Speichere die dwarf_data
        }
    }
   
    fn get_variable(&self, name: &str) -> Result<&VariableInfo, MemoryError> {
        self
            .dwarf_data
            .variables
            .iter()
            .find(|v| v.name == name)
            .ok_or_else(|| MemoryError::VariableNotFound(name.to_string()))
    }

    pub fn read_variable(&self, name: &str) -> Result<VariableValue, MemoryError> {
        let var_info = self.get_variable(name)?;

        // Typinformation aus der Registry holen
        let type_info = self.dwarf_data.types.get(&var_info.type_name)
            .ok_or_else(|| MemoryError::InvalidTypeInfo(var_info.type_name.clone()))?;

        // Wert basierend auf dem Typ lesen
        let value = self.read_typed_value(var_info.address, type_info)?;
        
        // Rohbytes lesen
        let raw_bytes = self.read_bytes(var_info.address, var_info.size as usize)?;

        Ok(VariableValue {
            name: name.to_string(),
            address: var_info.address,
            type_name: var_info.type_name.clone(),
            value,
            size: var_info.size,
            raw_bytes,
        })
    }

    pub fn read_int(&self, addr: u64, size: u64, signed: bool) -> Result<i64, MemoryError> {
        let end = addr + size;

        if end > self.memory.len().try_into().unwrap() {
            return Err(MemoryError::InvalidAccess { address: addr as u64 });
        }

        let bytes = &self.memory[addr as usize..end as usize];

        match size {
            1 => Ok(if signed {
                bytes[0] as i8 as i64
            } else {
                bytes[0] as u64 as i64
            }),
            2 => Ok(if signed {
                LittleEndian::read_i16(bytes) as i64
            } else {
                LittleEndian::read_u16(bytes) as i64
            }),
            4 => Ok(if signed {
                LittleEndian::read_i32(bytes) as i64
            } else {
                LittleEndian::read_u32(bytes) as i64
            }),
            8 => Ok(if signed {
                LittleEndian::read_i64(bytes) as i64
            } else {
                LittleEndian::read_u64(bytes) as i64
            }),
            _ => Err(MemoryError::UnsupportedSize { size: size.try_into().unwrap() }),
        }
    }

    fn read_struct(
        &self,
        address: u64,
        fields: &HashMap<String, FieldInfo>,
    ) -> Result<TypedValue, MemoryError> {
        let mut field_values = HashMap::new();

        for(field_name, field_info) in fields {
            let field_address = address + field_info.offset;
            let field_value = self.read_typed_value(field_address, &field_info.type_info)?;
            field_values.insert(field_name.clone(), field_value);
        }

        Ok(TypedValue::Struct(field_values))
    }

    pub fn read_typed_value(
        &self, 
        address: u64,
        type_info: &TypeInfo,
    ) -> Result<TypedValue, MemoryError> {
        match type_info {
            TypeInfo::Primitive { size, kind } => {
                self.read_primitive(address, *size, kind)
            }
            TypeInfo::Struct { fields, .. } => {
                self.read_struct(address, fields)
            }
            TypeInfo::Array { element_type, count, .. } => {
                self.read_array(address, element_type, *count)
            }
            TypeInfo::Pointer { size, .. } => {
                let pointer_value = self.read_int(address, *size, false)?;
                Ok(TypedValue::Pointer(pointer_value as u64))
            }
            _ => Err(MemoryError::UnsupportedType(format!(
                "Unsupported type: {:?}",
                type_info
            ))),
        }
    }

    fn read_primitive(&self, address: u64, size: u64, kind: &PrimitiveKind) -> Result<TypedValue, MemoryError> {
        let size_usize = size as usize;
        
        match kind {
            PrimitiveKind::Int { signed } => {
                let value = self.read_int(address, size, *signed)?;
                Ok(TypedValue::Int(value))
            }
            PrimitiveKind::Float => {
                if size == 4 {
                    let bytes = self.read_bytes(address, 4)?;
                    let value = f32::from_le_bytes(bytes.try_into().unwrap());
                    Ok(TypedValue::Float(value as f64))
                } else if size == 8 {
                    let bytes = self.read_bytes(address, 8)?;
                    let value = f64::from_le_bytes(bytes.try_into().unwrap());
                    Ok(TypedValue::Float(value))
                } else {
                    Err(MemoryError::UnsupportedSize { size: size_usize })
                }
            }
            PrimitiveKind::Double => {
                let bytes = self.read_bytes(address, 8)?;
                let value = f64::from_le_bytes(bytes.try_into().unwrap());
                Ok(TypedValue::Float(value))
            }
            PrimitiveKind::Bool => {
                let value = self.read_int(address, 1, false)?;
                Ok(TypedValue::Bool(value != 0))
            }
            PrimitiveKind::Char => {
                let value = self.read_int(address, 1, false)?;
                Ok(TypedValue::Char(char::from_u32(value as u32).unwrap_or('\0')))
            }
            // Behandlung anderer primitiver Typen...
            PrimitiveKind::Address => {
                let value = self.read_int(address, size, false)?;
                Ok(TypedValue::Pointer(value as u64))
            }
            PrimitiveKind::Void => {
                Ok(TypedValue::Void)
            }
        }
    }

    fn read_array(
        &self, 
        address: u64,
        element_type: &TypeInfo,
        count: u32
    ) -> Result<TypedValue, MemoryError> {
        let element_size = element_type.get_size() as usize;
        let mut elements = Vec::new();

        for i in 0..count {
            let element_address = address + (i as u64 * element_size as u64);
            let element_value = self.read_typed_value(element_address, element_type)?;
            elements.push(element_value);
        }
        Ok(TypedValue::Array(elements))
    }

    pub fn read_bytes(&self, address: u64, size: usize) -> Result<Vec<u8>, MemoryError> {
        let addr = address as usize;
        let end = addr + size;
        
        if end > self.memory.len() {
            return Err(MemoryError::InvalidAccess { address });
        }
        
        Ok(self.memory[addr..end].to_vec())
    }

    pub fn generate_visualization(&self) -> Visualization {
        let mut segments = Vec::new();

        // Variablen als Segmente hinzufügen
        for var in &self.dwarf_data.variables {
            let (segment_type, color) = match var.type_name.as_str() {
                s if s.contains("int") => ("integer", "#FF6B6B"),
                s if s.contains("float") => ("float", "#4ECDC4"),
                s if s.contains("double") => ("double", "#45B7D1"),
                s if s.contains("char") => ("char", "#96CEB4"),
                s if s.contains("bool") => ("boolean", "#FECA57"),
                _ => ("unknown", "#778CA3"),
            };

            segments.push(VisualizationSegment {
                name: var.name.clone(),
                address: var.address,
                size: var.size,
                segment_type: segment_type.to_string(),
                color: color.to_string(),
            });
        }

        // Gesamtgröße berechnen
        let total_size = self.memory.len();

        Visualization {
            segments,
            total_size,
        }

    }

    // Hilfsmethode zur Ausgabe als JSON
    pub fn generate_visualization_json(&self) -> String {
        let visualization = self.generate_visualization();
        serde_json::to_string_pretty(&visualization).unwrap_or_else(|_| "{}".to_string())
    }
}
