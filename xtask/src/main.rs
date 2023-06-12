//! `cargo xtask` helper commands for the wstp-rs project.
//!
//! This crate follows the [`cargo xtask`](https://github.com/matklad/cargo-xtask)
//! convention.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use wolfram_app_discovery::{SystemID, WolframApp, WolframVersion, WstpSdk};

const FILENAME: &str = "WSTP_bindings.rs";

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate and save WSTP bindings for the current platform.
    GenBindings {
        /// Target to generate bindings for.
        #[arg(long)]
        target: Option<String>,
    },
}

//======================================
// Main
//======================================

fn main() {
    let Cli {
        command: Commands::GenBindings { target },
    } = Cli::parse();

    let app = WolframApp::try_default().expect("unable to locate WolframApp");

    let wolfram_version: WolframVersion =
        app.wolfram_version().expect("unable to get WolframVersion");

    let wstp_sdks: Vec<WstpSdk> = app
        .wstp_sdks()
        .expect("unable to locate WSTP SDKs in app")
        .into_iter()
        .filter_map(|entry| entry.ok())
        .collect();

    let targets: Vec<&str> = match target {
        Some(ref target) => vec![target.as_str()],
        None => determine_targets().to_vec(),
    };

    println!("Generating bindings for: {targets:?}");

    for target in targets {
        let target_system_id = SystemID::try_from_rust_target(target).unwrap();

        // Find the WSTP SDK suitable for the specified Rust target.
        let sdk: Option<&WstpSdk> = wstp_sdks
            .iter()
            .find(|sdk| sdk.system_id() == target_system_id);

        let Some(sdk) = sdk else {
            println!("WARNING: App does not provide WSTP SDK for {target_system_id} (Rust target: {target}).");
            continue
        };

        // Path to the WSTP SDK 'wstp.h` header file.
        let wstp_h = sdk.wstp_c_header_path();

        generate_bindings(&wolfram_version, &wstp_h, target);
    }
}

/// Generte bindings for multiple targets at once, based on the current
/// operating system.
fn determine_targets() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["x86_64-apple-darwin", "aarch64-apple-darwin"]
    } else if cfg!(target_os = "windows") {
        &["x86_64-pc-windows-msvc"]
    } else if cfg!(target_os = "linux") {
        &["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]
    } else {
        panic!("unsupported operating system for determining LibraryLink bindings target architecture")
    }
}

fn generate_bindings(wolfram_version: &WolframVersion, wstp_h: &Path, target: &str) {
    assert!(wstp_h.file_name().unwrap() == "wstp.h");

    let target_system_id: SystemID = SystemID::try_from_rust_target(target)
        .expect("Rust target doesn't map to a known SystemID");

    let bindings = bindgen::Builder::default()
        .header(wstp_h.display().to_string())
        .generate_comments(true)
        // Force the WSE* error macro definitions to be interpreted as signed constants.
        // WSTP uses `int` as it's error type, so this is necessary to avoid having to
        // scatter `as i32` everywhere.
        .default_macro_constant_type(bindgen::MacroTypeVariation::Signed)
        .clang_args(&["-target", target])
        .generate()
        .expect("unable to generate Rust bindings to WSTP using bindgen");

    // OUT_DIR is set by cargo before running this build.rs file.
    let out_path = repo_root_dir()
        .join("wstp-sys")
        .join("generated")
        .join(&wolfram_version.to_string())
        .join(target_system_id.as_str())
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

        Output:                           {}

        ============================
        ",
        wstp_h.display(),
        target_system_id,
        wolfram_version,
        out_path
            .strip_prefix(repo_root_dir())
            .unwrap()
            .display()
    )
}

fn repo_root_dir() -> PathBuf {
    let xtask_crate = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    assert!(xtask_crate.file_name().unwrap() == "xtask");
    xtask_crate.parent().unwrap().to_path_buf()
}