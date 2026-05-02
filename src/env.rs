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

use std::collections::HashMap;

use crate::ast::Expr;
use crate::error::{BestError, Loc};
use crate::typecheck::Type;
use crate::eval::Value;

/* -- Type environment ------------------------------------------------------- */

pub struct TypeEnv {
    var_frames: Vec<HashMap<String, Type>>,
    pub funs: HashMap<String, (Vec<Type>, Type)>,
}

impl TypeEnv {
    pub fn new(funs: HashMap<String, (Vec<Type>, Type)>) -> Self {
        TypeEnv { var_frames: vec![HashMap::new()], funs }
    }

    pub fn push_frame(&mut self) {
        self.var_frames.push(HashMap::new());
    }

    pub fn pop_frame(&mut self) {
        self.var_frames.pop();
    }

    pub fn declare_var(&mut self, name: &str, ty: Type) {
        self.var_frames.last_mut().unwrap().insert(name.to_string(), ty);
    }

    pub fn lookup_var(&self, name: &str, loc: &Loc) -> Result<Type, BestError> {
        for frame in self.var_frames.iter().rev() {
            if let Some(ty) = frame.get(name) {
                return Ok(ty.clone());
            }
        }
        Err(BestError::at(format!("undefined variable '{name}'"), loc))
    }

    pub fn lookup_fun(&self, name: &str, loc: &Loc) -> Result<(Vec<Type>, Type), BestError> {
        self.funs.get(name).cloned()
            .ok_or_else(|| BestError::at(format!("undefined function '{name}'"), loc))
    }
}

/* -- Eval environment ------------------------------------------------------- */

#[derive(Clone)]
pub struct Function {
    pub params: Vec<String>,
    pub body: Expr,
}

pub struct EvalEnv {
    var_frames: Vec<HashMap<String, Value>>,
    pub funs: HashMap<String, Function>,
    pub call_depth: usize,
}

impl EvalEnv {
    pub fn new() -> Self {
        EvalEnv {
            var_frames: vec![HashMap::new()],
            funs: HashMap::new(),
            call_depth: 0,
        }
    }

    pub fn push_frame(&mut self, bindings: HashMap<String, Value>) {
        self.var_frames.push(bindings);
    }

    pub fn pop_frame(&mut self) {
        self.var_frames.pop();
    }

    pub fn declare_var(&mut self, name: &str, val: Value) {
        self.var_frames.last_mut().unwrap().insert(name.to_string(), val);
    }

    pub fn assign_var(&mut self, name: &str, val: Value, loc: &Loc) -> Result<(), BestError> {
        for frame in self.var_frames.iter_mut().rev() {
            if frame.contains_key(name) {
                frame.insert(name.to_string(), val);
                return Ok(());
            }
        }
        Err(BestError::at(format!("undefined variable '{name}'"), loc))
    }

    pub fn lookup_var(&self, name: &str, loc: &Loc) -> Result<Value, BestError> {
        for frame in self.var_frames.iter().rev() {
            if let Some(v) = frame.get(name) {
                return Ok(v.clone());
            }
        }
        Err(BestError::at(format!("undefined variable '{name}'"), loc))
    }
}
