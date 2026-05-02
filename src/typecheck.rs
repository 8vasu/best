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
use crate::env::TypeEnv;
use crate::error::{BestError, Loc};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Void,
}

impl Type {
    pub fn from_name(s: &str, loc: &Loc) -> Result<Type, BestError> {
        match s {
            "int"   => Ok(Type::Int),
            "float" => Ok(Type::Float),
            "bool"  => Ok(Type::Bool),
            "str"   => Ok(Type::Str),
            "void"  => Ok(Type::Void),
            other   => Err(BestError::at(format!("unknown type '{other}'"), loc)),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Type::Int   => "int",
            Type::Float => "float",
            Type::Bool  => "bool",
            Type::Str   => "str",
            Type::Void  => "void",
        }
    }
}

/* -- Helpers to destructure a List's head ---------------------------------- */

fn head_id(expr: &Expr) -> Option<&str> {
    if let Expr::List(items, _) = expr {
        if let Some(Expr::Id(s, _)) = items.first() {
            return Some(s.as_str());
        }
    }
    None
}

fn items(expr: &Expr) -> Option<&[Expr]> {
    if let Expr::List(v, _) = expr { Some(v) } else { None }
}

/* -- Pre-pass: collect all declare signatures ----------------------------- */

fn collect_signatures(program: &[Expr]) -> Result<HashMap<String, (Vec<Type>, Type)>, BestError> {
    let mut sigs = HashMap::new();
    for expr in program {
        if head_id(expr) != Some("declare") { continue; }
        let parts = items(expr).unwrap();
        // (declare name (param-types...) ret-type)
        if parts.len() != 4 {
            return Err(BestError::at(
                "declare expects: (declare name (param-types...) return-type)",
                expr.loc(),
            ));
        }
        let name = match &parts[1] {
            Expr::Id(s, _) => s.clone(),
            other => return Err(BestError::at("declare: name must be an identifier", other.loc())),
        };
        let param_types = parse_type_list(&parts[2])?;
        let ret_type = parse_single_type(&parts[3])?;
        sigs.insert(name, (param_types, ret_type));
    }
    Ok(sigs)
}

fn parse_type_list(expr: &Expr) -> Result<Vec<Type>, BestError> {
    match expr {
        Expr::List(items, _) => {
            items.iter().map(parse_single_type).collect()
        }
        other => Err(BestError::at("expected a list of types", other.loc())),
    }
}

fn parse_single_type(expr: &Expr) -> Result<Type, BestError> {
    match expr {
        Expr::TypeName(s, loc) => Type::from_name(s, loc),
        other => Err(BestError::at("expected a type name", other.loc())),
    }
}

/* -- Main type-check pass ------------------------------------------------- */

