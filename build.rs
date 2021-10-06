#![allow(unreachable_code)]

use gl_generator::{Api, Fallbacks, Profile, Registry, StaticGenerator};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Generate GL ES 2.0 bindings
    let dest = Path::new(&env::var("OUT_DIR").unwrap()).join("gles2_bindings.rs");
    let mut file = fs::File::create(&dest).unwrap();
    Registry::new(Api::Gles2, (2, 0), Profile::Core, Fallbacks::All, [])
        .write_bindings(StaticGenerator, &mut file)
        .unwrap();

    // Link GL ES 2.0
    let pkg_config = pkg_config::Config::new();
    pkg_config.probe("glesv2").unwrap();
}
