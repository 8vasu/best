// Best - A statically typed Lisp, implemented in POSIX {lex(1), yacc(1)} and Rust.
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

use std::fmt;

#[derive(Debug, Clone)]
pub struct Loc {
    pub line: i32,
    pub col: i32,
}

impl fmt::Display for Loc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.col)
    }
}

#[derive(Debug)]
pub struct BestError {
    pub message: String,
    pub loc: Option<Loc>,
}

impl BestError {
    pub fn new(message: impl Into<String>) -> Self {
        BestError { message: message.into(), loc: None }
    }

    pub fn at(message: impl Into<String>, loc: &Loc) -> Self {
        BestError { message: message.into(), loc: Some(loc.clone()) }
    }
}

impl fmt::Display for BestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.loc {
            Some(loc) => write!(f, "error at {}: {}", loc, self.message),
            None => write!(f, "error: {}", self.message),
        }
    }
}
