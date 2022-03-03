#![allow(
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    improper_ctypes
)]

// Ensure that linker flags from link-cplusplus are used.
extern crate link_cplusplus;


// The name of this file comes from `build.rs`.
include!(env!("CRATE_WSTP_SYS_BINDINGS"));
