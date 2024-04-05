use log::error;
use walrus::FunctionId;
use walrus::ir::{BinaryOp, ExtendedLoad, LoadKind, MemArg};
use crate::parser::types::{DataType, Dscript, Dtrace, Expr, Fn, Function, Module, Op, Probe, Provider, Statement, Value};
use crate::verifier::types::{Record, SymbolTable};

// =================================================
// ==== Emitter Trait --> Used By All Emitters! ====
// =================================================

pub trait Emitter {
    fn enter_scope(&mut self);
    fn exit_scope(&mut self);

    fn emit_dtrace(&mut self, dtrace: &Dtrace) -> bool;
    fn emit_dscript(&mut self, dscript: &Dscript) -> bool;
    fn emit_provider(&mut self, provider: &Provider) -> bool;

    // TODO -- should emit module/function/probe be private?
    fn emit_module(&mut self, module: &Module) -> bool;
    fn emit_function(&mut self, function: &Function) -> bool;
    fn emit_probe(&mut self, probe: &Probe) -> bool;

    fn emit_fn(&mut self, context_name: &String, f: &Fn) -> bool;
    fn emit_formal_param(&mut self, param: &(Expr, DataType)) -> bool;
    fn emit_global(&mut self, name: String, ty: DataType, val: &Option<Value>) -> bool;
    fn emit_stmt(&mut self, stmt: &Statement) -> bool;
    fn emit_expr(&mut self, expr: &Expr) -> bool;
    fn emit_op(&mut self, op: &Op) -> bool;
    fn emit_datatype(&mut self, datatype: &DataType) -> bool;
    fn emit_value(&mut self, val: &Value) -> bool;

    fn dump_to_file(&mut self, output_wasm_path: String) -> bool;
}

// ==============================
// ==== WasmRewritingEmitter ====
// ==============================

pub(crate) struct WasmRewritingEmitter {
    pub(crate) app_wasm: walrus::Module,
    pub(crate) table: SymbolTable,

    fn_providing_contexts: Vec<String>
}
impl WasmRewritingEmitter {
    pub fn new(app_wasm: walrus::Module, table: SymbolTable) -> Self {
        Self {
            app_wasm,
            table,
            fn_providing_contexts: vec![ "dtrace".to_string() ]
        }
    }

    fn emit_provided_fn(&mut self, context: &String, f: &Fn) -> bool {
        return if context == &"dtrace".to_string() && &f.name == &"strcmp".to_string() {
            self.emit_dtrace_strcmp_fn(f)
        } else {
            error!("Provided function, but could not find a context to provide the definition");
            false
        }
    }

