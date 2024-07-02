extern crate core;

use cli::{Cmd, WhammCli};

use crate::behavior::builder_visitor::*;
use crate::common::error::ErrorGen;
use crate::generator::emitters::{Emitter, WasmRewritingEmitter};
use crate::generator::init_generator::InitGenerator;
use crate::generator::instr_generator::InstrGenerator;
use crate::parser::whamm_parser::*;

pub mod behavior;
mod cli;
pub mod common;
pub mod generator;
pub mod parser;
pub mod verifier;

use clap::Parser;
use graphviz_rust::cmd::{CommandArg, Format};
use graphviz_rust::exec_dot;
use log::{error, info};
use orca::ir::Module as WasmModule;
use project_root::get_project_root;
use std::path::PathBuf;
use std::process::exit;
use walrus::Module;

use crate::behavior::tree::BehaviorTree;
use crate::behavior::visualize::visualization_to_file;
use crate::parser::types::Whamm;
use crate::verifier::types::SymbolTable;
use crate::verifier::verifier::{build_symbol_table, type_check};

const MAX_ERRORS: i32 = 15;

fn setup_logger() {
    env_logger::init();
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("error: {}", e);
        for c in e.iter_chain().skip(1) {
            eprintln!("  caused by {}", c);
        }
        eprintln!("{}", e.backtrace());
        exit(1)
    }
}

fn try_main() -> Result<(), failure::Error> {
    setup_logger();

    // Get information from user command line args
    let cli = WhammCli::parse();

    match cli.command {
        Cmd::Info {
            spec,
            globals,
            functions,
        } => {
            run_info(spec, globals, functions);
        }
        Cmd::Instr(args) => {
            run_instr(
                args.app,
                args.script,
                args.output_path,
                args.virgil,
                args.run_verifier,
            );
        }
        Cmd::VisWasm { wasm, output_path } => {
            run_vis_wasm(wasm, output_path);
        }
        Cmd::VisScript {
            script,
            run_verifier,
            output_path,
        } => {
            run_vis_script(script, run_verifier, output_path);
        }
    }

    Ok(())
}

fn run_info(spec: String, print_globals: bool, print_functions: bool) {
    // Parse the script and generate the information
    let mut err = ErrorGen::new("".to_string(), spec.clone(), MAX_ERRORS);
    print_info(spec, print_globals, print_functions, &mut err);

    err.fatal_report("PrintInfo");
}

fn run_instr(
    app_wasm_path: String,
    script_path: String,
    output_wasm_path: String,
    emit_virgil: bool,
    run_verifier: bool,
) {
    // Set up error reporting mechanism
    let mut err = ErrorGen::new(script_path.clone(), "".to_string(), MAX_ERRORS);

    // Process the script
    let mut whamm = get_script_ast(&script_path, &mut err);
    let symbol_table = get_symbol_table(&mut whamm, run_verifier, &mut err);
    let (behavior_tree, simple_ast) = build_behavior(&whamm, &mut err);

    // If there were any errors encountered, report and exit!
    err.check_has_errors();

    // Read app Wasm into Orca module
    let buff = std::fs::read(app_wasm_path).unwrap();
    let app_wasm = WasmModule::parse_only_module(&buff, false).unwrap();

    // Configure the emitter based on target instrumentation code format
    let mut emitter = if emit_virgil {
        unimplemented!();
    } else {
        WasmRewritingEmitter::new(app_wasm, symbol_table)
    };

    // Phase 0 of instrumentation (emit globals and provided fns)
    let mut init = InitGenerator {
        emitter: Box::new(&mut emitter),
        context_name: "".to_string(),
        err: &mut err,
    };
    init.run(&whamm);
    // If there were any errors encountered, report and exit!
    err.check_has_errors();

    // Phase 1 of instrumentation (actually emits the instrumentation code)
    // This structure is necessary since we need to have the fns/globals injected (a single time)
    // and ready to use in every body/predicate.
    let mut instr = InstrGenerator {
        tree: &behavior_tree,
        emitter: Box::new(&mut emitter),
        ast: simple_ast,
        err: &mut err,
        context_name: "".to_string(),
        curr_provider_name: "".to_string(),
        curr_package_name: "".to_string(),
        curr_event_name: "".to_string(),
        curr_probe_mode: "".to_string(),
        curr_probe: None,
    };
    instr.run(&behavior_tree);
    // If there were any errors encountered, report and exit!
    err.check_has_errors();

    // create output path if it doesn't exist
    if !PathBuf::from(&output_wasm_path).exists() {
        std::fs::create_dir_all(PathBuf::from(&output_wasm_path).parent().unwrap()).unwrap();
    }

    // emitter.app_wasm

    if let Err(e) = emitter.dump_to_file(output_wasm_path) {
        err.add_error(*e)
    }
    // If there were any errors encountered, report and exit!
    err.check_has_errors();
}

