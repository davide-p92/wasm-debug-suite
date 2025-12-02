
use std::path::Path;

fn main() {
    let llvm_prefix = "/usr/lib/llvm-18";

    if Path::new(llvm_prefix).exists() {
        println!("cargo:rustc-env=LLVM_SYS_180_PREFIX={}", llvm_prefix);
        println!("cargo:rustc-env=LLVM_SYS_NO_POLLY=1");
        println!("cargo:rustc-env=LLVM_SYS_180_NO_POLLY=1");

        println!("cargo:rustc-link-search=native={}/lib", llvm_prefix);
        println!("cargo:rustc-link-lib=dylib=LLVM"); // or static=LLVM if needed

        println!("cargo:warning=🔗 Using LLVM 18 from {}", llvm_prefix);
    } else {
        println!("cargo:warning=❌ LLVM 18 not found at {}", llvm_prefix);
    }

    println!("cargo:rerun-if-changed=build.rs");
}

