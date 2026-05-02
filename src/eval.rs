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
use std::fmt;

use crate::ast::Expr;
use crate::env::{EvalEnv, Function};
use crate::error::{BestError, Loc};

const MAX_DEPTH: usize = 8_000;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Void,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n)   => write!(f, "{n}"),
            Value::Float(n) => write!(f, "{n}"),
            Value::Bool(b)  => write!(f, "{}", if *b { "true" } else { "false" }),
            Value::Str(s)   => write!(f, "{s}"),
            Value::Void     => Ok(()),
        }
    }
}

pub fn eval_program(program: &[Expr]) -> Result<(), BestError> {
    let mut env = EvalEnv::new();

    // First pass: register all functions so forward calls and mutual
    // recursion work when we reach the top-level statements.
    for expr in program {
        if let Expr::List(parts, _) = expr {
            if matches!(parts.first(), Some(Expr::Id(s, _)) if s == "defun") {
                register_fun(parts, expr.loc(), &mut env)?;
            }
        }
    }

    // Second pass: evaluate top-level statements in order.
    for expr in program {
        // Skip declare/defun -- already handled.
        match expr {
            Expr::List(parts, _) => {
                match parts.first() {
                    Some(Expr::Id(s, _)) if s == "declare" || s == "defun" => continue,
                    _ => {}
                }
            }
            _ => {}
        }
        eval_expr(expr, &mut env)?;
    }

    Ok(())
}

fn register_fun(parts: &[Expr], loc: &Loc, env: &mut EvalEnv) -> Result<(), BestError> {
    // (defun name (params...) body)
    let name = match &parts[1] {
        Expr::Id(s, _) => s.clone(),
        other => return Err(BestError::at("defun: name must be an identifier", other.loc())),
    };
    let params = match &parts[2] {
        Expr::List(items, _) => {
            items.iter().map(|e| match e {
                Expr::Id(s, _) => Ok(s.clone()),
                other => Err(BestError::at("defun: param must be an identifier", other.loc())),
            }).collect::<Result<Vec<_>, _>>()?
        }
        other => return Err(BestError::at("defun: expected parameter list", other.loc())),
    };
    let body = parts[3].clone();
    env.funs.insert(name, Function { params, body });
    let _ = loc;
    Ok(())
}

pub fn eval_expr(expr: &Expr, env: &mut EvalEnv) -> Result<Value, BestError> {
    match expr {
        Expr::Int(n, _)       => Ok(Value::Int(*n)),
        Expr::Float(f, _)     => Ok(Value::Float(*f)),
        Expr::Bool(b, _)      => Ok(Value::Bool(*b)),
        Expr::Str(s, _)       => Ok(Value::Str(s.clone())),
        Expr::TypeName(_, loc) => Err(BestError::at("type name in expression", loc)),
        Expr::Id(name, loc)   => env.lookup_var(name, loc),

        Expr::List(parts, loc) => {
            if parts.is_empty() {
                return Err(BestError::at("empty list is not a valid expression", loc));
            }
            let op = match &parts[0] {
                Expr::Id(s, _) => s.as_str(),
                other => return Err(BestError::at("list head must be an identifier", other.loc())),
            };
            eval_list(op, parts, loc, env)
        }
    }
}

