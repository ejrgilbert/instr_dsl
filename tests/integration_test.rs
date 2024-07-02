mod common;

use log::error;
use orca::ir::Module as WasmModule;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use wabt::{wasm2wat, wat2wasm};
use walrus::Module;
use whamm::behavior::builder_visitor::{build_behavior_tree, SimpleAST};
use whamm::common::error::ErrorGen;
use whamm::generator::emitters::{Emitter, WasmRewritingEmitter};
use whamm::generator::init_generator::InitGenerator;
use whamm::generator::instr_generator::InstrGenerator;

const APP_WASM_PATH: &str = "tests/apps/dfinity/users.wasm";

const OUT_BASE_DIR: &str = "target";
const OUT_WASM_NAME: &str = "out.wasm";

/// This test just confirms that a wasm module can be instrumented with the preconfigured
/// scripts without errors occurring.
#[test]
fn instrument_dfinity_with_fault_injection() {
    common::setup_logger();
    let mut err = ErrorGen::new("".to_string(), "".to_string(), 0);
    let processed_scripts = common::setup_fault_injection("dfinity", &mut err);
    assert!(!processed_scripts.is_empty());
    err.fatal_report("Integration Test");

    for (script_path, script_text, whamm, symbol_table) in processed_scripts {
        // Build the behavior tree from the AST
        let mut simple_ast = SimpleAST::new();
        let mut behavior = build_behavior_tree(&whamm, &mut simple_ast, &mut err);
        behavior.reset();

        let buff = std::fs::read(APP_WASM_PATH).unwrap();
        let app_wasm =
            WasmModule::parse_only_module(&buff, false).expect("Failed to parse Wasm module");

        let mut err = ErrorGen::new(script_path.clone(), script_text, 0);
        let mut emitter = WasmRewritingEmitter::new(app_wasm, symbol_table);
        // Phase 0 of instrumentation (emit globals and provided fns)
        let mut init = InitGenerator {
            emitter: Box::new(&mut emitter),
            context_name: "".to_string(),
            err: &mut err,
        };
        assert!(init.run(&whamm));
        err.fatal_report("Integration Test");

        // Phase 1 of instrumentation (actually emits the instrumentation code)
        // This structure is necessary since we need to have the fns/globals injected (a single time)
        // and ready to use in every body/predicate.
        let mut instr = InstrGenerator {
            tree: &behavior,
            emitter: Box::new(&mut emitter),
            ast: simple_ast,
            context_name: "".to_string(),
            curr_provider_name: "".to_string(),
            curr_package_name: "".to_string(),
            curr_event_name: "".to_string(),
            curr_probe_mode: "".to_string(),
            curr_probe: None,
            err: &mut err,
        };
        // TODO add assertions here once I have error logic in place to check that it worked!
        instr.run(&behavior);
        err.fatal_report("Integration Test");

        if !Path::new(OUT_BASE_DIR).exists() {
            if let Err(err) = fs::create_dir(OUT_BASE_DIR) {
                error!("{}", err.to_string());
                panic!("Could not create base output path.");
            }
        }

        let out_wasm_path = format!("{OUT_BASE_DIR}/{OUT_WASM_NAME}");
        if let Err(e) = emitter.dump_to_file(out_wasm_path.clone()) {
            err.add_error(*e)
        }
        err.fatal_report("Integration Test");

        let mut wasm2wat = Command::new("wasm2wat");
        wasm2wat.stdout(Stdio::null()).arg(out_wasm_path);

        // wasm2wat verification check
        match wasm2wat.status() {
            Ok(code) => {
                if !code.success() {
                    panic!("`wasm2wat` verification check failed!");
                }
            }
            Err(err) => {
                error!("{}", err.to_string());
                panic!("`wasm2wat` verification check failed!");
            }
        };
    }
}

#[test]
fn instrument_handwritten_wasm_call() {
    common::setup_logger();
    // executable is located at target/debug/whamm
    let executable = "target/debug/whamm";

    // if you want to change the wat file
    // (calling wat2wasm from a child process doesn't work
    //  since somehow the executable can't write to the file system directly)
    let file_data = fs::read("tests/apps/handwritten/add.wat").unwrap();
    let wasm_data = wat2wasm(file_data).unwrap();
    fs::write("tests/apps/handwritten/add.wasm", wasm_data).unwrap();

    let res = Command::new(executable)
        .arg("instr")
        .arg("--script")
        .arg("tests/scripts/instr.mm")
        .arg("--app")
        .arg("tests/apps/handwritten/add.wasm")
        .output()
        .expect("failed to execute process");
    assert!(res.status.success());

    let file_data = fs::read("output/output.wasm").unwrap();
    let wat_data = wasm2wat(file_data).unwrap();
    println!("{}", wat_data);
}

#[test]
fn instrument_control_flow() {
    common::setup_logger();
    let executable = "target/debug/whamm";

    // run cargo run on control flow
    let a = Command::new("cargo")
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .current_dir("wasm_playground/control_flow")
        .output()
        .expect("failed to execute process");
    assert!(a.status.success());

    let res = Command::new(executable)
        .arg("instr")
        .arg("--script")
        .arg("tests/scripts/instr.mm")
        .arg("--app")
        .arg("wasm_playground/control_flow/target/wasm32-unknown-unknown/debug/cf.wasm")
        .output()
        .expect("failed to execute process");
    assert!(res.status.success());

    let file_data = fs::read("output/output.wasm").unwrap();
    let wat_data = wasm2wat(file_data).unwrap();
    fs::write("output/output.wat", wat_data).unwrap();
}

#[test]
fn instrument_spin_with_fault_injection() {
    common::setup_logger();
    let mut err = ErrorGen::new("".to_string(), "".to_string(), 0);
    let processed_scripts = common::setup_fault_injection("spin", &mut err);
    // TODO -- change this when you've supported this monitor type
    assert_eq!(processed_scripts.len(), 0);
}

#[test]
fn instrument_with_wizard_monitors() {
    common::setup_logger();
    let mut err = ErrorGen::new("".to_string(), "".to_string(), 0);
    let processed_scripts = common::setup_wizard_monitors(&mut err);
    // TODO -- change this when you've supported this monitor type
    assert_eq!(processed_scripts.len(), 0);
}

#[test]
fn instrument_with_replay() {
    common::setup_logger();
    let mut err = ErrorGen::new("".to_string(), "".to_string(), 0);
    let processed_scripts = common::setup_replay(&mut err);
    // TODO -- change this when you've supported this monitor type
    assert_eq!(processed_scripts.len(), 0);
}
