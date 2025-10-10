use std::collections::HashMap;
use serde::{Serialize, Deserialize};

// Data structs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableInfo {
    pub name: String,
    pub address: u64,
    pub type_name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypedValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    Struct(HashMap<String, TypedValue>),
    Array(Vec<TypedValue>),
    Pointer(u64),
    Void,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableValue {
    pub name: String,
    pub address: u64,
    pub type_name: String,
    pub value: TypedValue,
    pub size: u64,
    pub raw_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DwarfData {
    pub variables: Vec<VariableInfo>,
    pub types: HashMap<String, TypeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeInfo {
    Primitive {
        size: u64,
        kind: PrimitiveKind,
    },
    Struct {
        size: u64,
        fields: HashMap<String, FieldInfo>,
        name: String,
    },
    Array {
        element_type: Box<TypeInfo>,
        count: u32,
        size: u64,
    },
    Pointer {
        pointed_type: Box<TypeInfo>,
        size: u64,
    },
    Union {
        size: u64,
        variants: HashMap<String, TypeInfo>,
        name: String,
    },
    Enum {
        size: u64,
        base_type: Box<TypeInfo>,
        values: HashMap<String, i64>,
        name: String,
    },
    Function {
        return_type: Box<TypeInfo>,
        parameters: Vec<TypeInfo>,
    },
    Void,
    Unknown,
}

// Hilfsfunktionen für TypeInfo
impl TypeInfo {
    pub fn new_primitive(size: u64, kind: PrimitiveKind) -> Self {
        TypeInfo::Primitive { size, kind }
    }
    
    pub fn new_struct(name: String, size: u64, fields: HashMap<String, FieldInfo>) -> Self {
        TypeInfo::Struct { name, size, fields }
    }
    
    pub fn is_primitive(&self) -> bool {
        matches!(self, TypeInfo::Primitive { .. })
    }
    
    pub fn is_struct(&self) -> bool {
        matches!(self, TypeInfo::Struct { .. })
    }
    
    pub fn get_size(&self) -> u64 {
        match self {
            TypeInfo::Primitive { size, .. } => *size,
            TypeInfo::Struct { size, .. } => *size,
            TypeInfo::Array { size, .. } => *size,
            TypeInfo::Pointer { size, .. } => *size,
            TypeInfo::Union { size, .. } => *size,
            TypeInfo::Enum { size, .. } => *size,
            TypeInfo::Void => 0,
            TypeInfo::Unknown => 0,
            TypeInfo::Function { .. } => 0,  // Funktionen haben keine Größe im Speicher
        }
    }

    pub fn get_kind(&self) -> TypeKind {
        match self {
            TypeInfo::Primitive { kind, .. } => TypeKind::Primitive(kind.clone()),
            TypeInfo::Struct { .. } => TypeKind::Struct,
            TypeInfo::Array { .. } => TypeKind::Array,
            TypeInfo::Pointer { .. } => TypeKind::Pointer,
            TypeInfo::Union { .. } => TypeKind::Union,
            TypeInfo::Enum { .. } => TypeKind::Enum,
            TypeInfo::Function { .. } => TypeKind::Function,
            TypeInfo::Void => TypeKind::Void,
            TypeInfo::Unknown => TypeKind::Unknown,
        }
    }

    // Vordefinierte primitive Typen
    pub fn int8() -> Self {
        TypeInfo::new_primitive(1, PrimitiveKind::Int { signed: true })
    }
    
    pub fn uint8() -> Self {
        TypeInfo::new_primitive(1, PrimitiveKind::Int { signed: false })
    }
    
    pub fn int16() -> Self {
        TypeInfo::new_primitive(2, PrimitiveKind::Int { signed: true })
    }
    
    pub fn uint16() -> Self {
        TypeInfo::new_primitive(2, PrimitiveKind::Int { signed: false })
    }
    
    pub fn int32() -> Self {
        TypeInfo::new_primitive(4, PrimitiveKind::Int { signed: true })
    }
    
    pub fn uint32() -> Self {
        TypeInfo::new_primitive(4, PrimitiveKind::Int { signed: false })
    }
    
    pub fn int64() -> Self {
        TypeInfo::new_primitive(8, PrimitiveKind::Int { signed: true })
    }
    
    pub fn uint64() -> Self {
        TypeInfo::new_primitive(8, PrimitiveKind::Int { signed: false })
    }
    
    pub fn float() -> Self {
        TypeInfo::new_primitive(4, PrimitiveKind::Float)
    }
    
    pub fn double() -> Self {
        TypeInfo::new_primitive(8, PrimitiveKind::Double)
    }
    
    pub fn bool() -> Self {
        TypeInfo::new_primitive(1, PrimitiveKind::Bool)
    }
    
    pub fn char() -> Self {
        TypeInfo::new_primitive(1, PrimitiveKind::Char)
    }
    
    pub fn void() -> Self {
        TypeInfo::Void
    }
    
    pub fn pointer_to(pointed_type: TypeInfo) -> Self {
        // Pointer-Größe ist architekturabhängig, hier angenommen 4 oder 8 Bytes
        let pointer_size = if cfg!(target_pointer_width = "64") { 8 } else { 4 };
        TypeInfo::Pointer {
            pointed_type: Box::new(pointed_type),
            size: pointer_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub offset: u64,
    pub type_info: TypeInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimitiveKind {
    Int { signed: bool },
    Char,
    Bool,
    // Gleitkommatypen
    Float,
    Double,

    Void,
    Address, //Für Zeiger
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeKind {
    Primitive(PrimitiveKind),
    Struct,
    Array,
    Pointer,
    Union,
    Enum,
    Function,
    Void,
    Unknown,
}
