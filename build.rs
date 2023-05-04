extern crate bindgen;

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    // Tell cargo to look for shared libraries in the specified directory


    //println!("cargo:rustc-link-search={}", solclient_dir);

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    let target = env::var("TARGET").unwrap();

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let lib_dir = Path::new(&manifest_dir)
        .join("lib")
        .to_str()
        .unwrap()
        .to_string();

    //// Tell cargo to link againts solace library
    if  target == "aarch64-apple-darwin" {
        println!("cargo:rustc-link-search=native={}", lib_dir);

        // TODO
        // there is some problem with the build system
        // it looks for dynamic library with .dylib
        // right now, I am manually copied the files to the /target/TARGET/deps folder
        // might not work in the future
        println!("cargo:rustc-link-lib=dylib=crypto");
        println!("cargo:rustc-link-lib=dylib=ssl");
        println!("cargo:rustc-link-lib=dylib=solclient");
        println!("cargo:rustc-link-lib=dylib=solclientssl");

        //println!("cargo:rustc-link-lib=static=crypto");
        //println!("cargo:rustc-link-lib=static=ssl");
        //println!("cargo:rustc-link-lib=static=solclient");
        //println!("cargo:rustc-link-lib=static=solclientssl");
    } else {
        panic!("Unknown target {}", target)
    }
}
