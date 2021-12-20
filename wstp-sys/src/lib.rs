#![allow(
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    improper_ctypes
)]

// The name of this file comes from `build.rs`.
include!(concat!(
    "../generated/",
    env!("CRATE_WSTP_SYS_WL_VERSION_NUMBER"),
    "/",
    env!("CRATE_WSTP_SYS_WL_SYSTEM_ID"),
    "/WSTP_bindings.rs"
));
