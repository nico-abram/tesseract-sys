extern crate bindgen;

#[cfg(target_os = "macos")]
use pkg_config;
use std::env;
use std::path::PathBuf;
#[cfg(windows)]
use vcpkg;

#[cfg(windows)]
fn find_tesseract_system_lib() -> Vec<String> {
    println!("cargo:rerun-if-env-changed=TESSERACT_INCLUDE_PATHS");
    println!("cargo:rerun-if-env-changed=TESSERACT_LINK_PATHS");
    println!("cargo:rerun-if-env-changed=TESSERACT_LINK_LIBS");

    let vcpkg = || {
        let lib = vcpkg::Config::new().find_package("tesseract").unwrap();

        vec![lib
            .include_paths
            .iter()
            .map(|x| x.to_string_lossy())
            .collect::<String>()]
    };

    let include_paths = env::var("TESSERACT_INCLUDE_PATHS").ok();
    let include_paths = include_paths.as_deref().map(|x| x.split(','));
    let link_paths = env::var("TESSERACT_LINK_PATHS").ok();
    let link_paths = link_paths.as_deref().map(|x| x.split(','));
    let link_libs = env::var("TESSERACT_LINK_LIBS").ok();
    let link_libs = link_libs.as_deref().map(|x| x.split(','));
    if let (Some(include_paths), Some(link_paths), Some(link_libs)) =
        (include_paths, link_paths, link_libs)
    {
        for link_path in link_paths {
            println!("cargo:rustc-link-search={}", link_path)
        }

        for link_lib in link_libs {
            println!("cargo:rustc-link-lib={}", link_lib)
        }

        include_paths.map(|x| x.to_string()).collect::<Vec<_>>()
    } else {
        vcpkg()
    }
}

// we sometimes need additional search paths, which we get using pkg-config
// we can use tesseract installed anywhere on Linux.
// if you change install path(--prefix) to `configure` script.
// set `export PKG_CONFIG_PATH=/path-to-lib/pkgconfig` before.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn find_tesseract_system_lib() -> Vec<String> {
    let pk = pkg_config::Config::new().probe("tesseract").unwrap();
    // Tell cargo to tell rustc to link the system proj shared library.
    println!("cargo:rustc-link-search=native={:?}", pk.link_paths[0]);
    println!("cargo:rustc-link-lib=tesseract");

    let mut include_paths = pk.include_paths.clone();
    include_paths
        .iter_mut()
        .map(|x| {
            if !x.ends_with("include") {
                x.pop();
            }
            x
        })
        .map(|x| x.to_string_lossy())
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
}

#[cfg(all(not(windows), not(target_os = "macos"), not(target_os = "linux")))]
fn find_tesseract_system_lib() -> Vec<String> {
    println!("cargo:rustc-link-lib=tesseract");
    vec![]
}

fn main() {
    // Tell cargo to tell rustc to link the system tesseract
    // and leptonica shared libraries.
    let clang_extra_include = find_tesseract_system_lib();

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let mut capi_bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper_capi.h")
        .whitelist_function("^Tess.*")
        .blacklist_type("Boxa")
        .blacklist_type("Pix")
        .blacklist_type("Pixa")
        .blacklist_type("_IO_FILE")
        .blacklist_type("_IO_codecvt")
        .blacklist_type("_IO_marker")
        .blacklist_type("_IO_wide_data");

    for inc in &clang_extra_include {
        capi_bindings = capi_bindings.clang_arg(format!("-I{}", *inc));
    }

    // Finish the builder and generate the bindings.
    let capi_bindings = capi_bindings
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate capi bindings");

    let mut public_types_bindings = bindgen::Builder::default()
        .header("wrapper_public_types.hpp")
        .whitelist_var("^k.*")
        .blacklist_item("kPolyBlockNames");

    for inc in &clang_extra_include {
        public_types_bindings = public_types_bindings.clang_arg(format!("-I{}", *inc));
    }

    let public_types_bindings = public_types_bindings
        .generate()
        .expect("Unable to generate public types bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    capi_bindings
        .write_to_file(out_path.join("capi_bindings.rs"))
        .expect("Couldn't write capi bindings!");
    public_types_bindings
        .write_to_file(out_path.join("public_types_bindings.rs"))
        .expect("Couldn't write public types bindings!");
}