fn eval_list(op: &str, parts: &[Expr], loc: &Loc, env: &mut EvalEnv) -> Result<Value, BestError> {
    match op {
        "declare" => Ok(Value::Void),

        "let" => {
            let name = id_of(&parts[1]);
            let val = eval_expr(&parts[3], env)?;
            env.declare_var(name, val);
            Ok(Value::Void)
        }

        "assign" => {
            let name = id_of(&parts[1]);
            let val = eval_expr(&parts[2], env)?;
            env.assign_var(name, val, parts[1].loc())?;
            Ok(Value::Void)
        }

        "if" => {
            let cond = eval_expr(&parts[1], env)?;
            match cond {
                Value::Bool(true)  => eval_expr(&parts[2], env),
                Value::Bool(false) => eval_expr(&parts[3], env),
                _ => Err(BestError::at("if condition must be bool", parts[1].loc())),
            }
        }

        "while" => {
            loop {
                let cond = eval_expr(&parts[1], env)?;
                match cond {
                    Value::Bool(false) => break,
                    Value::Bool(true)  => {
                        for body in &parts[2..] {
                            eval_expr(body, env)?;
                        }
                    }
                    _ => return Err(BestError::at("while condition must be bool", parts[1].loc())),
                }
            }
            Ok(Value::Void)
        }

        "print" => {
            let val = eval_expr(&parts[1], env)?;
            println!("{val}");
            Ok(Value::Void)
        }

        "+" => bin_arith(parts, env, |a, b| a + b, |a, b| a + b, "+"),
        "-" => bin_arith(parts, env, |a, b| a - b, |a, b| a - b, "-"),
        "*" => bin_arith(parts, env, |a, b| a * b, |a, b| a * b, "*"),

        "/" => {
            let lv = eval_expr(&parts[1], env)?;
            let rv = eval_expr(&parts[2], env)?;
            match (lv, rv) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 { return Err(BestError::at("division by zero", loc)); }
                    Ok(Value::Int(a / b))
                }
                (Value::Float(a), Value::Float(b)) => {
                    if b == 0.0 { return Err(BestError::at("division by zero", loc)); }
                    Ok(Value::Float(a / b))
                }
                _ => Err(BestError::at("'/' requires numeric operands of the same type", loc)),
            }
        }

        "==" => cmp_op(parts, env, |a, b| a == b, |a, b| a == b, |a, b| a == b, "=="),
        "!=" => cmp_op(parts, env, |a, b| a != b, |a, b| a != b, |a, b| a != b, "!="),
        "<"  => ord_op(parts, env, |a, b| a < b,  |a, b| a < b,  "<"),
        ">"  => ord_op(parts, env, |a, b| a > b,  |a, b| a > b,  ">"),
        "<=" => ord_op(parts, env, |a, b| a <= b, |a, b| a <= b, "<="),
        ">=" => ord_op(parts, env, |a, b| a >= b, |a, b| a >= b, ">="),

        "and" => {
            let a = eval_bool(&parts[1], env, "and")?;
            let b = eval_bool(&parts[2], env, "and")?;
            Ok(Value::Bool(a && b))
        }
        "or" => {
            let a = eval_bool(&parts[1], env, "or")?;
            let b = eval_bool(&parts[2], env, "or")?;
            Ok(Value::Bool(a || b))
        }
        "not" => {
            let a = eval_bool(&parts[1], env, "not")?;
            Ok(Value::Bool(!a))
        }

        "concat" => {
            let a = eval_str(&parts[1], env, "concat")?;
            let b = eval_str(&parts[2], env, "concat")?;
            Ok(Value::Str(a + &b))
        }
        "length" => {
            let s = eval_str(&parts[1], env, "length")?;
            Ok(Value::Int(s.chars().count() as i64))
        }

        /* function call */
        name => {
            if env.call_depth >= MAX_DEPTH {
                return Err(BestError::at(
                    format!("stack overflow calling '{name}'"), loc));
            }
            let fun = env.funs.get(name).cloned().ok_or_else(|| {
                BestError::at(format!("undefined function '{name}'"), loc)
            })?;
            let mut frame: HashMap<String, Value> = HashMap::new();
            for (param, arg_expr) in fun.params.iter().zip(parts[1..].iter()) {
                let val = eval_expr(arg_expr, env)?;
                frame.insert(param.clone(), val);
            }
            env.call_depth += 1;
            env.push_frame(frame);
            let result = eval_expr(&fun.body, env);
            env.pop_frame();
            env.call_depth -= 1;
            result
        }
    }
}

fn id_of(expr: &Expr) -> &str {
    match expr {
        Expr::Id(s, _) => s.as_str(),
        _ => panic!("eval: expected Id (should have been caught by type checker)"),
    }
}

fn bin_arith(
    parts: &[Expr],
    env: &mut EvalEnv,
    fi: impl Fn(i64, i64) -> i64,
    ff: impl Fn(f64, f64) -> f64,
    op: &str,
) -> Result<Value, BestError> {
    let lv = eval_expr(&parts[1], env)?;
    let rv = eval_expr(&parts[2], env)?;
    match (lv, rv) {
        (Value::Int(a), Value::Int(b))     => Ok(Value::Int(fi(a, b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(ff(a, b))),
        _ => Err(BestError::new(format!("'{op}' requires numeric operands"))),
    }
}

fn cmp_op(
    parts: &[Expr],
    env: &mut EvalEnv,
    ii: impl Fn(i64, i64) -> bool,
    ff: impl Fn(f64, f64) -> bool,
    bb: impl Fn(bool, bool) -> bool,
    op: &str,
) -> Result<Value, BestError> {
    let lv = eval_expr(&parts[1], env)?;
    let rv = eval_expr(&parts[2], env)?;
    let result = match (lv, rv) {
        (Value::Int(a), Value::Int(b))     => ii(a, b),
        (Value::Float(a), Value::Float(b)) => ff(a, b),
        (Value::Bool(a), Value::Bool(b))   => bb(a, b),
        (Value::Str(a), Value::Str(b))     => a == b,
        _ => return Err(BestError::new(format!("'{op}' type mismatch"))),
    };
    Ok(Value::Bool(result))
}

fn ord_op(
    parts: &[Expr],
    env: &mut EvalEnv,
    ii: impl Fn(i64, i64) -> bool,
    ff: impl Fn(f64, f64) -> bool,
    op: &str,
) -> Result<Value, BestError> {
    let lv = eval_expr(&parts[1], env)?;
    let rv = eval_expr(&parts[2], env)?;
    let result = match (lv, rv) {
        (Value::Int(a), Value::Int(b))     => ii(a, b),
        (Value::Float(a), Value::Float(b)) => ff(a, b),
        _ => return Err(BestError::new(format!("'{op}' requires numeric operands"))),
    };
    Ok(Value::Bool(result))
}

fn eval_bool(expr: &Expr, env: &mut EvalEnv, op: &str) -> Result<bool, BestError> {
    match eval_expr(expr, env)? {
        Value::Bool(b) => Ok(b),
        _ => Err(BestError::at(format!("'{op}' requires bool operands"), expr.loc())),
    }
}

fn eval_str(expr: &Expr, env: &mut EvalEnv, op: &str) -> Result<String, BestError> {
    match eval_expr(expr, env)? {
        Value::Str(s) => Ok(s),
        _ => Err(BestError::at(format!("'{op}' requires str operands"), expr.loc())),
    }
}