pub fn typecheck(program: &[Expr]) -> Result<(), BestError> {
    let sigs = collect_signatures(program)?;
    let mut env = TypeEnv::new(sigs);
    let mut pending: Option<(String, Vec<Type>, Type, Loc)> = None;

    for expr in program {
        match head_id(expr) {
            Some("declare") => {
                if let Some((name, _, _, loc)) = &pending {
                    return Err(BestError::at(
                        format!("declare for '{name}' must be immediately followed by defun"),
                        loc,
                    ));
                }
                let parts = items(expr).unwrap();
                let name = match &parts[1] {
                    Expr::Id(s, _) => s.clone(),
                    other => return Err(BestError::at("declare: name must be an identifier", other.loc())),
                };
                let param_types = parse_type_list(&parts[2])?;
                let ret_type = parse_single_type(&parts[3])?;
                pending = Some((name, param_types, ret_type, expr.loc().clone()));
            }

            Some("defun") => {
                let parts = items(expr).unwrap();
                // (defun name (params...) body)
                if parts.len() != 4 {
                    return Err(BestError::at(
                        "defun expects: (defun name (params...) body)",
                        expr.loc(),
                    ));
                }
                let def_name = match &parts[1] {
                    Expr::Id(s, _) => s.clone(),
                    other => return Err(BestError::at("defun: name must be an identifier", other.loc())),
                };
                let (decl_name, param_types, ret_type, _) = pending.take().ok_or_else(|| {
                    BestError::at(
                        format!("defun '{def_name}' has no preceding declare"),
                        expr.loc(),
                    )
                })?;
                if decl_name != def_name {
                    return Err(BestError::at(
                        format!("declare/defun name mismatch: '{decl_name}' vs '{def_name}'"),
                        expr.loc(),
                    ));
                }
                let param_names = parse_param_names(&parts[2])?;
                if param_names.len() != param_types.len() {
                    return Err(BestError::at(
                        format!(
                            "defun '{def_name}': declared {} params but defined {}",
                            param_types.len(), param_names.len()
                        ),
                        expr.loc(),
                    ));
                }
                env.push_frame();
                for (n, t) in param_names.iter().zip(param_types.iter()) {
                    env.declare_var(n, t.clone());
                }
                let body_ty = check_expr(&parts[3], &mut env, Some(&ret_type))?;
                env.pop_frame();
                if body_ty != ret_type {
                    return Err(BestError::at(
                        format!(
                            "function '{def_name}' body has type '{}' but declared return type '{}'",
                            body_ty.name(), ret_type.name()
                        ),
                        parts[3].loc(),
                    ));
                }
            }

            _ => {
                if let Some((name, _, _, loc)) = &pending {
                    return Err(BestError::at(
                        format!("declare for '{name}' must be immediately followed by defun"),
                        loc,
                    ));
                }
                check_expr(expr, &mut env, None)?;
            }
        }
    }

    if let Some((name, _, _, loc)) = pending {
        return Err(BestError::at(
            format!("declare for '{name}' has no corresponding defun"),
            &loc,
        ));
    }

    Ok(())
}

fn parse_param_names(expr: &Expr) -> Result<Vec<String>, BestError> {
    match expr {
        Expr::List(items, _) => {
            items.iter().map(|e| match e {
                Expr::Id(s, _) => Ok(s.clone()),
                other => Err(BestError::at("parameter must be an identifier", other.loc())),
            }).collect()
        }
        other => Err(BestError::at("expected parameter list", other.loc())),
    }
}

/* Returns the type of expr and enforces all semantic rules. */
pub fn check_expr(expr: &Expr, env: &mut TypeEnv, ret: Option<&Type>) -> Result<Type, BestError> {
    match expr {
        Expr::Int(_, _)       => Ok(Type::Int),
        Expr::Float(_, _)     => Ok(Type::Float),
        Expr::Bool(_, _)      => Ok(Type::Bool),
        Expr::Str(_, _)       => Ok(Type::Str),
        Expr::TypeName(_, loc) => Err(BestError::at("type name used in expression position", loc)),
        Expr::Id(name, loc)   => env.lookup_var(name, loc),

        Expr::List(parts, loc) => {
            if parts.is_empty() {
                return Err(BestError::at("empty list is not a valid expression", loc));
            }
            let head = &parts[0];
            match head {
                Expr::Id(op, _) => check_list(op, parts, loc, env, ret),
                other => Err(BestError::at("list head must be an identifier", other.loc())),
            }
        }
    }
}

