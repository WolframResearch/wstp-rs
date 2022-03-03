//! This script links the Mathematica WSTPi4 library.
//!
//! It does this by finding the local Mathematica installation by using the users
//! `wolframscript` to evaluate `$InstallationDirectory`. This script will fail if
//! `wolframscript` is not on `$PATH`.


use std::path::PathBuf;
use std::process;

use wolfram_app_discovery::WolframApp;

fn main() {
    // This crate is being built by docs.rs. Skip trying to locate a WolframApp.
    // See: https://docs.rs/about/builds#detecting-docsrs
    if std::env::var("DOCS_RS").is_ok() {
        // Force docs.rs to use the bindings generated for this version / system.
        let bindings_path = make_bindings_path("13.0.0", "MacOSX-x86-64");

        // This environment variable is included using `env!()`. wstp-sys will fail to
        // build if it is not set correctly.
        println!(
            "cargo:rustc-env=CRATE_WSTP_SYS_BINDINGS={}",
            bindings_path.display()
        );

        return;
    }

    //
    // Error if this is a cross compilation
    //

    let host = std::env::var("HOST").expect("expected 'HOST' env var to be set");
    let target = std::env::var("TARGET").expect("expected 'TARGET' env var to be set");

    // Note: `host == target` is required for the use of `cfg!(..)` in this
    //       script to be valid.
    if host != target {
        panic!(
            "error: crate wstp-sys does not support cross compilation. (host: {}, target: {})",
            host,
            target
        );
    }



    let app = WolframApp::try_default().expect("unable to locate WolframApp");

    //-------------
    // Link to WSTP
    //-------------

    // Path to the WSTP static library file.
    let static_lib = &app
        .wstp_static_library_path()
        .expect("unable to get WSTP static library path");

    link_wstp_statically(&static_lib);

    //
    // Link to the C++ standard library, required by WSTP
    //

    // Note: This is now handled by the `link-cplusplus` crate dependency.

    // Note: This blog post explained this, and that this might need to change on Linux.
    //         https://flames-of-code.netlify.com/blog/rust-and-cmake-cplusplus/
    // println!("cargo:rustc-link-lib=dylib=c++");

    //-----------------------------------
    // Link to WSTP "interface" libraries
    //-----------------------------------

    // The CompilerAdditions/WSTP-targets.cmake file describes the dependencies
    // of the WSTP library that must be linked into the final artifact for any
    // code that depends on WSTP. (The contents of that file differ on each
    // platform). They are the `INTERFACE_LINK_LIBRARIES` of the
    // `WSTP::STATIC_LIBRARY` CMake target.
    //
    // On macOS, the Foundation framework is the only dependency. On Windows,
    // several system libraries must be linked.
    //
    // FIXME: Update this logic to cover the Linux interface libraries.

    //
    // macOS
    //

    // TODO: Look at the complete list of CMake libraries required by WSTP and update this
    //       logic for Windows and Linux.
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=Foundation");
    }

    //
    // Windows
    //

    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=dylib=kernel32");
        println!("cargo:rustc-link-lib=dylib=user32");
        println!("cargo:rustc-link-lib=dylib=advapi32");
        println!("cargo:rustc-link-lib=dylib=comdlg32");
        println!("cargo:rustc-link-lib=dylib=ws2_32");
        println!("cargo:rustc-link-lib=dylib=wsock32");
        println!("cargo:rustc-link-lib=dylib=rpcrt4");
    }

    //---------------------------------------------------------------
    // Choose the pre-generated bindings to use for the target system
    //---------------------------------------------------------------
    // See docs/Development.md for instructions on how to generate
    // bindings for new WL versions.

    let wolfram_version = app
        .wolfram_version()
        .expect("unable to get Wolfram Language vesion number");
    let system_id =
        wolfram_app_discovery::system_id_from_target(&std::env::var("TARGET").unwrap())
            .expect("unable to get System ID for target system");

    // FIXME: Check that this file actually exists, and generate a nicer error if it
    //        doesn't.

    let bindings_path = make_bindings_path(&wolfram_version.to_string(), &system_id);

    println!("cargo:rerun-if-changed={}", bindings_path.display());

    let absolute_bindings_path =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(&bindings_path);

    if !absolute_bindings_path.is_file() {
        println!(
            "
    ==== ERROR: wstp-sys =====

    Rust bindings for Wolfram WSTP for target configuration:

        WolframVersion:    {}
        SystemID:          {}

    have not been pre-generated.

    See wstp-sys/generated/ for a listing of currently available targets.

    =========================================
            ",
            wolfram_version, system_id
        );
        panic!("<See printed error>");
    }

    println!(
        "cargo:rustc-env=CRATE_WSTP_SYS_BINDINGS={}",
        bindings_path.display()
    );
}

