// The Minilux Programming Language
// Version: 0.1.0
// Author: Alexia Michelle <https://minilux.org>
// License: MPL 2.0
// SPDX-License-Identifier: MPL-2.0

use crate::parser::{BinOp, Expr, Statement, UnaryOp};
use crate::runtime::Runtime;
use crate::value::Value;
use regex::Regex;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashSet;

pub struct Interpreter {
    runtime: Runtime,
    current_return: Option<Value>,
    base_dirs: Vec<PathBuf>,
    modules_paths: Vec<PathBuf>,
    include_in_progress: HashSet<PathBuf>,
}

impl Interpreter {
    pub fn new() -> Self {
        let modules_paths = env::var("MINILUX_MODULES_PATH")
            .ok()
            .map(|v| Self::parse_modules_path_list(&v))
            .unwrap_or_default();

        Interpreter {
            runtime: Runtime::new(),
            current_return: None,
            base_dirs: vec![env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
            modules_paths,
            include_in_progress: HashSet::new(),
        }
    }

    pub fn push_base_dir(&mut self, dir: PathBuf) {
        if let Ok(canonical) = dir.canonicalize() {
            self.base_dirs.push(canonical);
        } else {
            self.base_dirs.push(dir);
        }
    }

    pub fn pop_base_dir(&mut self) {
        if self.base_dirs.len() > 1 {
            self.base_dirs.pop();
        }
    }

    fn current_base_dir(&self) -> Option<&PathBuf> {
        self.base_dirs.last()
    }

        fn parse_modules_path_list(spec: &str) -> Vec<PathBuf> {
        // Supports multiple paths separated by ':' (Unix) or ';' (Windows).
        let normalized = spec.replace(';', ":");
        normalized
            .split(':')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .map(|p| p.canonicalize().unwrap_or(p))
            .collect()
    }

    pub fn set_modules_path(&mut self, spec: &str) {
        self.modules_paths = Self::parse_modules_path_list(spec);
    }

fn resolve_include_path(&self, path: &str) -> PathBuf {
        let specified = Path::new(path);
        if specified.is_absolute() {
            return specified.to_path_buf();
        }

        if let Some(base) = self.current_base_dir() {
            let candidate = base.join(specified);
            if candidate.exists() {
                return candidate;
            }
        }

        for base in &self.modules_paths {
            let candidate = base.join(specified);
            if candidate.exists() {
                return candidate;
            }
        }

        match env::current_dir() {
            Ok(cwd) => {
                let candidate = cwd.join(specified);
                if candidate.exists() {
                    return candidate;
                }
                candidate
            }
            Err(_) => specified.to_path_buf(),
        }
    }

    pub fn execute(&mut self, statements: Vec<Statement>) -> Result<(), String> {
        for stmt in statements {
            self.execute_statement(&stmt)?;
        }
        Ok(())
    }

    fn execute_statement(&mut self, stmt: &Statement) -> Result<Option<Value>, String> {
        match stmt {
            Statement::Assignment { var, value } => {
                let val = self.eval_expr(value)?;
                self.runtime.set_var(var.clone(), val);
                Ok(None)
            }
            Statement::ArrayAssignment { var, index, value } => {
                let idx = self.eval_expr(index)?.to_int() as usize;
                let val = self.eval_expr(value)?;

                let mut array = self.runtime.get_var(var);
                if let Value::Array(ref mut arr) = array {
                    if idx < arr.len() {
                        arr[idx] = val;
                    }
                }
                self.runtime.set_var(var.clone(), array);
                Ok(None)
            }
            Statement::If {
                condition,
                then_body,
                elseif_parts,
                else_body,
            } => {
                let cond = self.eval_expr(condition)?;
                if cond.is_truthy() {
                    for s in then_body {
                        if let Ok(Some(v)) = self.execute_statement(s) {
                            return Ok(Some(v));
                        }
                    }
                } else {
                    let mut executed = false;
                    for (elif_cond, elif_body) in elseif_parts {
                        let elif_cond_val = self.eval_expr(elif_cond)?;
                        if elif_cond_val.is_truthy() {
                            for s in elif_body {
                                if let Ok(Some(v)) = self.execute_statement(s) {
                                    return Ok(Some(v));
                                }
                            }
                            executed = true;
                            break;
                        }
                    }

                    if !executed {
                        if let Some(else_stmts) = else_body {
                            for s in else_stmts {
                                if let Ok(Some(v)) = self.execute_statement(s) {
                                    return Ok(Some(v));
                                }
                            }
                        }
                    }
                }
                Ok(None)
            }
            Statement::While { condition, body } => {
                while self.eval_expr(condition)?.is_truthy() {
                    for s in body {
                        if let Ok(Some(v)) = self.execute_statement(s) {
                            return Ok(Some(v));
                        }
                    }
                }
                Ok(None)
            }
            Statement::Printf { format, args } => {
                let mut output = String::new();

                if !format.is_empty() {
                    output.push_str(&format);
                }

                for arg in args {
                    let val = self.eval_expr(arg)?;
                    match val {
                        Value::Int(n) => output.push_str(&n.to_string()),
                        Value::String(s) => output.push_str(&s),
                        Value::Array(arr) => output.push_str(&format!("[Array({})]", arr.len())),
                        Value::Regex(p) => output.push_str(&format!("/{}/", p)),
                        Value::Nil => (),
                    }
                }

                output = output.replace("\\n", "\n").replace("\\t", "\t");

                print!("{}", output);

                if !output.ends_with('\n') {
                    println!();
                }

                Ok(None)
            }
            Statement::Read { var } => {
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| format!("Failed to read input: {}", e))?;

                let trimmed = input
                    .trim_end_matches(|c| c == '\n' || c == '\r')
                    .to_string();
                self.runtime.set_var(var.clone(), Value::String(trimmed));
                Ok(None)
            }
            Statement::Inc { var, value } => {
                let current = self.runtime.get_var(var);
                let inc_val = self.eval_expr(value)?;
                let result = current.add(&inc_val);
                self.runtime.set_var(var.clone(), result);
                Ok(None)
            }
            Statement::Dec { var, value } => {
                let current = self.runtime.get_var(var);
                let dec_val = self.eval_expr(value)?;
                let result = current.subtract(&dec_val);
                self.runtime.set_var(var.clone(), result);
                Ok(None)
            }
            Statement::Push { array, value } => {
                let mut arr = self.runtime.get_var(array);
                let val = self.eval_expr(value)?;

                match arr {
                    Value::Array(ref mut elements) => {
                        elements.push(val);
                    }
                    _ => {
                        arr = Value::Array(vec![val]);
                    }
                }

                self.runtime.set_var(array.clone(), arr);
                Ok(None)
            }
            Statement::Pop { array } => {
                let mut arr = self.runtime.get_var(array);
                if let Value::Array(ref mut elements) = arr {
                    elements.pop();
                }
                self.runtime.set_var(array.clone(), arr);
                Ok(None)
            }
            Statement::Shift { array } => {
                let mut arr = self.runtime.get_var(array);
                if let Value::Array(ref mut elements) = arr {
                    if !elements.is_empty() {
                        elements.remove(0);
                    }
                }
                self.runtime.set_var(array.clone(), arr);
                Ok(None)
            }
            Statement::Unshift { array, value } => {
                let mut arr = self.runtime.get_var(array);
                let val = self.eval_expr(value)?;

                match arr {
                    Value::Array(ref mut elements) => {
                        elements.insert(0, val);
                    }
                    _ => {
                        arr = Value::Array(vec![val]);
                    }
                }

                self.runtime.set_var(array.clone(), arr);
                Ok(None)
            }
            Statement::Sockopen { name, host, port } => {
                let host_val = self.eval_expr(host)?.to_string();
                let port_val = self.eval_expr(port)?.to_int() as u16;
                let addr = format!("{}:{}", host_val, port_val);

                match TcpStream::connect(&addr) {
                    Ok(stream) => {
                        self.runtime.set_socket(name.clone(), stream);
                        Ok(None)
                    }
                    Err(_) => Err(format!("Failed to connect to {}", addr)),
                }
            }
            Statement::Sockclose { name } => {
                self.runtime.remove_socket(name);
                Ok(None)
            }
            Statement::Sockwrite { name, data } => {
                let data_val = self.eval_expr(data)?;
                let data_str = data_val.to_string();

                if let Some(stream) = self.runtime.get_socket(name) {
                    stream.write_all(data_str.as_bytes()).ok();
                    stream.flush().ok();
                }

                Ok(None)
            }
            Statement::Sockread { name, var } => {
                if let Some(stream) = self.runtime.get_socket(name) {
                    let mut buffer = [0; 1024];
                    match stream.read(&mut buffer) {
                        Ok(n) => {
                            let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                            self.runtime.set_var(var.clone(), Value::String(data));
                        }
                        Err(_) => {
                            self.runtime
                                .set_var(var.clone(), Value::String(String::new()));
                        }
                    }
                }

                Ok(None)
            }
                        Statement::Include { path } => {
                let resolved_path = self.resolve_include_path(path);
                let canonical = fs::canonicalize(&resolved_path).unwrap_or_else(|_| resolved_path.clone());

                if self.include_in_progress.contains(&canonical) {
                    return Err(format!(
                        "Include cycle detected (already in progress): {}",
                        canonical.display()
                    ));
                }

                self.include_in_progress.insert(canonical.clone());

                let result = match fs::read_to_string(&resolved_path) {
                    Ok(content) => {
                        let mut parser = crate::parser::Parser::new(&content);
                        let stmts = parser.parse();

                        let parent_dir = resolved_path.parent().map(|p| p.to_path_buf());
                        if let Some(dir) = parent_dir.clone() {
                            self.push_base_dir(dir);
                        }

                        let exec_result = self.execute(stmts);

                        if parent_dir.is_some() {
                            self.pop_base_dir();
                        }

                        exec_result?;
                        Ok(None)
                    }
                    Err(e) => Err(format!("Failed to include file: {}", e)),
                };

                self.include_in_progress.remove(&canonical);
                result
            }
Statement::FunctionDef { name, params, body } => {
                self.runtime
                    .define_function(name.clone(), params.clone(), body.clone());
                Ok(None)
            }
            Statement::FunctionCall { name, args } => {
                // Built-ins bypass user-defined lookup, so handle them early.
                if name == "sleep" {
                    if let Some(arg) = args.first() {
                        let val = self.eval_expr(arg)?;
                        let seconds = val.to_int() as u64;
                        std::thread::sleep(std::time::Duration::from_secs(seconds));
                    }
                    return Ok(None);
                }

                if let Some((params, body)) = self.runtime.get_function(name) {
                    // Evaluate arguments
                    let mut arg_vals: Vec<Value> = Vec::new();
                    for arg in args {
                        arg_vals.push(self.eval_expr(arg)?);
                    }

                    // Bind params (save old values for restoration)
                    let mut saved: Vec<(String, Option<Value>)> = Vec::new();
                    for (i, p) in params.iter().enumerate() {
                        let old = match self.runtime.get_var(p) {
                            Value::Nil => None,
                            v => Some(v),
                        };
                        saved.push((p.clone(), old));
                        let v = arg_vals.get(i).cloned().unwrap_or(Value::Nil);
                        self.runtime.set_var(p.clone(), v);
                    }

                    // Execute body
                    let mut ret: Option<Value> = None;
                    for stmt in &body {
                        if let Ok(Some(val)) = self.execute_statement(stmt) {
                            ret = Some(val);
                            break;
                        }
                    }

                    // Restore params
                    for (p, old) in saved.into_iter() {
                        if let Some(v) = old {
                            self.runtime.set_var(p, v);
                        } else {
                            self.runtime.remove_var(&p);
                        }
                    }

                    Ok(ret)
                } else {
                    eprintln!("Warning: function '{}' not defined", name);
                    Ok(None)
                }
            }
            Statement::Return { value } => {
                if let Some(expr) = value {
                    let val = self.eval_expr(expr)?;
                    self.current_return = Some(val.clone());
                    Ok(Some(val))
                } else {
                    self.current_return = Some(Value::Nil);
                    Ok(Some(Value::Nil))
                }
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Regex(pat) => Ok(Value::Regex(pat.clone())),

Expr::SubstCall { pat, repl, flags, input } => {
    let src_val = self.eval_expr(input)?;
    let src = src_val.to_string();

    let mut mode = String::new();
    if flags.contains('i') { mode.push('i'); }
    if flags.contains('m') { mode.push('m'); }
    if flags.contains('s') { mode.push('s'); }

    let pat2 = if mode.is_empty() {
        pat.clone()
    } else {
        format!("(?{}){}", mode, pat)
    };

    let re = Regex::new(&pat2)
        .map_err(|e| format!("Invalid regex /{}/: {}", pat, e))?;

    let out = if flags.contains('g') {
        re.replace_all(&src, repl.as_str()).to_string()
    } else {
        re.replace(&src, repl.as_str()).to_string()
    };

    Ok(Value::String(out))
}
            Expr::Variable(name) => Ok(self.runtime.get_var(name)),
            Expr::Binary { left, op, right } => {
                let left_val = self.eval_expr(left)?;
                let right_val = self.eval_expr(right)?;

                Ok(match op {
                    BinOp::Add => left_val.add(&right_val),
                    BinOp::Subtract => left_val.subtract(&right_val),
                    BinOp::Multiply => left_val.multiply(&right_val),
                    BinOp::Divide => left_val.divide(&right_val),
                    BinOp::Modulo => left_val.modulo(&right_val),
                    BinOp::Equal => Value::Int(if left_val.equals(&right_val) { 1 } else { 0 }),
                    BinOp::NotEqual => Value::Int(if !left_val.equals(&right_val) { 1 } else { 0 }),
                    BinOp::Less => {
                        if let Some(std::cmp::Ordering::Less) = left_val.compare(&right_val) {
                            Value::Int(1)
                        } else {
                            Value::Int(0)
                        }
                    }
                    BinOp::LessEqual => match left_val.compare(&right_val) {
                        Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal) => {
                            Value::Int(1)
                        }
                        _ => Value::Int(0),
                    },
                    BinOp::Greater => {
                        if let Some(std::cmp::Ordering::Greater) = left_val.compare(&right_val) {
                            Value::Int(1)
                        } else {
                            Value::Int(0)
                        }
                    }
                    BinOp::GreaterEqual => match left_val.compare(&right_val) {
                        Some(std::cmp::Ordering::Greater) | Some(std::cmp::Ordering::Equal) => {
                            Value::Int(1)
                        }
                        _ => Value::Int(0),
                    },
                    BinOp::And => Value::Int(if left_val.is_truthy() && right_val.is_truthy() {
                        1
                    } else {
                        0
                    }),
                    BinOp::Or => Value::Int(if left_val.is_truthy() || right_val.is_truthy() {
                        1
                    } else {
                        0
                    }),
BinOp::Match => {
    let text = left_val.to_string();
    let pat = match right_val {
        Value::Regex(p) => p,
        Value::String(s) => s,
        other => other.to_string(),
    };
    let re = Regex::new(&pat)
        .map_err(|e| format!("Invalid regex /{}/: {}", pat, e))?;
    Value::Int(if re.is_match(&text) { 1 } else { 0 })
}
                })
            }
            Expr::Unary { op, expr } => {
                let val = self.eval_expr(expr)?;
                Ok(match op {
                    UnaryOp::Not => Value::Int(if val.is_truthy() { 0 } else { 1 }),
                    UnaryOp::Negate => Value::Int(-val.to_int()),
                })
            }
            Expr::Array(elements) => {
                let mut values = Vec::new();
                for elem in elements {
                    values.push(self.eval_expr(elem)?);
                }
                Ok(Value::Array(values))
            }
            Expr::Index { expr, index } => {
                let arr = self.eval_expr(expr)?;
                let idx = self.eval_expr(index)?.to_int() as usize;

                match arr {
                    Value::Array(elements) => Ok(elements.get(idx).cloned().unwrap_or(Value::Nil)),
                    Value::String(s) => {
                        let chars: Vec<char> = s.chars().collect();
                        if idx < chars.len() {
                            Ok(Value::String(chars[idx].to_string()))
                        } else {
                            Ok(Value::Nil)
                        }
                    }
                    _ => Ok(Value::Nil),
                }
            }
            Expr::FunctionCall { name, args } => {
                match name.as_str() {
                    "len" | "strlen" => {
                        if let Some(arg) = args.first() {
                            let val = self.eval_expr(arg)?;
                            match val {
                                Value::String(s) => Ok(Value::Int(s.len() as i64)),
                                Value::Array(arr) => Ok(Value::Int(arr.len() as i64)),
                                _ => Ok(Value::Int(0)),
                            }
                        } else {
                            Ok(Value::Int(0))
                        }
                    }
                    "shell" => {
                        if let Some(arg) = args.first() {
                            let val = self.eval_expr(arg)?;
                            let cmd_str = val.to_string();

                            let output = if cfg!(target_os = "windows") {
                                Command::new("cmd").args(["/C", &cmd_str]).output()
                            } else {
                                Command::new("sh").arg("-c").arg(&cmd_str).output()
                            };

                            match output {
                                Ok(result) => {
                                    let mut stdout =
                                        String::from_utf8_lossy(&result.stdout).to_string();
                                    // Trim trailing newline so pipelines behave predictably.
                                    if stdout.ends_with('\n') {
                                        stdout.pop();
                                        if stdout.ends_with('\r') {
                                            stdout.pop();
                                        }
                                    }
                                    Ok(Value::String(stdout))
                                }
                                Err(_) => Ok(Value::String(String::new())),
                            }
                        } else {
                            Ok(Value::String(String::new()))
                        }
                    }
                    "number" => {
                        if let Some(arg) = args.first() {
                            let val = self.eval_expr(arg)?;
                            match val {
                                Value::Int(n) => Ok(Value::Int(n)),
                                Value::String(s) => {
                                    let trimmed = s.trim();
                                    match trimmed.parse::<i64>() {
                                        Ok(n) => Ok(Value::Int(n)),
                                        Err(_) => Ok(Value::Int(0)),
                                    }
                                }
                                Value::Array(_) | Value::Nil | Value::Regex(_) => Ok(Value::Int(0)),
                            }
                        } else {
                            Ok(Value::Int(0))
                        }
                    }
                    "lower" => {
                        if let Some(arg) = args.first() {
                            let val = self.eval_expr(arg)?;
                            Ok(Value::String(val.to_string().to_lowercase()))
                        } else {
                            Ok(Value::String(String::new()))
                        }
                    }
                    "upper" => {
                        if let Some(arg) = args.first() {
                            let val = self.eval_expr(arg)?;
                            Ok(Value::String(val.to_string().to_uppercase()))
                        } else {
                            Ok(Value::String(String::new()))
                        }
                    }
                    "sleep" => {
                        if let Some(arg) = args.first() {
                            let val = self.eval_expr(arg)?;
                            let seconds = val.to_int() as u64;
                            std::thread::sleep(std::time::Duration::from_secs(seconds));
                            Ok(Value::Nil)
                        } else {
                            Ok(Value::Nil)
                        }
                    }
                    _ => {
                        if let Some((params, body)) = self.runtime.get_function(name) {
                            // Evaluate args
                            let mut arg_vals: Vec<Value> = Vec::new();
                            for arg in args {
                                arg_vals.push(self.eval_expr(arg)?);
                            }

                            // Bind params
                            let mut saved: Vec<(String, Option<Value>)> = Vec::new();
                            for (i, p) in params.iter().enumerate() {
                                let old = match self.runtime.get_var(p) {
                                    Value::Nil => None,
                                    v => Some(v),
                                };
                                saved.push((p.clone(), old));
                                let v = arg_vals.get(i).cloned().unwrap_or(Value::Nil);
                                self.runtime.set_var(p.clone(), v);
                            }

                            // Execute
                            let mut ret: Value = Value::Nil;
                            for stmt in &body {
                                if let Ok(Some(val)) = self.execute_statement(stmt) {
                                    ret = val;
                                    break;
                                }
                            }

                            // Restore
                            for (p, old) in saved.into_iter() {
                                if let Some(v) = old {
                                    self.runtime.set_var(p, v);
                                } else {
                                    self.runtime.remove_var(&p);
                                }
                            }

                            Ok(ret)
                        } else {
                            eprintln!("Warning: unknown function '{}'", name);
                            Ok(Value::Nil)
                        }
                    }}
            }
        }
    }
}
