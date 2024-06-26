
// TODO: list of valid template names for warning if misspell
// TODO: arg for just print default value of template.
// TODO: trailing comma?
// TODO: allow `name` instead of `name=name`
#[macro_export] macro_rules! template {
    ($opts:expr, $id:literal, $($name:ident=$value:expr),*) => {
        match $opts.get_template_path($id) {
            None => template!($id, $($name = $value),*),
            Some(path) => {
                let content = fs::read_to_string(path)?;
                // TODO: apply replacements
                // todo!("cli override template {} to {path} containing {content}", $id)
                content
            }
        }
    };

    // TODO: figure out absolute paths so can include the data/
    ($id:literal, $($name:ident=$value:expr),*) => {
        format!(include!(concat!($id, ".txt")), $($name = $value),*)
    };
}


pub mod scratch_schema;
pub mod ast;
pub mod parse;
pub mod backend;

#[cfg(feature = "cli")]
pub mod cli;
mod infer;

pub mod wasm_interface {
    use std::alloc::{alloc, Layout};
    use std::ffi::{c_char, CStr, CString};
    use std::ptr::slice_from_raw_parts;
    use crate::ast::Project;
    use crate::backend::rust::{emit_rust};
    use crate::scratch_schema::parse;
    use crate::{AssetPackaging, Target};

    /// len does NOT include null terminator.
    #[no_mangle]
    pub unsafe extern "C" fn compile_sb3(project_json: *const u8, len: usize) -> *const c_char {
        let s = &*slice_from_raw_parts(project_json, len);
        let project: Project = parse(std::str::from_utf8(s).unwrap()).unwrap().into();
        let src = emit_rust(&project, Target::Macroquad, AssetPackaging::Fetch);
        CString::new(src).unwrap().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn get_cargo_toml() -> *const c_char {
        CString::new(String::from("TODO: ffi get_cargo_toml")).unwrap().into_raw()
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

// TODO: more thought about how this is implemented.
//       for ios put it in assets folder instead of binary?
//       for wasm you might rather always fetch but use the browser api?
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum AssetPackaging {
    #[default]
    Embed,
    Fetch,
}

#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum Target {
    Notan,
    Softbuffer,
    #[default]
    Macroquad,
}

impl Target {
    fn code_name(&self) -> &str {
        match self {
            Target::Notan => "notan",
            Target::Softbuffer => "softbuffer",
            Target::Macroquad => "macroquad",
        }
    }
}
