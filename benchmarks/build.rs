// Reuse the library's HMM table generator for the private-module probes.
#[allow(dead_code)]
mod library_build {
    include!("../build.rs");
    pub fn run() { generate_hmm_data(); }
}
fn main() {
    println!("cargo:rerun-if-changed=../build.rs");
    println!("cargo:rerun-if-changed=../src/jieba_rs/hmm.model");
    library_build::run();
}
