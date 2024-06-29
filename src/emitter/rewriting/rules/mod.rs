use std::collections::HashMap;
use walrus::ir::Instr;
use walrus::ValType;
use crate::parser::rules::{Probe, WhammProviderKind};
use crate::parser::types::Script;
use crate::verifier::types::Record;

pub mod wasm;
mod core;

/// A function that can be used to generate these emitter rule types
/// from the parser AST.
pub fn from(scripts: &Vec<Script>) -> Vec<WhammProvider> {
    todo!()
}

pub struct LocInfo<'a> {
    /// static information to be saved in symbol table
    static_data: HashMap<String, Record>,
    /// dynamic information corresponding to the operands of this location
    dynamic_data: Vec<ValType>,
    /// the probes that were matched for this instruction
    probes: Vec<&'a dyn Probe>
}

pub trait ProcessLoc {
    /// Pass some location to the provider and get back two types of data:
    fn get_loc_info(&self, app_wasm: &walrus::Module, instr: &Instr, instr_name: &str)
        -> LocInfo;
}

pub trait Provider {}
pub trait Package {}
pub trait Event {}

pub struct WhammProvider {
    kind: WhammProviderKind,
    /// The packages of the probes that have been used in the Script.
    pub packages: Vec<Box<dyn Package>>,
}
impl Provider for WhammProvider {}
impl ProcessLoc for WhammProvider {
    fn get_loc_info(&self, _app_wasm: &walrus::Module, _instr: &Instr, _instr_name: &str)
        -> LocInfo {
        match self.kind {
            WhammProviderKind::Core => {
                todo!()
            },
            WhammProviderKind::Wasm => {
                todo!()
            }
        }
    }
}
