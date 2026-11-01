use std::path::Path;

/// Track original headers, not files that this build script writes itself.
pub fn emit_header_input(filename: &str, output: &Path) {
    if !Path::new(filename).starts_with(output) {
        println!("cargo:rerun-if-changed={filename}");
    }
}

/// Replace copied inputs and native outputs whenever Cargo reruns the script.
pub fn prepare_native_source(source: &Path, destination: &Path, cmake_output: &Path) {
    println!("cargo:rerun-if-changed={}", source.display());
    for directory in [
        destination.to_path_buf(),
        cmake_output.join("build"),
        cmake_output.join("lib"),
        cmake_output.join("bin"),
        cmake_output.join("include"),
    ] {
        if directory.exists() {
            std::fs::remove_dir_all(&directory)
                .expect("failed to remove stale native build inputs or outputs");
        }
    }
    copy_tree(source, destination);
}

fn copy_tree(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).expect("failed to create native source directory");
    for entry in std::fs::read_dir(source).expect("failed to read native source directory") {
        let entry = entry.expect("failed to read native source entry");
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if entry
            .file_type()
            .expect("failed to read native source type")
            .is_dir()
        {
            copy_tree(&path, &target);
        } else {
            std::fs::copy(path, target).expect("failed to copy native source input");
        }
    }
}
