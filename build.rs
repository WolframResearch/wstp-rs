extern crate bindgen;

use std::env;
use std::path::PathBuf;
use std::process;

const WSTP_FRAMEWORK: &str = "Frameworks/wstp.framework/";
const WSTP_STATIC_ARCHIVE: &str =
    "SystemFiles/Links/WSTP/DeveloperKit/MacOSX-x86-64/CompilerAdditions/";

fn main() {
    let installation = get_wolfram_installation();

    println!(
        "cargo:warning=info: Using Wolfram installation at: {}",
        installation.display()
    );

    // if !WSTP_FRAMEWORK.exists() {
    //     // NOTE: For WRI developers, if the Mathematica installation at this path is a
    //     //       prototype / custom Kernel build, it's
    //     panic!("no Wolfram System WSTP framework files exist at '{}'",
    //            WSTP_FRAMEWORK.display());
    // }

    generate_bindings(&installation);
    link_wstp_statically(&installation);
}

fn link_wstp_statically(installation: &PathBuf) {
    println!(
        "cargo:rustc-link-search={}",
        installation.join(&*WSTP_STATIC_ARCHIVE).display()
    );
    println!("cargo:rustc-link-lib=static=WSTPi4-x86");
}

fn generate_bindings(installation: &PathBuf) {
    let header = installation.join(&*WSTP_FRAMEWORK).join("Headers/wstp.h");

    let bindings = bindgen::Builder::default()
        .clang_arg(format!(
            "-I/{}",
            installation
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
        .generate()
        .expect("unable to generate Rust bindings to WSTP using bindgen");

    let filename = "WSTP_bindings.rs";
    // OUT_DIR is set by cargo before running this build.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join(filename);
    bindings
        .write_to_file(out_path)
        .expect("failed to write Rust bindings with IO error");
}

fn get_wolfram_installation() -> PathBuf {
    let output: process::Output = process::Command::new("wolframscript")
        .args(&["-code", "$InstallationDirectory"])
        .output()
        .expect("unable to execute wolframscript command");

    if !output.status.success() {
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
        }
    };

    let first_line = stdout
        .lines()
        .next()
        .expect("wolframscript output was empty");

    PathBuf::from(first_line)
}
