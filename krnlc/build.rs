use std::{env::var, fs, path::PathBuf, process::Command};

macro_rules! toolchain_check {
    ($msg:literal) => {
        #[rustversion::stable]
        compile_error!($msg);
    
        #[rustversion::before(2023-01-14)]
        compile_error!($msg);

        #[rustversion::since(2023-01-28)]
        compile_error!($msg);
    };
}

toolchain_check!("krnlc requires nightly-2023-01-21, install with rustup:
rustup component add --toolchain nightly-2023-01-21 rust-src rustc-dev llvm-tools-preview");

fn main() {
    let output = Command::new(var("RUSTC").unwrap())
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    if !output.status.success() {
        panic!("{}", String::from_utf8(output.stderr).unwrap());
    }
    let sysroot = String::from_utf8(output.stdout).unwrap();
    let toolchain_lib = PathBuf::from(sysroot.trim()).join("lib");
    println!(
        "cargo:rustc-env=KRNLC_TOOLCHAIN_LIB={}",
        toolchain_lib.display()
    );
    for entry in fs::read_dir(&toolchain_lib).unwrap().map(Result::unwrap) {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.starts_with("libLLVM-") {
            println!("cargo:rustc-env=KRNLC_LIBLLVM={file_name}");
        } else if file_name.starts_with("librustc_driver-") {
            println!("cargo:rustc-env=KRNLC_LIBRUSTC_DRIVER={file_name}");
        } else if file_name.starts_with("libstd-") {
            println!("cargo:rustc-env=KRNLC_LIBSTD={file_name}");
        }
    }
    let out_dir = PathBuf::from(var("OUT_DIR").unwrap());
    let target_dir = out_dir.ancestors().nth(3).unwrap();
    let rustc_codegen_spirv_path = target_dir.join("librustc_codegen_spirv.so");
    if !rustc_codegen_spirv_path.exists() {
        std::fs::create_dir_all(target_dir).unwrap();
        std::fs::write(rustc_codegen_spirv_path, []).unwrap();
    }
}