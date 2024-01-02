use clap::ValueEnum;

pub mod scratch_schema;
pub mod ast;
pub mod parse;
pub mod backend;

pub mod wasm_interface {
    use std::alloc::{alloc, Layout};
    use std::ffi::{c_char, CStr, CString};
    use std::ptr::slice_from_raw_parts;
    use crate::ast::Project;
    use crate::backend::rust::{emit_rust, make_cargo_toml};
    use crate::scratch_schema::parse;
    use crate::Target;

    /// len does NOT include null terminator.
    #[no_mangle]
    pub unsafe extern "C" fn compile_sb3(project_json: *const u8, len: usize) -> *const c_char {
        let s = &*slice_from_raw_parts(project_json, len);
        let project: Project = parse(std::str::from_utf8(s).unwrap()).unwrap().into();
        let src = emit_rust(&project, Target::Notan);
        CString::new(src).unwrap().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn get_cargo_toml() -> *const c_char {
        CString::new(make_cargo_toml(Target::Notan)).unwrap().into_raw()
    }

    /// len DOES include null terminator
    #[no_mangle]
    pub unsafe extern "C" fn alloc_str(len: usize) -> *mut u8 {
        alloc(Layout::array::<u8>(len).unwrap())
    }

    #[no_mangle]
    pub unsafe extern "C" fn drop_c_str(ptr: *mut c_char) {
        let _ = CString::from_raw(ptr);
    }

    /// result does NOT include null terminator.
    #[no_mangle]
    pub unsafe extern "C" fn c_str_len(ptr: *mut c_char) -> usize {
        CStr::from_ptr(ptr).to_bytes().len()
    }
}


#[cfg_attr(feature = "cli", derive(ValueEnum))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AssetPackaging {
    Embed,
    Fetch,
}

#[cfg_attr(feature = "cli", derive(ValueEnum))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Target {
    Notan,
    Softbuffer,
}

impl Target {
    fn code_name(&self) -> &str {
        match self {
            Target::Notan => "notan",
            Target::Softbuffer => "softbuffer",
        }
    }
}
