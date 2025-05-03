use std::env;
use std::fs;
use std::path::Path;

// It used to connect dynamic libraries
#[allow(dead_code)]
fn copy_file_to_build_folder(file: &'static str, path: &'static str) {
    // Rebuild if the file changes
    println!("cargo:rerun-if-changed={}", file);

    // Project dir
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_file = Path::new(&project_dir).join(file);

    // Build folder
    let out_dir = String::from("target");
    let profile = std::env::var("PROFILE").unwrap();
    let dest_path = Path::new(&project_dir)
        .join(&out_dir)
        .join(&profile)
        .join(path);

    // Copy file to path
    let dest_file = dest_path.join(src_file.file_name().unwrap());
    println!("cargo:info={}", project_dir);
    println!("cargo:info={}", dest_file.to_str().unwrap());
    fs::copy(&src_file, &dest_file).unwrap();
}

fn main() {
    // Import WinAPI modules
    println!("cargo:rustc-link-lib=dylib=user32");
}
