use crate::emitter::rewriting::rules::{Event, LocInfo, Package, ProcessLoc};
use crate::parser::rules::wasm::{BytecodeEventKind, WasmPackageKind};
use crate::parser::rules::Probe;
use std::collections::HashMap;
use walrus::ir::Instr;

pub struct WasmPackage {
    kind: WasmPackageKind,
    pub events: Vec<Box<dyn Event>>,
}
impl Package for WasmPackage {}
impl ProcessLoc for WasmPackage {
    fn get_loc_info(
        &self,
        _app_wasm: &walrus::Module,
        _instr: &Instr,
        _instr_name: &str,
    ) -> LocInfo {
        match self.kind {
            WasmPackageKind::Bytecode => {
                todo!()
            }
        }
    }
}

pub struct BytecodeEvent<'a> {
    kind: BytecodeEventKind,
    // outer Vec represents script[0:9] for composable instrumentation,
    // inner HashMap is mode_name-> an ordered Vec of probes from the respective script
    probes: Vec<HashMap<String, Vec<&'a dyn Probe>>>,
}
impl Event for BytecodeEvent<'_> {}
impl ProcessLoc for BytecodeEvent<'_> {
    fn get_loc_info(
        &self,
        _app_wasm: &walrus::Module,
        _instr: &Instr,
        _instr_name: &str,
    ) -> LocInfo {
        match self.kind {
            BytecodeEventKind::Block => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Loop => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Call => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes

                // let func = app_wasm.funcs.get(func.func);
                // // get information about the function call
                // let (func_info, params) = crate::emitter::rewriting::get_func_info(app_wasm, func);
                //
                // crate::emitter::rewriting::InstrInfo {
                //     instr_name: instr_name.to_owned(),
                //     instr_args: params,
                //     called_func_info: Some(func_info),
                // }

                todo!()
            }
            BytecodeEventKind::CallIndirect => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::LocalGet => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::LocalSet => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::LocalTee => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::GlobalGet => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::GlobalSet => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Const => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Binop => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Unop => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Select => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Unreachable => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Br => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // - label_id
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::BrIf => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // - label_id
                // - condition
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::IfElse => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::BrTable => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Drop => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Return => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::MemorySize => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::MemoryGrow => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::MemoryInit => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::DataDrop => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::MemoryCopy => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::MemoryFill => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Load => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Store => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::AtomicRmw => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::Cmpxchg => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::AtomicNotify => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::AtomicWait => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::AtomicFence => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::TableGet => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::TableSet => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::TableGrow => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::TableSize => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::TableFill => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::RefNull => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::RefIsNull => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::RefFunc => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::V128Bitselect => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::I8x16Swizzle => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::I8x16Shuffle => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::LoadSimd => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::TableInit => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::ElemDrop => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
            BytecodeEventKind::TableCopy => {
                // check if the instr is of this event type
                // define static/dynamic vars
                // pull matched probes
                todo!()
            }
        }
    }
}
