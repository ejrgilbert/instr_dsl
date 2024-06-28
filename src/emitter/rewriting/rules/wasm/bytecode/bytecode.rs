use std::collections::HashMap;
use walrus::ir::Instr;
use walrus::{Module, ValType};
use crate::emitter::rewriting::providers::Package;
use crate::verifier::types::Record;

pub struct Bytecode;
impl Package for Bytecode {
    fn get_loc_info(app_wasm: &Module, instr: &Instr, instr_name: &String) -> (HashMap<String, Record>, Vec<ValType>) {
        todo!()
    }
}