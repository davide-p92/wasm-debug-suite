// src/cli.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "didspace", version = "1.0", about = "WASM/WAT Translator")]
pub struct Cli {
    #[arg(long, global = true)]
    pub report: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Convert WASM binary to hex dump
    Wasm2Hex {
        /// Path to the WASM file
        #[arg(value_name = "FILE")]
        file: String,
    },
    // Other commands like wasm2wat, wat2wasm...
    /// Convert WASM to WAT
    WasmWat { input: String, output: String },

    /// Convert WAT to WASM
    WatWasm { input: String, output: String },

    /// Convert WASM to C
    WasmC { input: String, output: String },

    /// Convert C to WASM
    C2Wasm {
        input: String,
        output: String,
        #[arg(long)] minimal: bool,
        #[arg(long)] wasi: bool,
        #[arg(long)] wasi_sysroot: Option<String>,
    },

    /// Convert C to WAT (via WASM)
    CWAT {
        input: String,
        output: String,
        #[arg(long)] minimal: bool,
        #[arg(long)] wasi: bool,
        #[arg(long)] wasi_sysroot: Option<String>,
    },

    /// Convert C++ to WASM
    Cpp2Wasm {
        input: String,
        output: String,
        #[arg(long)] wasi: bool,
        #[arg(long)] wasi_sysroot: Option<String>,
    },

    /// Convert C++ to WAT
    Cpp2Wat {
        input: String,
        output: String,
        #[arg(long)] wasi: bool,
        #[arg(long)] wasi_sysroot: Option<String>,
    },

    /// Analyze a WASM binary and show detailed report
    Analyze {
        #[arg(value_name = "FILE")]
        file: String,
    },

    /// Profile functions in a WASM bin for performance hot
    Profile {
        #[arg(value_name = "FILE")]
        file: String,
    },

    /// WASI detection
    Wasi {
        #[arg(value_name = "FILE")]
        file: String,
    },

    /// Component model for WASI
    Component {
        #[arg(value_name = "FILE")]
        file: String,
    },

    /// Doctor WASI-Wasmtime analysis
    Doctor {
        #[arg(value_name="FILE")]
        file: String,

        #[arg(long)]
        wasi_sysroot: Option<String>,

        #[arg(long, default_value_t = 20)]
        max_list: usize,

        #[arg(long)]
        json: bool,

        #[arg(long)]
        pretty: bool,

        #[arg(long = "check-toolchain")]
        check_toolchain: bool,
    },
    
    Repl,

}
