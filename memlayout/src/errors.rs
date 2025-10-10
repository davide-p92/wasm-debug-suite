use thiserror::Error;
use gimli;
use std::io;

#[derive(Error, Debug)]
pub enum DwarfError {
    #[error("Failed to parse DWARF information: {0}")]
    ParseError(String),
    
    #[error("Invalid DWARF section: {0}")]
    InvalidSection(String),
    
    #[error("Missing DWARF information: {0}")]
    MissingInfo(String),
    
    #[error("Unsupported DWARF version: {0}")]
    UnsupportedVersion(u16),

    #[error("Register {0:?} unavailable: {1}")]
    RegisterUnavailable(gimli::Register, String),

    #[error("Frame base unavailable: {0}")]
    FrameBaseUnavailable(String),

    #[error("Relocation error at offset {0}: {1}")]
    RelocationError(u64, String),

    #[error("Unsupported expression: {0}")]
    UnsupportedExpression(String),

    #[error("Unsupported location: {0}")]
    UnsupportedLocation(String),

    #[error("Memory read error at location 0x{0:x} with size {1}: {2}")]
    MemoryReadError(u64, u64, String), // Keine benannten Parameter!

    #[error("TLS unavailable: {0}")]
    TlsUnavailable(String),

    #[error("Entry value unavailable: {0}")]
    EntryValueEvaluationError(String),

    #[error("Invalid location expression")]
    InvalidLocation,
    
    #[error("Type resolution failed: {0}")]
    TypeResolutionFailed(String),
    
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    
    #[error("Gimli error: {0}")]
    GimliError(#[from] gimli::Error),

    #[error("Unknown DWARF error: {0}")]
    Unknown(String),
}

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Invalid memory access at address 0x{address:x}")]
    InvalidAccess { address: u64 },
    
    #[error("Unsupported integer size: {size}")]
    UnsupportedSize { size: usize },
    
    #[error("Variable '{0}' not found")]
    VariableNotFound(String),
    
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },
    
    #[error("Invalid type information for variable '{0}'")]
    InvalidTypeInfo(String),
    
    #[error("Memory not initialized")]
    MemoryNotInitialized,
    
    #[error("WASM runtime error: {0}")]
    WasmRuntimeError(String),
    
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    
    #[error("Unknown memory error: {0}")]
    Unknown(String),

    #[error("Unsupported type: {0}")]
    UnsupportedType(String),

    #[error("Failed to parse value: {0}")]
    ParseError(String),
}

// Hilfsfunktionen für einfache Fehlerkonvertierung
impl From<String> for DwarfError {
    fn from(s: String) -> Self {
        DwarfError::Unknown(s)
    }
}

impl From<&str> for DwarfError {
    fn from(s: &str) -> Self {
        DwarfError::Unknown(s.to_string())
    }
}

impl From<RuntimeError> for DwarfError {
    fn from(err: RuntimeError) -> Self {
        match err {
            RuntimeError::InvalidMemoryAccess => {
                DwarfError::MemoryReadError(0, 0, "Invalid memory access".to_string())
            }
            other => DwarfError::Unknown(format!("Runtime error: {}", other)),
        }
    }
}

impl From<wasmparser::BinaryReaderError> for DwarfError {
    fn from(e: wasmparser::BinaryReaderError) -> Self {
        DwarfError::ParseError(e.to_string())
    }
}

/*
impl From<DwarfError> for gimli::Error {
    fn from(err:DwarfError) -> Self {
        gimli::Error::Io(
            std::io::Error::new(std::io::ErrorKind::Other, format!("{}", err)),
        );
    }
}*/

impl From<String> for MemoryError {
    fn from(s: String) -> Self {
        MemoryError::Unknown(s)
    }
}

impl From<&str> for MemoryError {
    fn from(s: &str) -> Self {
        MemoryError::Unknown(s.to_string())
    }
}

// Erweiterte Fehlerbehandlung
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Failed to create WASM instance: {0}")]
    InstanceCreation(String),

    #[error("Memory '{0}' not found")]
    MemoryNotFound(String),

    #[error("Function '{0}' not found")]
    FunctionNotFound(String),

    #[error("Failed to call function '{0}': {1}")]
    FunctionCall(String, String),

    #[error("Global '{0}' not found")]
    GlobalNotFound(String),

    #[error("Failed to set global '{0}': {1}")]
    GlobalSetFailed(String, String),

    #[error("Table '{0}' not found")]
    TableNotFound(String),

    #[error("Failed to access table '{0}': {1}")]
    TableAccessFailed(String, String),

    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid memory access")]
    InvalidMemoryAccess,

    #[error("Unknown runtime error: {0}")]
    Unknown(String)
}

