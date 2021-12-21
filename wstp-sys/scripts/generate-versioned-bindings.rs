//! ```cargo
//! [package]
//! edition = "2021"
//!
//! [dependencies]
//! bindgen = "^0.58.1"
//! wolfram-app-discovery = { git = "ssh://git@stash.wolfram.com:7999/~connorg/wolfram-app-discovery.git" }
//! ```

use std::path::PathBuf;

use wolfram_app_discovery::{WolframApp, WolframVersion};

const WSTP_FRAMEWORK: &str = "wstp.framework/";

const FILENAME: &str = "WSTP_bindings.rs";

fn main() {
    let app = WolframApp::try_default().expect("unable to locate WolframApp");

    // Path to the WSTP SDK 'CompilerAdditions' directory, which contains the libary
    // header files and static and dynamic library files.
    let sdk_compiler_additions = app.wstp_compiler_additions_path()
        .expect("unable to get CompilerAdditions directory");

    if !sdk_compiler_additions.is_dir() {
        panic!(
            "Error: WSTP CompilerAdditions directory does not exist: {}",
            sdk_compiler_additions.display()
        );
    }

    generate_bindings(&app, &sdk_compiler_additions);
}

fn generate_bindings(app: &WolframApp, compiler_additions: &PathBuf) {
    let header = compiler_additions
        .join(&*WSTP_FRAMEWORK)
        .join("Headers/wstp.h");

    let bindings = bindgen::Builder::default()
        // PRE_COMMIT: This is not necessary?
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

    let version: WolframVersion = app.wolfram_version()
        .expect("unable to get WolframVersion");

    // OUT_DIR is set by cargo before running this build.rs file.
    let out_path = out_dir()
        .join("generated")
        .join(&version.to_string())
        .join(wolfram_app_discovery::target_system_id())
        .join(FILENAME);

    std::fs::create_dir_all(out_path.parent().unwrap())
        .expect("failed to create parent directories for generating bindings file");

    bindings
        .write_to_file(&out_path)
        .expect("failed to write Rust bindings with IO error");

    println!(
        "
        ==== GENERATED BINDINGS ====

        WSTP CompilerAdditions Directory: {}

        $SystemID:                        {}

        $VersionNumber / $ReleaseNumber:  {}

        Output:                           <out_dir>/{}

        ============================
        ",
        compiler_additions.display(),
        wolfram_app_discovery::target_system_id(),
        version,
        out_path.strip_prefix(out_dir()).unwrap().display()
    )
}

fn out_dir() -> PathBuf {
    // TODO: Provide a way to override this location using an environment variable.
    std::env::current_dir().expect("unable to get process current working directory")
}
