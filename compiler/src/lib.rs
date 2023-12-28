use std::ffi::{c_char, CString};
use std::ptr::slice_from_raw_parts;
use crate::ast::Project;
use crate::backend::rust::emit_rust;
use crate::scratch_schema::parse;

pub mod scratch_schema;
pub mod ast;
pub mod parse;
pub mod backend;

#[no_mangle]
pub extern "C" fn compile_sb3(project_json: *const u8, len: usize) -> *const c_char {
    let s = unsafe { &*slice_from_raw_parts(project_json, len) };
    let project: Project = parse(std::str::from_utf8(s).unwrap()).unwrap().into();
    let src = emit_rust(&project);
    CString::new(src).unwrap().into_raw()
}
