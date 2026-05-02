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

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let lex_src = manifest_dir.join("best.l");
    let y_src = manifest_dir.join("best.y");
    let lex_out = out_dir.join("lex.yy.c");
    let ytab_c = out_dir.join("y.tab.c");

    let flex_ok = Command::new("flex")
        .args(["--posix", "-o"])
        .arg(&lex_out)
        .arg(&lex_src)
        .status()
        .expect("flex not found -- run: sudo apt install flex")
        .success();
    if !flex_ok {
        panic!("flex failed on best.l");
    }

    let bison_ok = Command::new("bison")
        .args(["--yacc", "-d", "-o"])
        .arg(&ytab_c)
        .arg(&y_src)
        .status()
        .expect("bison not found -- run: sudo apt install bison")
        .success();
    if !bison_ok {
        panic!("bison failed on best.y");
    }

    cc::Build::new()
        .file(&lex_out)
        .file(&ytab_c)
        .include(&out_dir)
        .warnings(false)
        .compile("best_parser");

    println!("cargo:rerun-if-changed=best.l");
    println!("cargo:rerun-if-changed=best.y");
}
