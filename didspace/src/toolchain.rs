use std::path::{Path, PathBuf};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum CheckStatus {
    Ok,
    Warn,
    Err,
    Skip,
}

#[derive(Debug, Clone, Serialize)]
pub struct Toolcheck {
    pub name: &'static str,
    pub status: CheckStatus,
    pub details: String,
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolchainReport {
    pub checks: Vec<Toolcheck>,
}

impl ToolchainReport {
    pub fn summary_counts(&self) -> (usize, usize, usize, usize) {
        let mut ok = 0;
        let mut warn = 0;
        let mut err = 0;
        let mut skip = 0;
        for c in &self.checks {
            match c.status {
                CheckStatus::Ok => ok += 1,
                CheckStatus::Warn => warn += 1,
                CheckStatus::Err => err += 1,
                CheckStatus::Skip => skip += 1,
            }
        }
        (ok, warn, err, skip)
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        let (ok, warn, err, skip) = self.summary_counts();
        out.push_str(&format!(
            "Toolchain checks: ✅ {}  ⚠️ {}  ❌ {}  ⏭ {} \n",
            ok, warn, err, skip
        ));

        for c in &self.checks {
            let icon = match c.status {
                CheckStatus::Ok => "✅",
                CheckStatus::Warn => "⚠️",
                CheckStatus::Err => "❌",
                CheckStatus::Skip => "⏭",
            };
            out.push_str(&format!("{} {}: {}\n", icon, c.name, c.details));
            if let Some(fix) = &c.fix {
                out.push_str(&format!("   fix: {}\n", fix));
            }
        }
        out
    }
}

/// Opzioni (puoi espanderle in futuro)
pub struct ToolchainOptions<'a> {
    pub wasi_sysroot: Option<&'a str>,
    pub check_cpp: bool,
    pub check_wasmtime: bool,
}

/// Entry point
pub fn toolchain_check(opts: ToolchainOptions<'_>) -> ToolchainReport {
    let mut checks: Vec<Toolcheck> = Vec::new();

    // 1) wasmtime
    if opts.check_wasmtime {
        match which("wasmtime") {
            Some(p) => checks.push(Toolcheck {
                name: "wasmtime",
                status: CheckStatus::Ok,
                details: format!("found at {}", p.display()),
                fix: None,
            }),
            None => checks.push(Toolcheck {
                name: "wasmtime",
                status: CheckStatus::Warn,
                details: "not found in PATH (run suggestions may not work)".into(),
                fix: Some("install wasmtime and ensure it's in PATH".into()),
            }),
        }
    }

    // 2) compilers (wrapper preferred)
    match which("wasm32-wasi-clang") {
        Some(p) => checks.push(Toolcheck {
            name: "wasm32-wasi-clang",
            status: CheckStatus::Ok,
            details: format!("found at {}", p.display()),
            fix: None,
        }),
        None => checks.push(Toolcheck {
            name: "wasm32-wasi-clang",
            status: CheckStatus::Warn,
            details: "not found in PATH (C→WASM may fail unless you use clang with --target/--sysroot)".into(),
            fix: Some("add wasi-sdk/bin to PATH or install wasi-sdk release".into()),
        }),
    }

    if opts.check_cpp {
        match which("wasm32-wasi-clang++") {
            Some(p) => checks.push(Toolcheck {
                name: "wasm32-wasi-clang++",
                status: CheckStatus::Ok,
                details: format!("found at {}", p.display()),
                fix: None,
            }),
            None => checks.push(Toolcheck {
                name: "wasm32-wasi-clang++",
                status: CheckStatus::Warn,
                details: "not found in PATH (C++→WASM may fail)".into(),
                fix: Some("add wasi-sdk/bin to PATH or install wasi-sdk release".into()),
            }),
        }
    }

    // 3) sysroot detection
    let sysroot = detect_sysroot(opts.wasi_sysroot);
    match &sysroot {
        Some(p) => checks.push(Toolcheck {
            name: "wasi-sysroot",
            status: CheckStatus::Ok,
            details: format!("using {}", p.display()),
            fix: None,
        }),
        None => checks.push(Toolcheck {
            name: "wasi-sysroot",
            status: CheckStatus::Skip,
            details: "not provided (pass --wasi-sysroot or set WASI_SYSROOT)".into(),
            fix: Some("export WASI_SYSROOT=/path/to/wasi-sdk/share/wasi-sysroot".into()),
        }),
    }

    // 4) sysroot sanity (stdio.h)
    if let Some(sysroot) = &sysroot {
        let candidates = [
            sysroot.join("include").join("stdio.h"),
            sysroot.join("include").join("wasm32-wasi").join("stdio.h"),
            sysroot.join("include").join("wasm32-wasip1").join("stdio.h"),
            sysroot.join("include").join("wasm32-wasip2").join("stdio.h"),
        ];
        
        let found = candidates.iter().find(|p| p.exists());

        if let Some(p) = found {
            checks.push(Toolcheck {
                name: "sysroot:stdio.h",
                status: CheckStatus::Ok,
                details: "found stdio.h".into(),
                fix: None,
            });
        } else {
            checks.push(Toolcheck {
                name: "sysroot:stdio.h",
                status: CheckStatus::Err,
                details: format!("missing stdio.h (checked {})", candidates
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
                ),
                fix: Some("sysroot is wrong/incomplete; use a wasi-sdk release or correct --wasi-sysroot".into()),
            });
        }

        // 5) C++ headers sanity (iostream)
        if opts.check_cpp {
            let p1 = sysroot.join("include").join("c++").join("v1").join("iostream");
            let p2 = sysroot
                .join("include")
                .join("wasm32-wasi")
                .join("c++")
                .join("v1")
                .join("iostream");

            if p1.exists() || p2.exists() {
                checks.push(Toolcheck {
                    name: "sysroot:iostream",
                    status: CheckStatus::Ok,
                    details: "found C++ headers (iostream)".into(),
                    fix: None,
                });
            } else {
                checks.push(Toolcheck {
                    name: "sysroot:iostream",
                    status: CheckStatus::Warn,
                    details: "C++ headers not found (iostream). C++ builds may fail or be limited".into(),
                    fix: Some("install a wasi-sdk build that includes libc++ headers".into()),
                });
            }
        }

        // 6) emulation libs available (info-level via OK/WARN)
        let emus = find_emulated_libs(sysroot);
        if emus.is_empty() {
            checks.push(Toolcheck {
                name: "sysroot:emulations",
                status: CheckStatus::Warn,
                details: "no libwasi-emulated-*.a found".into(),
                fix: Some("use a wasi-sdk sysroot that ships emulated libs, or disable emulation flags".into()),
            });
        } else {
            checks.push(Toolcheck {
                name: "sysroot:emulations",
                status: CheckStatus::Ok,
                details: format!("found: {}", emus.join(", ")),
                fix: None,
            });
        }
    }

    ToolchainReport { checks }
}

