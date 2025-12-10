use clap::{Parser, Subcommand};
use std::fs;
use std::process::Command as SysCommand;

/// WASM Converter CLI
#[derive(Parser)]
#[command(name = "wasm-convert")]
#[command(version = "1.4")]
#[command(about = "Convert between WASM, WAT, C, and C++", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    }

}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::WasmWat { input, output } => {
            ensure_file_exists(input)?;
            let wat = wasmprinter::print_file(input)
                .map_err(|e| anyhow::anyhow!("Failed to convert WASM to WAT: {}", e))?;
            fs::write(output, wat)?;
            println!("✅ Converted {} → {}", input, output);
        }

        Commands::WatWasm { input, output } => {
            ensure_file_exists(input)?;
            let wat_src = fs::read_to_string(input)?;
            let wasm_bytes = wat::parse_str(&wat_src)
                .map_err(|e| anyhow::anyhow!("Failed to convert WAT to WASM: {}", e))?;
            fs::write(output, wasm_bytes)?;
            println!("✅ Converted {} → {}", input, output);
        }

        Commands::WasmC { input, output } => {
            ensure_file_exists(input)?;
            let status = SysCommand::new("wasm2c")
                .arg(input)
                .arg("-o")
                .arg(output)
                .status()
                .map_err(|e| anyhow::anyhow!("Failed to run wasm2c: {}", e))?;
            if !status.success() {
                return Err(anyhow::anyhow!("wasm2c command failed"));
            }
            println!("✅ Converted {} → {}", input, output);
        }

        Commands::C2Wasm {
            input,
            output,
            minimal,
            wasi,
            wasi_sysroot,
        } => compile_c_to_wasm(input, output, *minimal, *wasi, wasi_sysroot)?,

        Commands::CWAT {
            input,
            output,
            minimal,
            wasi,
            wasi_sysroot,
        } => {
            let temp_wasm = "temp.wasm";
            compile_c_to_wasm(input, temp_wasm, *minimal, *wasi, wasi_sysroot)?;
            let wat = wasmprinter::print_file(temp_wasm)?;
            fs::write(output, wat)?;
            println!("✅ Converted {} → {}", input, output);
            std::fs::remove_file(temp_wasm)?;
        }


        Commands::Cpp2Wat {
            input,
            output,
            wasi,
            wasi_sysroot,
        } => {
            let temp_wasm = "temp_cpp.wasm";
            ensure_file_exists(input)?;
            if *wasi {
                println!("🔹 Compiling C++ in WASI mode...");
                let sysroot = wasi_sysroot
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("--wasi requires --wasi-sysroot"))?;

                let status = SysCommand::new("clang++")
                    .args([
                        "--target=wasm32-wasi",
                        "--sysroot", sysroot,
                        "-D_WASI_EMULATED_SIGNAL",
                        "-o", temp_wasm,
                        input,
                        "-lc++", "-lc++abi", "-lwasi-emulated-signal",
                    ])
                    .status()?;

                if !status.success() {
                    return Err(anyhow::anyhow!("clang++ failed in WASI mode"));
                }
                println!("✅ C++ → WASM done: {}", temp_wasm);

                let wat = wasmprinter::print_file(temp_wasm)?;
                fs::write(output, wat)?;
                println!("✅ Converted {} → {}", input, output);
                std::fs::remove_file(temp_wasm)?;
            } else {
                return Err(anyhow::anyhow!("Currently only --wasi mode is supported for C++"));
            }
        }

        Commands::Cpp2Wasm {
            input,
            output,
            wasi,
            wasi_sysroot,
        } => {
            ensure_file_exists(input)?;
            if *wasi {
                println!("🔹 Compiling C++ in WASI mode with signal emulation...");
                let sysroot = wasi_sysroot
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("--wasi requires --wasi-sysroot"))?;

                let status = SysCommand::new("clang++")
                    .args([
                        "--target=wasm32-wasi",
                        "--sysroot", sysroot,
                        "-D_WASI_EMULATED_SIGNAL",
                        "-o", output,
                        input,
                        "-lc++", "-lc++abi", "-lwasi-emulated-signal",
                    ])
                    .status()?;

                if !status.success() {
                    return Err(anyhow::anyhow!("clang++ failed in WASI mode"));
                }
                println!("✅ C++ → WASM done: {}", output);
            } else {
                return Err(anyhow::anyhow!("Currently only --wasi mode is supported for C++"));
            }
        }
    }

    Ok(())
}

fn compile_c_to_wasm(
    input: &str,
    output: &str,
    minimal: bool,
    wasi: bool,
    wasi_sysroot: &Option<String>,
) -> anyhow::Result<()> {
    ensure_file_exists(input)?;
    if minimal && wasi {
        return Err(anyhow::anyhow!("Cannot use both --minimal and --wasi."));
    }
    if minimal {
        println!("🔹 Compiling in minimal mode...");
        let status = SysCommand::new("clang")
            .args([
                "--target=wasm32",
                "-nostdlib",
                "-Wl,--no-entry",
                "-Wl,--export-all",
                "-o",
                output,
                input,
            ])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("clang failed in minimal mode"));
        }
    } else if wasi {
        println!("🔹 Compiling in WASI mode with signal emulation...");
        let sysroot = wasi_sysroot
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("--wasi requires --wasi-sysroot"))?;
        let status = SysCommand::new("clang")
            .args([
                "--target=wasm32-wasi",
                "--sysroot", sysroot,
                "-D_WASI_EMULATED_SIGNAL",
                "-o", output,
                input,
                "-lwasi-emulated-signal",
            ])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("clang failed in WASI mode"));
        }
    } else {
        return Err(anyhow::anyhow!("Specify --minimal or --wasi."));
    }
    println!("✅ C → WASM done: {}", output);
    Ok(())
}

fn ensure_file_exists(path: &str) -> anyhow::Result<()> {
    if !std::path::Path::new(path).exists() {
        return Err(anyhow::anyhow!("Input file '{}' does not exist", path));
    }
    Ok(())
}

