use wabt::wat2wasm;
use wasmprinter;

pub fn wat_to_wasm(wat: &str) -> Result<Vec<u8>, String> {
    wat2wasm(wat).map_err(|e| e.to_string())
}

pub fn wasm_to_wat(bytes: &[u8]) -> Result<String, String> {
    wasmprinter::print_bytes(bytes).map_err(|e| e.to_string())
}