    fn emit_dtrace_strcmp_fn(&mut self, f: &Fn) -> bool {
        let strcmp_params = vec![walrus::ValType::I32, walrus::ValType::I32, walrus::ValType::I32, walrus::ValType::I32];
        let strcmp_result = vec![walrus::ValType::I32];

        let mut strcmp = walrus::FunctionBuilder::new(&mut self.app_wasm.types, &strcmp_params, &strcmp_result);

        // get memory id
        let memory_id = self.app_wasm.memories
            .iter()
            .next()
            .expect("only single memory is supported")
            .id();

        // create params
        let str0_offset = self.app_wasm.locals.add(walrus::ValType::I32);
        let str0_size = self.app_wasm.locals.add(walrus::ValType::I32);
        let str1_offset = self.app_wasm.locals.add(walrus::ValType::I32);
        let str1_size = self.app_wasm.locals.add(walrus::ValType::I32);

        // create locals
        let i = self.app_wasm.locals.add(walrus::ValType::I32);
        let str0_char = self.app_wasm.locals.add(walrus::ValType::I32);
        let str1_char = self.app_wasm.locals.add(walrus::ValType::I32);

        // create the body of strcmp
        strcmp
            .func_body()
            .block(None, |neq_block| {
                let neq = neq_block.id();

                neq_block.block(None, |eq_block| {
                    let eq = eq_block.id();

                    // 1. Check if sizes are equal, if not return 0
                    eq_block
                        .local_get(str0_size)
                        .local_get(str1_size)
                        .binop(BinaryOp::I32Eq)
                        .br_if(neq);

                    // 2. Check if mem offset is equal, if yes return non-zero (we are comparing the same data)
                    eq_block
                        .local_get(str0_offset)
                        .local_get(str1_offset)
                        .binop(BinaryOp::I32Eq)
                        .br_if(eq);

                    // 3. iterate over each string and check equivalence of chars, if any not equal, return 0
                    eq_block
                        .i32_const(0)
                        .local_set(i)
                        .loop_(None, |loop_| {
                            let cmp_char = loop_.id();

                            // Check if we've reached the end of the string
                            loop_
                                .local_get(i)
                                .local_get(str0_size) // (can compare with either str size, equal at this point)
                                .binop(BinaryOp::I32LtU)
                                .i32_const(0)
                                .binop(BinaryOp::I32Eq)
                                .br_if(eq); // We've reached the end without failing equality checks!

                            // get char for str0
                            loop_
                                .local_get(str0_offset)
                                .local_get(i)
                                .binop(BinaryOp::I32Add)
                                .load(
                                    memory_id,
                                    LoadKind::I32_8 {
                                        kind: ExtendedLoad::SignExtend,
                                    },
                                    MemArg {
                                        offset: 0,
                                        align: 1,
                                    },
                                )
                                .local_set(str0_char);

                            // get char for str1
                            loop_
                                .local_get(str1_offset)
                                .local_get(i)
                                .binop(BinaryOp::I32Add)
                                .load(
                                    memory_id,
                                    LoadKind::I32_8 {
                                        kind: ExtendedLoad::SignExtend,
                                    },
                                    MemArg {
                                        offset: 0,
                                        align: 1,
                                    },
                                )
                                .local_set(str1_char);

                            // compare the two chars
                            loop_
                                .local_get(str0_char)
                                .local_get(str1_char)
                                .binop(BinaryOp::I32Ne)
                                .br_if(neq); // If they are not equal, exit and return '0'

                            // Increment i and continue loop
                            loop_
                                .local_get(i)
                                .i32_const(1)
                                .binop(BinaryOp::I32Add)
                                .local_set(i)
                                .br(cmp_char);
                        })
                        // 4. Reached the end of each string without returning, return nonzero
                        .br_if(eq);
                })
                // they are equal, return '1'
                .i32_const(1)
                .return_();
            })
            // they are not equal, return '0'
            .i32_const(0)
            .return_();

        let strcmp_id = strcmp.finish(vec![ str0_offset, str0_size, str1_offset, str1_size ], &mut self.app_wasm.funcs);
        let rec_id = match self.table.lookup(&f.name) {
            Some(rec_id) => rec_id.clone(),
            _ => {
                error!("strcmp fn symbol does not exist in this scope!");
                return false;
            }
        };

        let rec = self.table.get_record_mut(&rec_id);
        return match rec {
            Some(Record::Fn { mut addr, .. }) => {
                addr = Some(strcmp_id);
                true
            },
            Some(ty) => {
                error!("Incorrect global variable record, expected Record::Var, found: {:?}", ty);
                false
            },
            None => {
                error!("Global variable symbol does not exist!");
                false
            }
        };
    }
}
/// Walrus Visitor over `app.wasm`
/// - as we get relevant info, lookup in SymbolTable for binding to globally set that value
/// - for each bytecode, do we have a probe?
///   - fold predicate with known globals. FALSE? Don't inject! NOT FALSE? inject (with remaining Expr, not folded parts)
///   - See fold Rust pattern: https://rust-unofficial.github.io/patterns/patterns/creational/fold.html
/// - now we have instrumented `app.wasm`
///   - write to app_instr.wasm
impl Emitter for WasmRewritingEmitter {
    fn enter_scope(&mut self) {
        self.table.enter_scope();
    }
    fn exit_scope(&mut self) {
        self.table.exit_scope();
    }
    fn emit_dtrace(&mut self, _dtrace: &Dtrace) -> bool {
        // nothing to do here
        true
    }
    fn emit_dscript(&mut self, _dscript: &Dscript) -> bool {
        // nothing to do here
        true
    }
    fn emit_provider(&mut self, provider: &Provider) -> bool {
        let mut is_success = true;
        provider.modules.iter().for_each(|(_name, module)| {
            is_success &= self.emit_module(module);
        });
        is_success
    }
    fn emit_module(&mut self, _module: &Module) -> bool {
        // TODO -- define any compiler constants
        // TODO -- set up `walrus::ir::VisitorMut`
        //         at each bytecode as traversing IR, do we have a `function` for the bytecode?
        //         If so, enter that function
        todo!();
    }
    fn emit_function(&mut self, _function: &Function) -> bool {
        // TODO -- define any compiler constants
        // TODO -- inject probes (should be at this point in the `walrus::ir::VisitorMut` since visited from `visit_module` above
        todo!();
    }
    fn emit_probe(&mut self, _probe: &Probe) -> bool {
        // TODO -- define any compiler constants
        todo!();
    }
    fn emit_fn(&mut self, context: &String, f: &Fn) -> bool {
        self.table.enter_scope();
        // figure out if this is a provided fn.
        if f.is_provided {
            return if self.fn_providing_contexts.contains(context) {
                self.emit_provided_fn(context, f)
            } else {
                error!("Provided function, but could not find a context to provide the definition");
                false
            }
        }

        // TODO -- emit non-provided fn
        //         only when we're supporting user-defined fns in dscript...
        unimplemented!();
        return false
    }
    fn emit_formal_param(&mut self, _param: &(Expr, DataType)) -> bool {
        // TODO -- only when we're supporting user-defined fns in dscript...
        unimplemented!();
        return false
    }
    fn emit_global(&mut self, name: String, _ty: DataType, _val: &Option<Value>) -> bool {
        let rec_id = match self.table.lookup(&name) {
            Some(rec_id) => rec_id.clone(),
            _ => {
                error!("Global variable symbol does not exist in this scope!");
                return false
            } // Ignore, continue to emit
        };

        let rec = self.table.get_record_mut(&rec_id);
        return match rec {
            Some(Record::Var { addr, .. }) => {
                // TODO -- emit global variable and set addr in symbol table
                //         only when we're supporting user-defined globals in dscript...
                unimplemented!();
            },
            Some(ty) => {
                error!("Incorrect global variable record, expected Record::Var, found: {:?}", ty);
                false
            },
            None => {
                error!("Global variable symbol does not exist!");
                return false;
            }
        }
    }

    fn emit_stmt(&mut self, _stmt: &Statement) -> bool {
        todo!()
    }

    fn emit_expr(&mut self, _expr: &Expr) -> bool {
        todo!()
    }

    fn emit_op(&mut self, _op: &Op) -> bool {
        todo!()
    }

    fn emit_datatype(&mut self, _datatype: &DataType) -> bool {
        todo!()
    }

    fn emit_value(&mut self, _val: &Value) -> bool {
        todo!()
    }

    fn dump_to_file(&mut self, output_wasm_path: String) -> bool {
        match self.app_wasm.emit_wasm_file(&output_wasm_path) {
            Ok(_ok) => {
                true
            },
            Err(err) => {
                error!("Failed to dump instrumented wasm to {} from error: {}", &output_wasm_path, err);
                false
            },
        }
    }
}

// =====================
// ==== WasiEmitter ====
// =====================
// TODO

// =======================
// ==== VirgilEmitter ====
// =======================
// TODO
