//! ```cargo
//! [package]
//! edition = "2021"
//!
//! [dependencies]
//! bindgen = "^0.58.1"
//! wolfram-app-discovery = "0.2.0"
//! ```

use std::path::PathBuf;

use wolfram_app_discovery::{WolframApp, WolframVersion};

const FILENAME: &str = "WSTP_bindings.rs";

fn main() {
    let app = WolframApp::try_default().expect("unable to locate WolframApp");

    generate_bindings(&app);
}

fn generate_bindings(app: &WolframApp) {
    // Path to the WSTP SDK 'wstp.h` header file.
    let wstp_h = app
        .wstp_c_header_path()
        .expect("unable to get 'wstp.h' location");

    assert!(wstp_h.file_name().unwrap() == "wstp.h");

    let bindings = bindgen::Builder::default()
        .header(wstp_h.display().to_string())
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

    let version: WolframVersion =
        app.wolfram_version().expect("unable to get WolframVersion");

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

        wstp.h location: {}

        $SystemID:                        {}

        $VersionNumber / $ReleaseNumber:  {}

        Output:                           <out_dir>/{}

        ============================
        ",
        wstp_h.display(),
        wolfram_app_discovery::target_system_id(),
        version,
        out_path.strip_prefix(out_dir()).unwrap().display()
    )
}

fn out_dir() -> PathBuf {
    // TODO: Provide a way to override this location using an environment variable.
    std::env::current_dir().expect("unable to get process current working directory")
}
