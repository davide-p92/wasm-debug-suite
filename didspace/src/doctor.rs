use anyhow::Result;
use serde::Serialize;
use wasmparser::{Parser, Payload};

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub kind: String,
    pub wasi: WasiInfo,
    pub core: Option<CoreInfo>,
    pub component: Option<ComponentInfo>,
    pub heuristics: Heuristics,
    pub sysroot: Option<SysrootInfo>,
    pub suggestions: Suggestions,
}

#[derive(Debug, Serialize)]
pub struct WasiInfo {
    pub detected: bool,
    pub flavor: String, // preview1 | wasi | unknown
}

#[derive(Debug, Serialize)]
pub struct CoreInfo {
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ComponentInfo {
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Heuristics {
    pub cxx_exceptions_strings: bool,
    pub detected_strings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SysrootInfo {
    pub path: String,
    pub emulations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Suggestions {
    pub wasmtime_run: Vec<String>,
    pub compile_hints: Vec<String>,
}

pub struct DoctorOptions<'a> {
    pub wasi_sysroot: Option<&'a str>,
    pub max_list: usize,
}

pub fn doctor_report(bytes: &[u8], opts: DoctorOptions<'_>) -> Result<DoctorReport> {
    let kind = detect_kind(bytes)?;
    let (wasi_flavor, wasi_detected) = detect_wasi_preview_and_imports(bytes)?;

    let (core, component, suggestions) = if kind == "component" {
        let (imports, exports) = collect_component_imports_exports(bytes)?;
        let mut run = Vec::new();
        if exports.iter().any(|e| e.contains("wasi:cli/run") || e.contains("wasi:cli")) {
            run.push("wasmtime run component.wasm".to_string());
        } else {
            run.push("wasmtime run component.wasm".to_string());
            run.push("wasmtime run --invoke 'export_name(...)' component.wasm".to_string());
        }
        (
            None,
            Some(ComponentInfo {
                imports: limit_vec(imports, opts.max_list),
                exports: limit_vec(exports, opts.max_list),
            }),
            Suggestions {
                wasmtime_run: run,
                compile_hints: Vec::new(),
            },
        )
    } else {
        let (imports, exports) = collect_core_imports_exports(bytes)?;
        let mut run = Vec::new();
        if wasi_detected {
            run.push("wasmtime run --dir=. module.wasm".to_string());
        } else {
            run.push("wasmtime run module.wasm".to_string());
        }
        (
            Some(CoreInfo {
                imports: limit_vec(imports, opts.max_list),
                exports: limit_vec(exports, opts.max_list),
            }),
            None,
            Suggestions {
                wasmtime_run: run,
                compile_hints: Vec::new(),
            },
        )
    };

    let (eh_found, detected_strings) = detect_cxx_eh_strings(bytes);
    let mut compile_hints = suggestions.compile_hints;
    
    if eh_found {
        compile_hints.push(
            "C++ exceptions strings found; if linking fails on __cxa_*, try: -fno_exceptions -fno-rtti -D_LIBCPP_NO_EXCEPTIONS"
                .to_string(),
            );
    }

    let sysroot = opts.wasi_sysroot.map(|p| SysrootInfo {
        path: p.to_string(),
        emulations: find_emulated_libs(p),
    });

    Ok(DoctorReport {
        kind,
        wasi: WasiInfo {
            detected: wasi_detected,
            flavor: wasi_flavor,
        },
        core,
        component,
        heuristics: Heuristics {
            cxx_exceptions_strings: eh_found,
            detected_strings,
        },
        sysroot,
        suggestions: Suggestions {
            wasmtime_run: suggestions.wasmtime_run,
            compile_hints,
        },
    })
}

/// Versione testo, no JSON
pub fn report_to_text(r: &DoctorReport) -> String {
    let mut out = String::new();
    out.push_str("didspace doctor\n");
    out.push_str("==============\n\n");

    out.push_str(&format!("Kind: {}\n", r.kind));
    out.push_str(&format!("WASI: {}\n", r.wasi.flavor));

    if let Some(core) = &r.core {
        out.push_str(&format!("Core imports: {}\n", core.imports.len()));
        for s in &core.imports {
            out.push_str(&format!(" -  {}\n", s));
        }
        out.push_str(&format!("\nCore exports: {}\n", core.exports.len()));
        for s in &core.exports {
            out.push_str(&format!(" -  {}\n", s));
        }
        out.push('\n');
    }

    out.push_str("Heuristics:\n");
    if r.heuristics.cxx_exceptions_strings {
        out.push_str("  ⚠ C++ EH strings detected (try/catch likely)\n");
        for s in &r.heuristics.detected_strings {
            out.push_str(&format!("    - {}\n", s));
        }
    } else {
        out.push_str("  ✓ No obvious C++ EH strings found\n");
    }

    if let Some(sys) = &r.sysroot {
        out.push_str("\nEmulations available in sysroot:\n");
        if sys.emulations.is_empty() {
            out.push_str("  (none found)\n");
        } else {
            for e in &sys.emulations {
                out.push_str(&format!(" -  {}\n", e));
            }
        }
    }
    
    out.push_str("\nWasmtime suggestions:\n");
    for cmd in &r.suggestions.wasmtime_run {
        out.push_str(&format!("  {}\n", cmd));
    }

    if !r.suggestions.compile_hints.is_empty() {
        out.push_str("\nCompile hints:\n");
        for h in &r.suggestions.compile_hints {
            out.push_str(&format!(" -  {}\n", h));
        }
    }

    out
}

// ---------- helpers ----------

fn limit_vec(mut v: Vec<String>, max: usize) -> Vec<String> {
    if v.len() > max {
        v.truncate(max);
    }
    v
}

fn detect_kind(bytes: &[u8]) -> Result<String> {
    let parser = Parser::new(0);
    for payload in parser.parse_all(bytes) {
        match payload? {
            // component payloads exist only for components
            Payload::ComponentTypeSection(_) |
            Payload::ComponentImportSection(_) |
            Payload::ComponentExportSection(_) => return Ok("component".into()),
            Payload::End(_) => break,
            _ => {}
        }
    }
    Ok("core module".into())
}

fn detect_wasi_preview_and_imports(bytes: &[u8]) -> Result<(String, bool)> {
    let parser = Parser::new(0);
    let mut found_preview1 = false;
    let mut found_wasi_like = false;

    for payload in parser.parse_all(bytes) {
        match payload? {
            Payload::ImportSection(imports) => {
                for im in imports {
                    let im = im?;
                    if im.module == "wasi_snapshot_preview1" {
                        found_preview1 = true;
                        found_wasi_like = true;
                    }
                    if im.module.starts_with("wasi") {
                        found_wasi_like = true;
                    }
                }
            }
            Payload::End(_) => break,
            _ => {}
        }
    }

    if found_preview1 {
        Ok(("preview1 (wasi_snapshot_preview1)".into(), true))
    } else if found_wasi_like {
        Ok(("wasi (unknown flavor)".into(), true))
    } else {
        Ok(("none detected".into(), false))
    }
}

fn collect_core_imports_exports(bytes: &[u8]) -> Result<(Vec<String>, Vec<String>)> {
    let parser = Parser::new(0);
    let mut imports = Vec::new();
    let mut exports = Vec::new();

    for payload in parser.parse_all(bytes) {
        match payload? {
            Payload::ImportSection(reader) => {
                for im in reader {
                    let im = im?;
                    imports.push(format!("{}::{}", im.module, im.name));
                }
            }
            Payload::ExportSection(reader) => {
                for ex in reader {
                    let ex = ex?;
                    exports.push(ex.name.to_string());
                }
            }
            Payload::End(_) => break,
            _ => {}
        }
    }
    Ok((imports, exports))
}

fn collect_component_imports_exports(bytes: &[u8]) -> Result<(Vec<String>, Vec<String>)> {
    let parser = Parser::new(0);
    let mut imports = Vec::new();
    let mut exports = Vec::new();

    for payload in parser.parse_all(bytes) {
        match payload? {
            Payload::ComponentImportSection(reader) => {
                for im in reader {
                    let im = im?;
                    imports.push(format!("{:?}", im.name));
                }
            }
            Payload::ComponentExportSection(reader) => {
                for ex in reader {
                    let ex = ex?;
                    exports.push(format!("{:?}", ex.name));
                }
            }
            Payload::End(_) => break,
            _ => {}
        }
    }
    Ok((imports, exports))
}

fn detect_cxx_eh_strings(bytes: &[u8]) -> (bool, Vec<String>) {
    let needles: [(&[u8], &str); 3] = [
        (b"__cxa_throw", "__cxa_throw"),
        (b"__cxa_allocate_exception", "__cxa_allocate_exception"),
        (b"__gxx_personality_v0", "__gxx_personality_v0"),
    ];

    let mut found = Vec::new();
    for (n, name) in needles {
        if bytes.windows(n.len()).any(|w| w == n) {
            found.push(name.to_string());
        }
    }
    ( !found.is_empty(), found )
}

fn contains_any(haystack: &[u8], needles: &[&[u8]]) -> bool {
    needles.iter().any(|n| haystack.windows(n.len()).any(|w| w == *n))
}

fn find_emulated_libs(sysroot: &str) -> Vec<String> {
    // v1 super semplice: cerca sottostringhe note nei path standard (no filesystem walk, zero dipendenze)
    // Se vuoi fare meglio: usa walkdir crate e cerca "libwasi-emulated-*.a"
    let candidates = [
        "libwasi-emulated-signal.a",
        "libwasi-emulated-mman.a",
        "libwasi-emulated-getpid.a",
        "libwasi-emulated-process-clocks.a",
    ];

    // Path tipici nel tuo sysroot:
    //   $SYSROOT/lib/wasm32-wasi/...
    //   $SYSROOT/lib/wasm32-wasip1/...
    //   $SYSROOT/lib/wasm32-wasip2/...
    let prefixes = [
        "lib/wasm32-wasi",
        "lib/wasm32-wasip1",
        "lib/wasm32-wasip2",
        "lib/wasm32-wasi-threads",
        "lib/wasm32-wasip1-threads",
    ];

    let mut found = Vec::new();
    for p in prefixes {
        for c in candidates {
            let full = format!("{}/{}/{}", sysroot, p, c);
            if std::path::Path::new(&full).exists() {
                found.push(format!("{}/{}", p, c));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

