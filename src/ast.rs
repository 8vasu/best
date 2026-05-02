// Best - A statically typed Lisp, implemented with POSIX lex/yacc and Rust.
// Copyright (C) 2026 Soumendra Ganguly

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::ffi::CStr;
use std::os::raw::{c_char, c_double, c_int, c_longlong};
use std::sync::Mutex;

use crate::error::Loc;

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64, Loc),
    Float(f64, Loc),
    Bool(bool, Loc),
    Str(String, Loc),
    Id(String, Loc),
    TypeName(String, Loc),
    List(Vec<Expr>, Loc),
}

impl Expr {
    pub fn loc(&self) -> &Loc {
        match self {
            Expr::Int(_, l) | Expr::Float(_, l) | Expr::Bool(_, l)
            | Expr::Str(_, l) | Expr::Id(_, l) | Expr::TypeName(_, l)
            | Expr::List(_, l) => l,
        }
    }
}

static PROGRAM: Mutex<Option<Vec<Expr>>> = Mutex::new(None);

pub fn take_program() -> Vec<Expr> {
    PROGRAM.lock().unwrap().take().unwrap_or_default()
}

unsafe fn c_str(ptr: *const c_char) -> String {
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

fn mk_loc(line: c_int, col: c_int) -> Loc {
    Loc { line, col }
}

#[no_mangle]
pub extern "C" fn best_make_int(val: c_longlong, line: c_int, col: c_int) -> *mut Expr {
    Box::into_raw(Box::new(Expr::Int(val as i64, mk_loc(line, col))))
}

#[no_mangle]
pub extern "C" fn best_make_float(val: c_double, line: c_int, col: c_int) -> *mut Expr {
    Box::into_raw(Box::new(Expr::Float(val, mk_loc(line, col))))
}

#[no_mangle]
pub extern "C" fn best_make_bool(val: c_int, line: c_int, col: c_int) -> *mut Expr {
    Box::into_raw(Box::new(Expr::Bool(val != 0, mk_loc(line, col))))
}

#[no_mangle]
pub extern "C" fn best_make_str(val: *const c_char, line: c_int, col: c_int) -> *mut Expr {
    let s = unsafe { c_str(val) };
    let s = s.trim_matches('"').to_string();
    Box::into_raw(Box::new(Expr::Str(s, mk_loc(line, col))))
}

#[no_mangle]
pub extern "C" fn best_make_id(val: *const c_char, line: c_int, col: c_int) -> *mut Expr {
    let s = unsafe { c_str(val) };
    Box::into_raw(Box::new(Expr::Id(s, mk_loc(line, col))))
}

#[no_mangle]
pub extern "C" fn best_make_type_node(val: *const c_char, line: c_int, col: c_int) -> *mut Expr {
    let s = unsafe { c_str(val) };
    Box::into_raw(Box::new(Expr::TypeName(s, mk_loc(line, col))))
}

#[no_mangle]
pub extern "C" fn best_list_new(expr: *mut Expr) -> *mut Vec<Expr> {
    let mut v = Box::new(Vec::new());
    v.push(unsafe { *Box::from_raw(expr) });
    Box::into_raw(v)
}

#[no_mangle]
pub extern "C" fn best_list_append(list: *mut Vec<Expr>, expr: *mut Expr) -> *mut Vec<Expr> {
    let mut v = unsafe { Box::from_raw(list) };
    v.push(unsafe { *Box::from_raw(expr) });
    Box::into_raw(v)
}

#[no_mangle]
pub extern "C" fn best_list_empty() -> *mut Vec<Expr> {
    Box::into_raw(Box::new(Vec::new()))
}

#[no_mangle]
pub extern "C" fn best_make_list(exprs: *mut Vec<Expr>, line: c_int, col: c_int) -> *mut Expr {
    let v = unsafe { *Box::from_raw(exprs) };
    Box::into_raw(Box::new(Expr::List(v, mk_loc(line, col))))
}

#[no_mangle]
pub extern "C" fn best_make_empty_list(line: c_int, col: c_int) -> *mut Expr {
    Box::into_raw(Box::new(Expr::List(vec![], mk_loc(line, col))))
}

#[no_mangle]
pub extern "C" fn best_set_program(list: *mut Vec<Expr>) {
    let v = unsafe { *Box::from_raw(list) };
    *PROGRAM.lock().unwrap() = Some(v);
}
