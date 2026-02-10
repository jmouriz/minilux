// The Minilux Programming Language
// Version: 0.1.0
// Author: Alexia Michelle <https://minilux.org>
// License: MPL 2.0
// SPDX-License-Identifier: MPL-2.0

mod interpreter;
mod lexer;
mod parser;
mod runtime;
mod value;

use interpreter::Interpreter;
use parser::Parser;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut modules_spec: Option<String> = None;
    let mut script: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-m" | "--modules" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: -m/--modules requires a path");
                    std::process::exit(1);
                }
                modules_spec = Some(args[i].clone());
            }
            "-h" | "--help" => {
                print_usage_and_exit(&args[0]);
            }
            s if s.starts_with('-') => {
                eprintln!("Error: unknown option: {}", s);
                print_usage_and_exit(&args[0]);
            }
            _ => {
                // first positional arg is script path
                if script.is_none() {
                    script = Some(args[i].clone());
                } else {
                    eprintln!("Error: unexpected extra argument: {}", args[i]);
                    print_usage_and_exit(&args[0]);
                }
            }
        }
        i += 1;
    }

    if let Some(path) = script {
        if let Err(e) = execute_file(&path, modules_spec.as_deref()) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    } else {
        run_repl(modules_spec.as_deref());
    }
}

fn execute_file(path: &str, modules_spec: Option<&str>) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut parser = Parser::new(&content);
    let statements = parser.parse();

    let mut interpreter = Interpreter::new();
    if let Some(spec) = modules_spec {
        interpreter.set_modules_path(spec);
    }
    let absolute_path = {
        let provided = Path::new(path);
        if provided.is_absolute() {
            provided.to_path_buf()
        } else {
            env::current_dir()
                .map_err(|e| format!("Failed to determine current directory: {}", e))?
                .join(provided)
        }
    };

    let base_dir = absolute_path.parent().map(|p| p.to_path_buf());
    if let Some(dir) = base_dir.clone() {
        interpreter.push_base_dir(dir);
    }

    let result = interpreter.execute(statements);

    if base_dir.is_some() {
        interpreter.pop_base_dir();
    }

    result
}

fn run_repl(modules_spec: Option<&str>) {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut input = String::new();

    println!("Minilux Interpreter Console (REPL)");
    println!("Version 0.1.0 on {} -- [Rust]", get_system_info());
    println!("Type \"exit\" to quit");
    println!();

    loop {
        input.clear();
        print!("> ");
        std::io::stdout().flush().ok();

        if reader.read_line(&mut input).is_err() {
            break;
        }

        let trimmed = input.trim();
        if trimmed == "exit" {
            break;
        }

        if trimmed.is_empty() {
            continue;
        }

        let mut parser = Parser::new(trimmed);
        let statements = parser.parse();

        let mut interpreter = Interpreter::new();
        if let Some(spec) = modules_spec {
            interpreter.set_modules_path(spec);
        }
    if let Some(spec) = modules_spec {
        interpreter.set_modules_path(spec);
    }
        if let Err(e) = interpreter.execute(statements) {
            eprintln!("Error: {}", e);
        }
    }
}

fn print_usage_and_exit(prog: &str) -> ! {
    eprintln!("Usage: {} [-m <paths>] [script.mi]", prog);
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -m, --modules <paths>   Module search path list (':' or ';' separated)");
    eprintln!("  -h, --help              Show this help");
    std::process::exit(1);
}

fn get_system_info() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    format!("{}/{}", os, arch)
}
