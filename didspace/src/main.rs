use clap::{Arg, Parser, Subcommand};
mod cli;
use cli::{Cli, Commands};
mod hex_dump;
use hex_dump::wasm_to_hex;
mod analysis;
use analysis::WasmAnalysis;
mod repl;
use repl::{start_repl, CommandCompleter};
mod utils; 
mod converter;
use std::fs;
mod wasi;
use wasi::{detect_wasi_imports, detect_component_model, analyze_component};
mod doctor;
use doctor::{doctor_report, report_to_text, DoctorOptions};
use std::process::Command as SysCommand;
use anyhow::anyhow;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let result: anyhow::Result<String> = match cli.command {
        Commands::Wasm2Hex { file } => {
            let bytes = fs::read(&file).expect("Failed to read WASM file");
            let dump = wasm_to_hex(&bytes);
            println!("{}", dump);
            let mut out = String::new();
            out.push_str("Wasm2Hex\n");
            out.push_str(&format!("  file: {}\n\n", file));
            out.push_str(&dump);

            Ok(out)
        }

        Commands::WasmWat { input, output } => {
            ensure_file_exists(&input)?;
            let mut report = String::new();
            report.push_str("Wasm2Wat\n");
            let wat = wasmprinter::print_file(&input)
                .map_err(|e| anyhow::anyhow!("Failed to convert WASM to WAT: {}", e))?;
            fs::write(&output, wat)?;
            println!("✅ Converted {} → {}", input, output);
            report.push_str(&format!("  Compiled WASM → WAT: {}\n", output));
            Ok(report)
        }

        Commands::WatWasm { input, output } => {
            ensure_file_exists(&input)?;
            let wat_src = fs::read_to_string(&input)?;
            let mut report = String::new();
            report.push_str("Wat2Wasm\n");
            report.push_str(&format!("  input:  {}\n", input));
            report.push_str(&format!("  output: {}\n", output));
            let wasm_bytes = wat::parse_str(&wat_src)
                .map_err(|e| anyhow::anyhow!("Failed to convert WAT to WASM: {}", e))?;
            fs::write(&output, wasm_bytes)?;
            println!("✅ Converted {} → {}", input, output);
            report.push_str(&format!("  - compiled C → WASM: {}\n", output));
            Ok(report)
        }

        Commands::WasmC { input, output } => {
            ensure_file_exists(&input)?;
            let status = SysCommand::new("wasm2c")
                .arg(&input)
                .arg("-o")
                .arg(&output)
                .status()
                .map_err(|e| anyhow::anyhow!("Failed to run wasm2c: {}", e))?;
            if !status.success() {
                Err(anyhow::anyhow!("wasm2c command failed"))
            } else {
                let mut report = String::new();
                report.push_str("WasmC\n");
                report.push_str(&format!("  input: {}\n", input));
                report.push_str(&format!("  output: {}\n", output));
                report.push_str("✅ Steps:\n");
                report.push_str(&format!("  - ran: wasm2c {} -o {}\n", input, output));
                println!("✅ Converted {} → {}", input, output);
                Ok(report)
        
            }
        }

        Commands::C2Wasm {
            input,
            output,
            minimal,
            wasi,
            wasi_sysroot,
        } => {
            let mut report = String::new();
            report.push_str("CWAT\n");
            report.push_str(&format!("  input:  {}\n", input));
            report.push_str(&format!("  output: {}\n", output));
            report.push_str(&format!("  mode:   {}\n", if minimal { "minimal" } else if wasi { "wasi" } else { "none" }));
            if wasi {
                report.push_str(&format!(
                    "  wasi_sysroot: {}\n",
                    wasi_sysroot.as_deref().unwrap_or("<missing>")
                ));
            }
            report.push('\n');
            compile_c_to_wasm(&input, &output, minimal, wasi, &wasi_sysroot)?;
            report.push_str("✅ Steps:\n");
            report.push_str(&format!("  - compiled C → WASM: {}\n", output));

            Ok(report)
        }

        Commands::CWAT {
            input,
            output,
            minimal,
            wasi,
            wasi_sysroot,
        } => {
            let temp_wasm = "temp.wasm";
            let mut report = String::new();
            report.push_str("CWAT\n");
            report.push_str(&format!("  input:  {}\n", input));
            report.push_str(&format!("  output: {}\n", output));
            report.push_str(&format!("  mode:   {}\n", if minimal { "minimal" } else if wasi { "wasi" } else { "none" }));
            if wasi {
                report.push_str(&format!(
                    "  wasi_sysroot: {}\n",
                    wasi_sysroot.as_deref().unwrap_or("<missing>")
                ));
            }
            report.push('\n');

            compile_c_to_wasm(&input, temp_wasm, minimal, wasi, &wasi_sysroot)?;
            let wat = wasmprinter::print_file(temp_wasm)?;
            fs::write(&output, wat)?;
            report.push_str("✅ Steps:\n");
            report.push_str(&format!("  - compiled C → WASM: {}\n", temp_wasm));
            report.push_str(&format!("  - printed WASM → WAT (wasmprinter)\n"));
            report.push_str(&format!("  - wrote WAT to: {}\n", output));
            report.push_str(&format!("  - removed temp: {}\n", temp_wasm));

            std::fs::remove_file(temp_wasm)?;

            println!("✅ Converted {} → {}", input, output);

            Ok(report)
        }


        Commands::Cpp2Wat {
            input,
            output,
            wasi,
            wasi_sysroot,
        } => {
            let temp_wasm = "temp_cpp.wasm";
            ensure_file_exists(&input)?;
            if wasi {
                println!("🔹 Compiling C++ in WASI mode...");
                let sysroot = wasi_sysroot
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("--wasi requires --wasi-sysroot"))?;

                let status = SysCommand::new("wasm32-wasi-clang++")
                    .args([
                        "--target=wasm32-wasi",
                        "--sysroot", sysroot,
                        "-D_WASI_EMULATED_SIGNAL", "-D_WASI_EMULATED_MMAN",
                        //"-fno-exceptions",
                        "-fno-rtti",
                        "-o", temp_wasm,
                        &input,
                        "-lc++", "-lc++abi", "-lwasi-emulated-signal", "-lwasi-emulated-mman",
                    ])
                    .status()?;

                if !status.success() {
                    return Err(anyhow::anyhow!("clang++ failed in WASI mode"));
                }
                println!("✅ C++ → WAT done: {}", temp_wasm);

                let wat = wasmprinter::print_file(temp_wasm)?;
                fs::write(&output, &wat)?;
                println!("✅ Converted {} → {}", input, output);
                std::fs::remove_file(temp_wasm)?;
                
                let mut report = String::new();
                report.push_str("Cpp2Wat\n");
                report.push_str(&format!("  input:       {}\n", input));
                report.push_str(&format!("  output:      {}\n", output));
                report.push_str(&format!("  wat:         {}\n", wat));
                report.push_str(&format!("  wasi_sysroot: {}\n\n", sysroot));

                report.push_str("✅ Steps:\n");
                /*report.push_str(&format!("  - compiled C++ → WASM: {}\n", temp_wasm));*/
                report.push_str(&format!("  - converted WASM → WAT: {}\n", output));
                report.push_str(&format!("  - removed temp: {}\n", temp_wasm));

                Ok(report)
            } else {
                Err(anyhow::anyhow!("Currently only --wasi mode is supported for C++"))
            }
        }

        Commands::Cpp2Wasm {
            input,
            output,
            wasi,
            wasi_sysroot,
        } => {
            ensure_file_exists(&input)?;
            if wasi {
                println!("🔹 Compiling C++ in WASI mode with signal emulation...");
                let sysroot = wasi_sysroot
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("--wasi requires --wasi-sysroot"))?;

                let status = SysCommand::new("wasm32-wasi-clang++")
                    .args([
                        "--target=wasm32-wasi",
                        "--sysroot", sysroot,
                        "-D_WASI_EMULATED_SIGNAL", "-D_WASI_EMULATED_MMAN",
                        "-fno-exceptions",
                        "-fno-rtti",
                        "-o", &output,
                        &input,
                        "-lc++", "-lc++abi", "-lwasi-emulated-signal", "-lwasi-emulated-mman",
                    ])
                    .status()?;

                if !status.success() {
                    return Err(anyhow::anyhow!("clang++ failed in WASI mode"));
                }
                println!("✅ C++ → WASM done: {}", output);
                //Ok("Conversion from C++ to WASM completed".to_string())
                let mut report = String::new();
                report.push_str("Cpp2Wasm\n");
                report.push_str(&format!("  input:       {}\n", input));
                report.push_str(&format!("  output:      {}\n", output));
                report.push_str(&format!("  wasi_sysroot: {}\n\n", sysroot));

                report.push_str("✅ Steps:\n");
                report.push_str(&format!("  - compiled C++ → WASM: {}\n", output));

                Ok(report)
            } else {
                Err(anyhow::anyhow!("Currently only --wasi mode is supported for C++"))
            }
        }
        Commands::Analyze { file } => {
            let bytes = fs::read(&file).expect("Failed to read WASM file");
            let analysis = WasmAnalysis::analyze(&bytes);
            println!("{}", analysis?.report());
            Ok(WasmAnalysis::analyze_report(&bytes)?)
        }

        Commands::Profile { file } => {
            let bytes = std::fs::read(&file)?;
            Ok(WasmAnalysis::profile_functions(&bytes)?)
            //Ok("WASM profiling completed".to_string())
        }

        Commands::Wasi { file } => {
            let bytes = std::fs::read(&file)?;
            let (found, report) = detect_wasi_imports(&bytes)?;
            let mut out = String::new();
            out.push_str(&format!("file: {}\n\n", file));
            out.push_str(&report);
            out.push_str(&format!("\nSummary: wasi_found={}\n", found));
            Ok(out)
            //Ok("WASI analysis completed".to_string())
        }

        Commands::Component { file } => {
            let bytes = std::fs::read(&file)?;
            //detect_component_model(&bytes);
            let report = analyze_component(&bytes).map_err(|e| anyhow!(e))?;
            // Ritorna comunque una String, per uniformarsi agli altri rami
            Ok(report)
        }

        Commands::Doctor { file, wasi_sysroot, max_list, json, pretty } => {
            let bytes = std::fs::read(&file)?;
            let rep = doctor_report(
                &bytes,
                DoctorOptions {
                    wasi_sysroot: wasi_sysroot.as_deref(),
                    max_list,
                },
            )?;

            let out = if json {
                if pretty {
                    serde_json::to_string_pretty(&rep)?
                } else {
                    serde_json::to_string(&rep)?
                }
            } else {
                report_to_text(&rep)
            };

            Ok(out)
        }

        Commands::Repl => {
            start_repl();
            Ok("Started REPL".to_string())
        }

        _ => Ok("No command provided".to_string()),
    };

    if let Some(path) = cli.report {
        let content = result?;
        std::fs::write(&path, content)?;
        println!("✅ Output written to {}", path);
    } else {
        println!("{:?}", result);
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
    ensure_file_exists(&input)?;
    if minimal && wasi {
        return Err(anyhow::anyhow!("Cannot use both --minimal and --wasi."));
    }
    if minimal {
        println!("🔹 Compiling in minimal mode...");
        let status = SysCommand::new("wasm32-wasi-clang")
            .args([
                "--target=wasm32",
                //"-nostdlib",
                "-fno-exceptions",
                "-fno-rtti",
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
        let status = SysCommand::new("wasm32-wasi-clang")
            .args([
                "--target=wasm32-wasi",
                "--sysroot", sysroot,
                "-D_WASI_EMULATED_SIGNAL", "-D_WASI_EMULATED_MMAN",
                "-o", output,
                input,
                "-lwasi-emulated-signal", "-lwasi-emulated-mman",
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