fn check_list(
    op: &str,
    parts: &[Expr],
    loc: &Loc,
    env: &mut TypeEnv,
    ret: Option<&Type>,
) -> Result<Type, BestError> {
    match op {
        /* -- let ----------------------------------------------------------- */
        "let" => {
            // (let name type value)
            if parts.len() != 4 {
                return Err(BestError::at("let expects: (let name type value)", loc));
            }
            let name = require_id(&parts[1], "let: variable name")?;
            let ty = require_type(&parts[2], "let: type")?;
            let val_ty = check_expr(&parts[3], env, ret)?;
            if val_ty != ty {
                return Err(BestError::at(
                    format!("let '{name}': value type '{}' does not match declared type '{}'",
                            val_ty.name(), ty.name()),
                    parts[3].loc(),
                ));
            }
            env.declare_var(&name, ty);
            Ok(Type::Void)
        }

        /* -- assign --------------------------------------------------------- */
        "assign" => {
            // (assign name value)
            if parts.len() != 3 {
                return Err(BestError::at("assign expects: (assign name value)", loc));
            }
            let name = require_id(&parts[1], "assign: variable name")?;
            let var_ty = env.lookup_var(&name, parts[1].loc())?;
            let val_ty = check_expr(&parts[2], env, ret)?;
            if val_ty != var_ty {
                return Err(BestError::at(
                    format!("assign '{name}': value type '{}' does not match variable type '{}'",
                            val_ty.name(), var_ty.name()),
                    parts[2].loc(),
                ));
            }
            Ok(Type::Void)
        }

        /* -- if ------------------------------------------------------------- */
        "if" => {
            // (if cond then else)
            if parts.len() != 4 {
                return Err(BestError::at("if expects: (if cond then else)", loc));
            }
            let cond_ty = check_expr(&parts[1], env, ret)?;
            if cond_ty != Type::Bool {
                return Err(BestError::at(
                    format!("if condition must be bool, got '{}'", cond_ty.name()),
                    parts[1].loc(),
                ));
            }
            let then_ty = check_expr(&parts[2], env, ret)?;
            let else_ty = check_expr(&parts[3], env, ret)?;
            if then_ty != else_ty {
                return Err(BestError::at(
                    format!("if branches have different types: '{}' vs '{}'",
                            then_ty.name(), else_ty.name()),
                    loc,
                ));
            }
            Ok(then_ty)
        }

        /* -- while ---------------------------------------------------------- */
        "while" => {
            // (while cond body...)
            if parts.len() < 2 {
                return Err(BestError::at("while expects: (while cond body...)", loc));
            }
            let cond_ty = check_expr(&parts[1], env, ret)?;
            if cond_ty != Type::Bool {
                return Err(BestError::at(
                    format!("while condition must be bool, got '{}'", cond_ty.name()),
                    parts[1].loc(),
                ));
            }
            for body_expr in &parts[2..] {
                check_expr(body_expr, env, ret)?;
            }
            Ok(Type::Void)
        }

        /* -- print ---------------------------------------------------------- */
        "print" => {
            if parts.len() != 2 {
                return Err(BestError::at("print expects exactly one argument", loc));
            }
            let ty = check_expr(&parts[1], env, ret)?;
            if ty == Type::Void {
                return Err(BestError::at("cannot print a void value", parts[1].loc()));
            }
            Ok(Type::Void)
        }

        /* -- defun / declare inside expression ----------------------------- */
        "defun" | "declare" => {
            Err(BestError::at(
                format!("'{op}' is only allowed at top level"),
                loc,
            ))
        }

        /* -- arithmetic ----------------------------------------------------- */
        "+" | "-" | "*" | "/" => {
            if parts.len() != 3 {
                return Err(BestError::at(
                    format!("'{op}' expects exactly two arguments"), loc));
            }
            let lt = check_expr(&parts[1], env, ret)?;
            let rt = check_expr(&parts[2], env, ret)?;
            if lt != rt {
                return Err(BestError::at(
                    format!("'{op}' operands have different types: '{}' vs '{}'",
                            lt.name(), rt.name()),
                    loc,
                ));
            }
            if lt != Type::Int && lt != Type::Float {
                return Err(BestError::at(
                    format!("'{op}' requires numeric operands, got '{}'", lt.name()),
                    loc,
                ));
            }
            Ok(lt)
        }

        /* -- comparison ----------------------------------------------------- */
        "==" | "!=" => {
            if parts.len() != 3 {
                return Err(BestError::at(
                    format!("'{op}' expects exactly two arguments"), loc));
            }
            let lt = check_expr(&parts[1], env, ret)?;
            let rt = check_expr(&parts[2], env, ret)?;
            if lt != rt {
                return Err(BestError::at(
                    format!("'{op}' operands have different types: '{}' vs '{}'",
                            lt.name(), rt.name()),
                    loc,
                ));
            }
            Ok(Type::Bool)
        }

        "<" | ">" | "<=" | ">=" => {
            if parts.len() != 3 {
                return Err(BestError::at(
                    format!("'{op}' expects exactly two arguments"), loc));
            }
            let lt = check_expr(&parts[1], env, ret)?;
            let rt = check_expr(&parts[2], env, ret)?;
            if lt != rt {
                return Err(BestError::at(
                    format!("'{op}' operands have different types: '{}' vs '{}'",
                            lt.name(), rt.name()),
                    loc,
                ));
            }
            if lt != Type::Int && lt != Type::Float {
                return Err(BestError::at(
                    format!("'{op}' requires numeric operands, got '{}'", lt.name()),
                    loc,
                ));
            }
            Ok(Type::Bool)
        }

        /* -- boolean ops ---------------------------------------------------- */
        "and" | "or" => {
            if parts.len() != 3 {
                return Err(BestError::at(
                    format!("'{op}' expects exactly two arguments"), loc));
            }
            let lt = check_expr(&parts[1], env, ret)?;
            let rt = check_expr(&parts[2], env, ret)?;
            if lt != Type::Bool {
                return Err(BestError::at(
                    format!("'{op}' left operand must be bool, got '{}'", lt.name()), loc));
            }
            if rt != Type::Bool {
                return Err(BestError::at(
                    format!("'{op}' right operand must be bool, got '{}'", rt.name()), loc));
            }
            Ok(Type::Bool)
        }

        "not" => {
            if parts.len() != 2 {
                return Err(BestError::at("not expects exactly one argument", loc));
            }
            let t = check_expr(&parts[1], env, ret)?;
            if t != Type::Bool {
                return Err(BestError::at(
                    format!("not requires bool, got '{}'", t.name()), loc));
            }
            Ok(Type::Bool)
        }

        /* -- string ops ----------------------------------------------------- */
        "concat" => {
            if parts.len() != 3 {
                return Err(BestError::at("concat expects exactly two arguments", loc));
            }
            let lt = check_expr(&parts[1], env, ret)?;
            let rt = check_expr(&parts[2], env, ret)?;
            if lt != Type::Str {
                return Err(BestError::at(
                    format!("concat left operand must be str, got '{}'", lt.name()), loc));
            }
            if rt != Type::Str {
                return Err(BestError::at(
                    format!("concat right operand must be str, got '{}'", rt.name()), loc));
            }
            Ok(Type::Str)
        }

        "length" => {
            if parts.len() != 2 {
                return Err(BestError::at("length expects exactly one argument", loc));
            }
            let t = check_expr(&parts[1], env, ret)?;
            if t != Type::Str {
                return Err(BestError::at(
                    format!("length requires str, got '{}'", t.name()), loc));
            }
            Ok(Type::Int)
        }

        /* -- function call -------------------------------------------------- */
        name => {
            let (param_types, ret_type) = env.lookup_fun(name, loc)?;
            let args = &parts[1..];
            if args.len() != param_types.len() {
                return Err(BestError::at(
                    format!("function '{name}' expects {} argument(s), got {}",
                            param_types.len(), args.len()),
                    loc,
                ));
            }
            for (i, (arg, expected)) in args.iter().zip(param_types.iter()).enumerate() {
                let got = check_expr(arg, env, ret)?;
                if &got != expected {
                    return Err(BestError::at(
                        format!("function '{name}' argument {}: expected '{}', got '{}'",
                                i + 1, expected.name(), got.name()),
                        arg.loc(),
                    ));
                }
            }
            Ok(ret_type)
        }
    }
}

fn require_id(expr: &Expr, context: &str) -> Result<String, BestError> {
    match expr {
        Expr::Id(s, _) => Ok(s.clone()),
        other => Err(BestError::at(format!("{context} must be an identifier"), other.loc())),
    }
}

fn require_type(expr: &Expr, context: &str) -> Result<Type, BestError> {
    match expr {
        Expr::TypeName(s, loc) => Type::from_name(s, loc),
        other => Err(BestError::at(format!("{context} must be a type name"), other.loc())),
    }
}
