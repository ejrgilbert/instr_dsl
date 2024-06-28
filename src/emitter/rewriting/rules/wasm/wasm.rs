use std::collections::HashMap;
use walrus::ir::Instr;
use walrus::{Module, ValType};
use crate::emitter::rewriting::providers::Provider;
use crate::verifier::types::Record;

pub struct Wasm;
impl Provider for Wasm {
    fn get_loc_info(app_wasm: &Module, instr: &Instr, instr_name: &String) -> (HashMap<String, Record>, Vec<ValType>) {
        todo!()
    }
}

fn get_instr_info(app_wasm: &walrus::Module, instr: &Instr, instr_name: &String) -> crate::emitter::rewriting::emitter::InstrInfo {
    let instr_args = vec![];
    match instr {
        Instr::Call(func) => {
            let func = app_wasm.funcs.get(func.func);
            // get information about the function call
            let (func_info, params) = crate::emitter::rewriting::emitter::get_func_info(app_wasm, func);

            crate::emitter::rewriting::InstrInfo {
                instr_name: instr_name.to_owned(),
                instr_args: params,
                called_func_info: Some(func_info),
            }
        }
        Instr::CallIndirect(_) => todo!(),
        Instr::LocalGet(_) => todo!(),
        Instr::LocalSet(_) => todo!(),
        Instr::LocalTee(_) => todo!(),
        Instr::GlobalGet(_) => todo!(),
        Instr::GlobalSet(_) => todo!(),
        Instr::Drop(_) => todo!(),
        Instr::Const(_) => todo!(),
        Instr::Binop(_) => todo!(),
        Instr::Unop(_) => todo!(),
        Instr::Select(_) => todo!(),
        Instr::Br(_) => {
            // label_id
            todo!()
        },
        Instr::BrIf(_) => {
            // label_id
            // condition
            todo!()
        },
        Instr::IfElse(_) => todo!(),
        Instr::BrTable(_) => todo!(),
        Instr::Return(_) => todo!(),
        Instr::MemorySize(_) => todo!(),
        Instr::MemoryGrow(_) => todo!(),
        Instr::MemoryInit(_) => todo!(),
        Instr::DataDrop(_) => todo!(),
        Instr::MemoryCopy(_) => todo!(),
        Instr::MemoryFill(_) => todo!(),
        Instr::Load(_) => todo!(),
        Instr::Store(_) => todo!(),
        Instr::TableGet(_) => todo!(),
        Instr::TableSet(_) => todo!(),
        Instr::TableGrow(_) => todo!(),
        Instr::TableSize(_) => todo!(),
        Instr::TableFill(_) => todo!(),
        Instr::TableInit(_) => todo!(),
        Instr::TableCopy(_) => todo!(),
        Instr::ElemDrop(_) => todo!(),
        Instr::RefNull(_) => todo!(),
        Instr::RefIsNull(_) => todo!(),
        Instr::RefFunc(_) => todo!(),
        Instr::V128Bitselect(_) => todo!(),
        Instr::I8x16Swizzle(_) => todo!(),
        Instr::I8x16Shuffle(_) => todo!(),
        Instr::LoadSimd(_) => todo!(),
        Instr::AtomicRmw(_) => todo!(),
        Instr::Cmpxchg(_) => todo!(),
        Instr::AtomicNotify(_) => todo!(),
        Instr::AtomicWait(_) => todo!(),
        Instr::AtomicFence(_) => todo!(),
        Instr::Block(_) |
        Instr::Loop(_) |
        Instr::Unreachable(_) => {
            // no arguments to these instructions
            crate::emitter::rewriting::emitter::InstrInfo {
                instr_name: instr_name.to_owned(),
                instr_args,
                called_func_info: None,
            }
        }
    }
}