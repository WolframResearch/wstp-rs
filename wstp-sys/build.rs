//! This script links the Mathematica WSTPi4 library.
//!
//! It does this by finding the local Mathematica installation by using the users
//! `wolframscript` to evaluate `$InstallationDirectory`. This script will fail if
//! `wolframscript` is not on `$PATH`.
extern crate bindgen;

use cfg_if::cfg_if;

use std::env;
use std::path::PathBuf;
use std::process;

const WSTP_FRAMEWORK: &str = "wstp.framework/";
const WSTP_STATIC_ARCHIVE: &str = "libWSTPi4.a";

fn main() {
    // Path to the WSTP SDK 'CompilerAdditions' directory, which contains the libary
    // header files and static and dynamic library files.
    let sdk_compiler_additions = get_compiler_additions_directory();

    println!(
        "cargo:warning=info: Using WSTP CompilerAdditions directory at: {}",
        sdk_compiler_additions.display()
    );

    if !sdk_compiler_additions.is_dir() {
        println!(
            "cargo:error=WSTP CompilerAdditions directory does not exist: {}",
            sdk_compiler_additions.display()
        );
        panic!();
    }

    generate_bindings(&sdk_compiler_additions);
    link_wstp_statically(&sdk_compiler_additions);

    // Note: This blog post explained this, and that this might need to change on Linux.
    //         https://flames-of-code.netlify.com/blog/rust-and-cmake-cplusplus/
    println!("cargo:rustc-link-lib=dylib=c++");

    // TODO: Look at the complete list of CMake libraries required by WSTP and update this
    //       logic for Windows and Linux.
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
}

cfg_if![if #[cfg(all(target_os = "macos", target_arch = "x86_64"))] {
    fn link_wstp_statically(compiler_additions: &PathBuf) {
        let lib = compiler_additions.join(WSTP_STATIC_ARCHIVE);
        let lib = lib.to_str()
            .expect("could not convert WSTP archive path to str");
        let lib = lipo_native_library(&lib);
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
    fn lipo_native_library(wstp_lib: &str) -> PathBuf {
        // `lipo` will return an error if run on a non-universal binary, so avoid doing
        // that by using the `file` command to check the type of `wstp_lib`.
        let is_universal_binary = {
            let stdout = process::Command::new("file")
                .args(&[wstp_lib])
                .output()
                .expect("failed to run `file` system utility").stdout;
            let stdout = String::from_utf8(stdout).unwrap();
            stdout.contains("Mach-O universal binary")
        };

        if !is_universal_binary {
            return PathBuf::from(wstp_lib);
        }

        // Place the lipo'd library file in the system temporary directory.
        let output_lib = std::env::temp_dir().join("libWSTP-x86-64.a");
        let output_lib = output_lib.to_str()
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
} else {
    // FIXME: Add support for Windows and Linux platforms.
    compile_error!("unsupported target platform");
}];

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

fn generate_bindings(compiler_additions: &PathBuf) {
    let header = compiler_additions
        .join(&*WSTP_FRAMEWORK)
        .join("Headers/wstp.h");

    let bindings = bindgen::Builder::default()
        .clang_arg(format!(
            "-I/{}",
            compiler_additions
                .join(&*WSTP_FRAMEWORK)
                .join("Headers/")
                .display()
        ))
        .header(header.display().to_string())
        .generate_comments(true)
        // NOTE: At time of writing this will silently fail to work if you are using a
        //       nightly version of Rust, making the generated bindings almost impossible
        //       to decipher.
        //
        //       Instead, use `$ cargo doc --document-private-items && open target/doc` to
        //       have a look at the generated documentation, which is easier to read and
        //       navigate anyway.
        .rustfmt_bindings(true)
        // Force the WSE* error macro definitions to be interpreted as signed constants.
        // WSTP uses `int` as it's error type, so this is necessary to avoid having to
        // scatter `as i32` everywhere.
        .default_macro_constant_type(bindgen::MacroTypeVariation::Signed)
        .generate()
        .expect("unable to generate Rust bindings to WSTP using bindgen");

    let filename = "WSTP_bindings.rs";
    // OUT_DIR is set by cargo before running this build.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join(filename);
    bindings
        .write_to_file(out_path)
        .expect("failed to write Rust bindings with IO error");
}

//======================================
// Path lookup
//======================================

fn get_compiler_additions_directory() -> PathBuf {
    if let Some(path) = get_env_var("WSTP_COMPILER_ADDITIONS") {
        let path = PathBuf::from(path);
        // Force a rebuild if the path has changed. This happens when developing WSTP.
        println!("cargo:rerun-if-changed={}", path.display());
        return path;
    }

    get_wolfram_installation()
        .join("SystemFiles/Links/WSTP/DeveloperKit/")
        .join(SYSTEM_ID)
        .join("CompilerAdditions")
}

cfg_if![
    if #[cfg(all(target_os = "macos", target_arch = "x86_64"))] {
        const SYSTEM_ID: &str = "MacOSX-x86-64";
    } else {
        // FIXME: Update this to include common Linux/Windows (and ARM macOS)
        //        platforms.
        compile_error!("wstp-sys is unimplemented for this platform");
    }
];

/// Evaluate `$InstallationDirectory` using wolframscript to get location of the
/// developers Mathematica installation.
///
/// TODO: Make this value settable using an environment variable; some people don't have
///       wolframscript, or they may have multiple Mathematica installations and will want
///       to be able to exactly specify which one to use. WOLFRAM_INSTALLATION_DIRECTORY.
fn get_wolfram_installation() -> PathBuf {
    let output: process::Output = process::Command::new("wolframscript")
        .args(&["-code", "$InstallationDirectory"])
        .output()
        .expect("unable to execute wolframscript command");

    // NOTE: The purpose of the 2nd clause here checking for exit code 3 is to work around
    //       a mis-feature of wolframscript to return the same exit code as the Kernel.
    // TODO: Fix the bug in wolframscript which makes this necessary and remove the check
    //       for `3`.
    if !output.status.success() && output.status.code() != Some(3) {
        panic!(
            "wolframscript exited with non-success status code: {}",
            output.status
        );
    }

    let stdout = match String::from_utf8(output.stdout.clone()) {
        Ok(s) => s,
        Err(err) => {
            panic!(
                "wolframscript output is not valid UTF-8: {}: {}",
                err,
                String::from_utf8_lossy(&output.stdout)
            );
        },
    };

    let first_line = stdout
        .lines()
        .next()
        .expect("wolframscript output was empty");

    PathBuf::from(first_line)
}

fn get_env_var(var: &'static str) -> Option<String> {
    println!("cargo:rerun-if-env-changed={}", var);
    match std::env::var(var) {
        Ok(string) => Some(string),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(err)) => {
            panic!("value of env var '{}' is not valid unicode: {:?}", var, err)
        },
    }
}
