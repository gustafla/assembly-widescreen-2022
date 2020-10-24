use gl_generator::{Api, Fallbacks, Profile, Registry, StaticStructGenerator};
use std::env;
use std::fs;
use std::path::Path;

fn generate_gl() {
    let dest = Path::new(&env::var("OUT_DIR").unwrap()).join("gles2_bindings.rs");
    let mut file = fs::File::create(&dest).unwrap();

    Registry::new(Api::Gles2, (2, 0), Profile::Core, Fallbacks::All, [])
        .write_bindings(StaticStructGenerator, &mut file)
        .unwrap();
}

fn add_resources() {
    // Copy resource files from resources to target directory
    let target_dir = Path::new(&env::var("OUT_DIR").unwrap())
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap();

    let resource_file_paths = fs::read_dir("resources").unwrap();

    for dir_entry in resource_file_paths {
        let source_path = dir_entry.unwrap().path();
        let target_path = target_dir.join(source_path.file_name().unwrap());

        println!("source_path {}", source_path.display());
        println!("target_path {}", target_path.display());

        fs::copy(source_path, target_path).unwrap();
    }
}

fn main() {
    generate_gl();
    add_resources();
}