// ---------------- utilities ----------------

fn detect_sysroot(cli: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = cli {
        return Some(PathBuf::from(p));
    }
    if let Ok(p) = std::env::var("WASI_SYSROOT") {
        if !p.trim().is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    // Optional: if you want, also support WASI_SDK_PATH → derive sysroot
    if let Ok(sdk) = std::env::var("WASI_SDK_PATH") {
        if !sdk.trim().is_empty() {
            let p = PathBuf::from(sdk).join("share").join("wasi-sysroot");
            return Some(p);
        }
    }
    None
}

fn which(cmd: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let full = dir.join(cmd);
        if is_executable(&full) {
            return Some(full);
        }
        // windows (non ti serve ora, ma non fa male)
        #[cfg(windows)]
        {
            let full_exe = dir.join(format!("{}.exe", cmd));
            if is_executable(&full_exe) {
                return Some(full_exe);
            }
        }
    }
    None
}

fn is_executable(p: &Path) -> bool {
    if !p.is_file() {
        return false;
    }
    // su linux basta che esista; se vuoi puoi controllare permessi +x
    true
}

fn find_emulated_libs(sysroot: &Path) -> Vec<String> {
    let candidates = [
        "libwasi-emulated-signal.a",
        "libwasi-emulated-mman.a",
        "libwasi-emulated-getpid.a",
        "libwasi-emulated-process-clocks.a",
    ];
    let prefixes = [
        "lib/wasm32-wasi",
        "lib/wasm32-wasip1",
        "lib/wasm32-wasip2",
        "lib/wasm32-wasi-threads",
        "lib/wasm32-wasip1-threads",
    ];

    let mut found = Vec::new();
    for pre in prefixes {
        for c in candidates {
            let full = sysroot.join(pre).join(c);
            if full.exists() {
                found.push(format!("{}/{}", pre, c));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

