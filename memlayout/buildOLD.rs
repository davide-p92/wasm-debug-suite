fn main() {
    cc::Build::new()
        .file("src/probestack_shim.c")
        .compile("probestack_shim");
}
