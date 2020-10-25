use gl_generator::{Api, Fallbacks, Profile, Registry, StructGenerator};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let dest = Path::new(&env::var("OUT_DIR").unwrap()).join("gles2_bindings.rs");
    let mut file = fs::File::create(&dest).unwrap();

    Registry::new(Api::Gles2, (2, 0), Profile::Core, Fallbacks::All, [])
        .write_bindings(StructGenerator, &mut file)
        .unwrap();
}