fn run_vis_wasm(wasm_path: String, output_path: String) {
    // Read app Wasm into Walrus module
    let _config = walrus::ModuleConfig::new();
    let app_wasm = Module::from_file(wasm_path).unwrap();

    if !PathBuf::from(&output_path).exists() {
        std::fs::create_dir_all(PathBuf::from(&output_path).parent().unwrap()).unwrap();
    }

    if app_wasm.write_graphviz_dot(output_path.clone()).is_ok() {
        match std::fs::read_to_string(output_path.clone()) {
            Ok(dot_str) => {
                let svg_path = format!("{}.svg", output_path.clone());

                if let Err(e) = exec_dot(
                    dot_str,
                    vec![Format::Svg.into(), CommandArg::Output(svg_path.clone())],
                ) {
                    println!("{}", e);
                    exit(1);
                }

                if let Err(err) = opener::open(svg_path.clone()) {
                    error!("Could not open visualization of wasm at: {}", svg_path);
                    error!("{:?}", err)
                }
            }
            Err(error) => {
                error!("Cannot read specified file {}: {}", output_path, error);
                exit(1);
            }
        };
    }
}

fn run_vis_script(script_path: String, run_verifier: bool, output_path: String) {
    // Set up error reporting mechanism
    let mut err = ErrorGen::new(script_path.clone(), "".to_string(), MAX_ERRORS);

    let mut whamm = get_script_ast(&script_path, &mut err);
    // building the symbol table is necessary since it does some minor manipulations of the AST
    // (adds declared globals to the script AST node)
    let _symbol_table = get_symbol_table(&mut whamm, run_verifier, &mut err);
    let (behavior_tree, ..) = build_behavior(&whamm, &mut err);

    // if there are any errors, should report and exit!
    err.check_has_errors();

    if !PathBuf::from(&output_path).exists() {
        std::fs::create_dir_all(PathBuf::from(&output_path).parent().unwrap()).unwrap();
    }

    let path = match get_pb(&PathBuf::from(output_path.clone())) {
        Ok(pb) => pb,
        Err(_) => exit(1),
    };

    if visualization_to_file(&behavior_tree, path).is_ok() {
        if let Err(err) = opener::open(output_path.clone()) {
            error!("Could not open visualization tree at: {}", output_path);
            error!("{:?}", err)
        }
    }
    exit(0);
}

fn get_symbol_table(ast: &mut Whamm, run_verifier: bool, err: &mut ErrorGen) -> SymbolTable {
    let mut st = build_symbol_table(ast, err);
    err.check_too_many();
    verify_ast(ast, &mut st, run_verifier, err);
    st
}

fn verify_ast(ast: &Whamm, st: &mut SymbolTable, run_verifier: bool, err: &mut ErrorGen) {
    if run_verifier && !type_check(ast, st, err) {
        error!("AST failed verification!");
    }
    err.check_too_many();
}

fn get_script_ast(script_path: &String, err: &mut ErrorGen) -> Whamm {
    match std::fs::read_to_string(script_path) {
        Ok(unparsed_str) => {
            // Parse the script and build the AST
            match parse_script(&unparsed_str, err) {
                Some(ast) => {
                    info!("successfully parsed");
                    err.check_too_many();
                    ast
                }
                None => {
                    err.report();
                    exit(1);
                }
            }
        }
        Err(error) => {
            error!("Cannot read specified file {}: {}", script_path, error);
            exit(1);
        }
    }
}

fn build_behavior<'a>(whamm: &'a Whamm, err: &mut ErrorGen) -> (BehaviorTree, SimpleAST<'a>) {
    // Build the behavior tree from the AST
    let mut simple_ast = SimpleAST::new();
    let mut behavior = build_behavior_tree(whamm, &mut simple_ast, err);
    err.check_too_many();
    behavior.reset();

    (behavior, simple_ast)
}

pub(crate) fn get_pb(file_pb: &PathBuf) -> Result<PathBuf, String> {
    if file_pb.is_relative() {
        match get_project_root() {
            Ok(r) => {
                let mut full_path = r.clone();
                full_path.push(file_pb);
                Ok(full_path)
            }
            Err(e) => Err(format!("the root folder does not exist: {:?}", e)),
        }
    } else {
        Ok(file_pb.clone())
    }
}
