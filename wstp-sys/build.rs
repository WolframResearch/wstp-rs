//! This script links the Mathematica WSTPi4 library.
//!
//! It does this by finding the local Mathematica installation by using the users
//! `wolframscript` to evaluate `$InstallationDirectory`. This script will fail if
//! `wolframscript` is not on `$PATH`.


use std::path::PathBuf;
use std::process;

use wolfram_app_discovery::{SystemID, WolframApp, WolframVersion};

/// Oldest Wolfram Version that wstp-rs aims to be compatible with.
const WOLFRAM_VERSION: WolframVersion = WolframVersion::new(13, 0, 1);

fn main() {
    env_logger::init();

    // Ensure that changes to environment variables checked by wolfram-app-discovery will
    // cause cargo to rebuild the current crate.
    wolfram_app_discovery::config::set_print_cargo_build_script_directives(true);

    // This crate is being built by docs.rs. Skip trying to locate a WolframApp.
    // See: https://docs.rs/about/builds#detecting-docsrs
    if std::env::var("DOCS_RS").is_ok() {
        // Force docs.rs to use the bindings generated for this version / system.
        let bindings_path = make_bindings_path(&WOLFRAM_VERSION, SystemID::MacOSX_x86_64);

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

    let app: Option<WolframApp> = WolframApp::try_default().ok();

    let target_system_id: SystemID =
        SystemID::try_from_rust_target(&std::env::var("TARGET").unwrap())
            .expect("unable to get System ID for target system");

    //-------------
    // Link to WSTP
    //-------------

    link_to_wstp(app.as_ref());

    //----------------------------------------------------
    // Generate or use pre-generated Rust bindings to WSTP
    //----------------------------------------------------
    // See docs/Maintenance.md for instructions on how to pre-generate
    // bindings for new WL versions.

    // TODO: Update to a higher minimum WSTP version and remove this workaround.
    // NOTE: WSTP didn't support 64-bit ARM Linux in v13.0.1, so pre-generated
    //       bindings aren't available. If starting Linux-ARM64, use bindings
    //       from a newer version. (This mismatch is neglible since there were
    //       no significant API changes to WSTP between these two versions anyway.)
    let wolfram_version = match target_system_id {
        SystemID::Linux_ARM64 => WolframVersion::new(13, 2, 0),
        _ => WOLFRAM_VERSION
    };

    // TODO: Make use of pre-generated bindings useable via a feature flag?
    //       Using pre-generated bindings seems to currently only have a distinct
    //       advantage over compile-time-generated bindings when building on
    //       docs.rs, where the WSTP SDK is not available.
    //
    //       In other situations, using pre-generated bindings doesn't offer the
    //       advantage of not needing the WSTP SDK available locally, because you
    //       still need to link against the WSTP static library.
    //
    //       NOTE: Pre-generated bindings have the advantage of working when
    //             libclang is not available (which bindgen requires), which
    //             happens e.g. in Windows CI/CD builds.
    let bindings_path = use_pregenerated_bindings(wolfram_version, target_system_id);

    println!(
        "cargo:rustc-env=CRATE_WSTP_SYS_BINDINGS={}",
        bindings_path.display()
    );
}

//========================================================================
// Tell `lib.rs` where to find the file containing the WSTP Rust bindings.
//========================================================================

//-----------------------
// Pre-generated bindings
//-----------------------

/// Use bindings that have been pre-generated.
#[allow(dead_code)]
fn use_pregenerated_bindings(wolfram_version: WolframVersion, target_system_id: SystemID) -> PathBuf {
    // FIXME: Check that this file actually exists, and generate a nicer error if it
    //        doesn't.
    let bindings_path = make_bindings_path(&wolfram_version, target_system_id);

    println!("cargo:rerun-if-changed={}", bindings_path.display());

    if !bindings_path.is_file() {
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
            wolfram_version, target_system_id
        );
        panic!("<See printed error>");
    }

    println!(
        "cargo:warning=info: using pre-generated bindings for WSTP ({wolfram_version}, {target_system_id}): {}",
        bindings_path.display()
    );

    bindings_path
}

fn make_bindings_path(wolfram_version: &WolframVersion, system_id: SystemID) -> PathBuf {
    let bindings_path = PathBuf::from("generated")
        .join(&wolfram_version.to_string())
        .join(system_id.as_str())
        .join("WSTP_bindings.rs");

    let absolute_bindings_path =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(&bindings_path);

    absolute_bindings_path
}

//======================================
// Link to WSTP
//======================================

/// Emits the necessary `cargo` instructions to link to the WSTP static library,
/// and also links the WSTP interface libraries (the libraries that WSTP itself
/// depends on).
fn link_to_wstp(app: Option<&WolframApp>) {
    // Path to the WSTP static library file.
    let static_lib = wolfram_app_discovery::build_scripts::wstp_static_library_path(app)
        .expect("unable to get WSTP static library path")
        .into_path_buf();

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

    //
    // Linux
    //

    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=uuid")
    }
}

fn link_wstp_statically(lib: &PathBuf) {
    let mut lib = lib.clone();

    if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        lib = lipo_native_library(&lib, "x86_64");
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        lib = lipo_native_library(&lib, "arm64");
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
fn lipo_native_library(wstp_lib: &PathBuf, lipo_arch: &str) -> PathBuf {
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
    let output_lib = std::env::temp_dir().join("libWSTP-thin.a");
    let output_lib = output_lib
        .to_str()
        .expect("could not convert WSTP archive path to str");

    let output = process::Command::new("lipo")
        .args(&[wstp_lib, "-thin", lipo_arch, "-output", output_lib])
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