fn make_bindings_path(wolfram_version: &str, system_id: &str) -> PathBuf {
    let bindings_path = PathBuf::from("generated")
        .join(wolfram_version)
        .join(system_id)
        .join("WSTP_bindings.rs");

    bindings_path
}

fn link_wstp_statically(lib: &PathBuf) {
    let mut lib = lib.clone();

    if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        lib = lipo_native_library(&lib);
    }

    link_library_file(lib);
}

/* NOTE:
    This code was necessary prior to 12.1, where the versions of WSTP in the
    Mathematica layout were univeral binaries containing 32-bit and 64-bit copies of
    the libary. However, it appears that starting with 12.1, the layout build of
    libWSTP is no longer a "fat" archive. (This is possibly due to the fact that macOS
    Catalina, released ~6 months prior, and dropped support for 32-bit applications on
    macOS.)

    I'm electing to leave this code around in the meantime, in case the situation
    changes, but it appears this `lipo` operation may no longer be necessary.

    Update: This code is still useful, because the advent of ARM macOS machines means
            that local development builds of WSTP will build universal x86_64 and
            arm64 binaries by default on macOS.
*/
/// Use the macOS `lipo` command to construct an x86_64 archive file from the WSTPi4.a
/// file in the Mathematica layout. This is necessary as a workaround to a bug in the
/// Rust compiler at the moment: https://github.com/rust-lang/rust/issues/50220.
/// The problem is that WSTPi4.a is a so called "universal binary"; it's an archive
/// file with multiple copies of the same library, each for a different target
/// architecture. The `lipo -thin` command creates a new archive which contains just
/// the library for the named architecture.
fn lipo_native_library(wstp_lib: &PathBuf) -> PathBuf {
    let wstp_lib = wstp_lib
        .to_str()
        .expect("could not convert WSTP archive path to str");

    // `lipo` will return an error if run on a non-universal binary, so avoid doing
    // that by using the `file` command to check the type of `wstp_lib`.
    let is_universal_binary = {
        let stdout = process::Command::new("file")
            .args(&[wstp_lib])
            .output()
            .expect("failed to run `file` system utility")
            .stdout;
        let stdout = String::from_utf8(stdout).unwrap();
        stdout.contains("Mach-O universal binary")
    };

    if !is_universal_binary {
        return PathBuf::from(wstp_lib);
    }

    // Place the lipo'd library file in the system temporary directory.
    let output_lib = std::env::temp_dir().join("libWSTP-x86-64.a");
    let output_lib = output_lib
        .to_str()
        .expect("could not convert WSTP archive path to str");

    let output = process::Command::new("lipo")
        .args(&[wstp_lib, "-thin", "x86_64", "-output", output_lib])
        .output()
        .expect("failed to invoke macOS `lipo` command");

    if !output.status.success() {
        panic!("unable to lipo WSTP library: {:#?}", output);
    }

    PathBuf::from(output_lib)
}

fn link_library_file(libfile: PathBuf) {
    let search_dir = libfile.parent().unwrap().display().to_string();

    let libname = libfile
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .trim_start_matches("lib");
    println!("cargo:rustc-link-search={}", search_dir);
    println!("cargo:rustc-link-lib=static={}", libname);
}
