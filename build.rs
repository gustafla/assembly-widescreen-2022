#![allow(unreachable_code)]

use gl_generator::{Api, Fallbacks, Profile, Registry, StaticGenerator};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    #[cfg(all(feature = "glutin", feature = "rpi"))]
    {
        println!();
        println!();
        println!();
        println!("------------------------------------------------------------------------------");
        println!("glutin and rpi aren't supported at the same time (use --no-default-features)");
        println!("------------------------------------------------------------------------------");
        println!();
        println!();
        println!();
        panic!();
    }
    #[cfg(not(any(feature = "glutin", feature = "rpi")))]
    {
        println!();
        println!();
        println!();
        println!("------------------------------------------------------------------------------");
        println!("Please select either glutin or rpi. (use --features \"glutin\" or \"rpi\")");
        println!("------------------------------------------------------------------------------");
        println!();
        println!();
        println!();
        panic!();
    }

    // Generate GL ES 2.0 bindings
    let dest = Path::new(&env::var("OUT_DIR").unwrap()).join("gles2_bindings.rs");
    let mut file = fs::File::create(&dest).unwrap();
    Registry::new(Api::Gles2, (2, 0), Profile::Core, Fallbacks::All, [])
        .write_bindings(StaticGenerator, &mut file)
        .unwrap();

    // Link GL ES 2.0
    let pkg_config = pkg_config::Config::new();
    #[cfg(feature = "glutin")]
    {
        pkg_config.probe("glesv2").unwrap();
    }
    #[cfg(feature = "rpi")]
    {
        pkg_config.probe("brcmegl").unwrap();
        pkg_config.probe("brcmglesv2").unwrap();
    }
}
