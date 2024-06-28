use std::collections::HashMap;
use walrus::ir::Instr;
use walrus::ValType;
use crate::verifier::types::Record;

pub mod wasm;

pub trait Provider {
    /// Pass some location to the provider and get back two types of data:
    /// 1. HashMap<String, Record> -> static information to be saved in symbol table
    /// 2. Vec<ValType> -> dynamic information corresponding to the operands of this location
    fn get_loc_info(app_wasm: &walrus::Module, instr: &Instr, instr_name: &String) -> (HashMap<String, Record>, Vec<ValType>);
}

pub trait Package {
    /// Pass some location to the package and get back two types of data:
    /// 1. HashMap<String, Record> -> static information to be saved in symbol table
    /// 2. Vec<ValType> -> dynamic information corresponding to the operands of this location
    fn get_loc_info(app_wasm: &walrus::Module, instr: &Instr, instr_name: &String) -> (HashMap<String, Record>, Vec<ValType>);
}

pub trait Event {
    /// Pass some location to the event and get back two types of data:
    /// 1. HashMap<String, Record> -> static information to be saved in symbol table
    /// 2. Vec<ValType> -> dynamic information corresponding to the operands of this location
    fn get_loc_info(app_wasm: &walrus::Module, instr: &Instr, instr_name: &String) -> (HashMap<String, Record>, Vec<ValType>);
}

pub trait Mode {
    /// Pass some location to the mode and get back two types of data:
    /// 1. HashMap<String, Record> -> static information to be saved in symbol table
    /// 2. Vec<ValType> -> dynamic information corresponding to the operands of this location
    fn get_loc_info(app_wasm: &walrus::Module, instr: &Instr, instr_name: &String) -> (HashMap<String, Record>, Vec<ValType>);
}