use crate::emitter::rewriting::rules::{Event, LocInfo, Package, ProcessLoc};
use crate::parser::rules::core::{CoreEventKind, CorePackageKind};
use walrus::ir::Instr;

pub struct CorePackage {
    kind: CorePackageKind,
    pub events: Vec<Box<dyn Event>>,
}
impl Package for CorePackage {}
impl ProcessLoc for CorePackage {
    fn get_loc_info(
        &self,
        _app_wasm: &walrus::Module,
        _instr: &Instr,
        _instr_name: &str,
    ) -> LocInfo {
        match self.kind {
            CorePackageKind::Default => {
                todo!()
            }
        }
    }
}

pub struct CoreEvent {
    kind: CoreEventKind,
    pub events: Vec<Box<dyn Event>>,
}
impl Event for CoreEvent {}
impl ProcessLoc for CoreEvent {
    fn get_loc_info(
        &self,
        _app_wasm: &walrus::Module,
        _instr: &Instr,
        _instr_name: &str,
    ) -> LocInfo {
        match self.kind {
            CoreEventKind::Default => {
                todo!()
            }
        }
    }
}
