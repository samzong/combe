#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub fn resources_dir() -> &'static str {
    env!("GHOSTTY_RESOURCES_DIR")
}
