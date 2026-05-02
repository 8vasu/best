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

mod ast;
mod env;
mod error;
mod eval;
mod typecheck;

use std::ffi::CString;
use std::os::raw::c_int;

extern "C" {
    fn parse_file(filename: *const std::ffi::c_char) -> c_int;
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: best <source-file>");
        std::process::exit(1);
    }

    let filename = CString::new(args[1].as_str()).expect("filename contains null byte");

    let rc = unsafe { parse_file(filename.as_ptr()) };
    if rc != 0 {
        std::process::exit(1);
    }

    let program = ast::take_program();

    if let Err(e) = typecheck::typecheck(&program) {
        eprintln!("{e}");
        std::process::exit(1);
    }

    if let Err(e) = eval::eval_program(&program) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
