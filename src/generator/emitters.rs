use crate::common::error::{ErrorGen, WhammError};
use crate::generator::types::ExprFolder;
use crate::parser::types::{BinOp, DataType, Expr, Fn, Statement, UnOp, Value};
use crate::verifier::types::{Record, SymbolTable, VarAddr};
use log::{debug, info};
use orca;
use regex::Regex;
use walrus::ir::{BinaryOp, ExtendedLoad, Instr, InstrSeqId, LoadKind, MemArg};
use walrus::{
    ActiveData, ActiveDataLocation, DataKind, FunctionBuilder, FunctionId, FunctionKind,
    ImportedFunction, InitExpr, InstrSeqBuilder, LocalFunction, MemoryId, ModuleData, ValType,
};
use wasmparser;

// =================================================
// ==== Emitter Trait --> Used By All Emitters! ====
// =================================================

pub trait Emitter {
    fn enter_scope(&mut self) -> Result<(), Box<WhammError>>;
    fn enter_named_scope(&mut self, scope_name: &str) -> bool;
    fn exit_scope(&mut self) -> Result<(), Box<WhammError>>;
    fn reset_children(&mut self);

    fn init_instr_iter(&mut self, instrs_of_interest: &[String]) -> Result<(), Box<WhammError>>;
    fn has_next_instr(&self) -> bool;
    fn init_first_instr(&mut self) -> bool;
    fn next_instr(&mut self) -> bool;
    fn curr_instr_type(&mut self) -> String;
    fn incr_loc_pointer(&mut self);

    fn has_params(&mut self) -> Result<bool, Box<WhammError>>;
    fn save_params(&mut self) -> bool;
    fn emit_params(&mut self) -> Result<bool, Box<WhammError>>;
    fn define_compiler_var(
        &mut self,
        context: &str,
        var_name: &str,
    ) -> Result<bool, Box<WhammError>>;
    // fn emit_event(&mut self, context: &str, event: &mut Event) -> bool;
    fn fold_expr(&mut self, expr: &mut Expr) -> bool;
    fn emit_expr(&mut self, expr: &mut Expr) -> Result<bool, Box<WhammError>>;

    fn emit_fn(&mut self, context_name: &str, f: &Fn) -> Result<bool, Box<WhammError>>;
    fn emit_formal_param(&mut self, param: &(Expr, DataType)) -> bool;
    fn emit_global(
        &mut self,
        name: String,
        ty: DataType,
        val: &Option<Value>,
    ) -> Result<bool, Box<WhammError>>;
    fn remove_orig(&mut self) -> bool;
    fn emit_orig(&mut self) -> bool;
    fn emit_if(&mut self) -> bool;
    fn emit_if_else(&mut self) -> bool;
    /// Will configure the emitter to emit subsequent expression as the condition of an if or if/else stmt
    fn emit_condition(&mut self) -> bool;
    /// Will configure the emitter to emit subsequent statements into the consequent body of an if or if/else stmt
    fn emit_consequent(&mut self) -> bool;
    /// Will configure the emitter to emit subsequent statements into the alternate body of an if/else stmt
    fn emit_alternate(&mut self) -> bool;
    /// Will configure the emitter to emit subsequent statements in the outer block of some branching logic
    fn finish_branch(&mut self) -> bool;
    fn emit_global_stmts(&mut self, stmts: &mut Vec<Statement>) -> Result<bool, Box<WhammError>>;
    fn emit_body(&mut self, body: &mut Vec<Statement>) -> Result<bool, Box<WhammError>>;
    fn has_alt_call(&mut self) -> bool; // TODO -- remove need for this
    fn emit_alt_call(&mut self) -> Result<bool, Box<WhammError>>; // TODO -- remove need for this
    fn emit_stmt(&mut self, stmt: &mut Statement) -> Result<bool, Box<WhammError>>;

    fn dump_to_file(&mut self, output_wasm_path: String) -> Result<bool, Box<WhammError>>;
}

// =================================================================================
// ================ WasmRewritingEmitter - HELPER FUNCTIONS ========================
// Necessary to extract common logic between Emitter and InstrumentationVisitor.
// Can't pass an Emitter instance to InstrumentationVisitor due to Rust not
// allowing nested references to a common mutable object. So I can't pass the
// Emitter to the InstrumentationVisitor since I must iterate over Emitter.app_wasm
// with a construction of InstrumentationVisitor inside that loop.
// =================================================================================
// =================================================================================

const UNEXPECTED_ERR_MSG: &str =
    "WasmRewritingEmitter: Looks like you've found a bug...please report this behavior!";

// Reliant on walrus
fn data_type_to_val_type(ty: &DataType) -> (ValType, InitExpr) {
    match ty {
        DataType::U32 => (ValType::I32, InitExpr::Value(walrus::ir::Value::I32(0))),
        DataType::I32 => (ValType::I32, InitExpr::Value(walrus::ir::Value::I32(0))),
        DataType::Boolean => (ValType::I32, InitExpr::Value(walrus::ir::Value::I32(0))),
        DataType::Null => unimplemented!(),
        DataType::Str => unimplemented!(),
        DataType::Tuple { .. } => unimplemented!(),
        // the ID used to track this var in the lib
        DataType::Map { .. } => (ValType::I32, InitExpr::Value(walrus::ir::Value::I32(0))),
        &DataType::AssumeGood => unimplemented!(),
    }
}

// Reliant on walrus
fn emit_set(
    table: &mut SymbolTable,
    var_id: &mut Expr,
    instr_builder: &mut InstrSeqBuilder,
    index: &mut usize,
) -> Result<bool, Box<WhammError>> {
    if let Expr::VarId { name, .. } = var_id {
        let var_rec_id = match table.lookup(name) {
            Some(rec_id) => *rec_id,
            _ => {
                return Err(Box::new(ErrorGen::get_unexpected_error(
                    true,
                    Some(format!(
                        "{UNEXPECTED_ERR_MSG} \
                                                VarId '{name}' does not exist in this scope!"
                    )),
                    None,
                )));
            }
        };
        match table.get_record_mut(&var_rec_id) {
            Some(Record::Var { addr, loc, .. }) => {
                // this will be different based on if this is a global or local var
                match addr {
                    Some(VarAddr::Global { addr }) => {
                        instr_builder.instr_at(*index, walrus::ir::GlobalSet {
                            global: *addr
                        });
                        // update index to point to what follows our insertions
                        *index += 1;
                    }
                    Some(VarAddr::Local { addr }) => {
                        instr_builder.instr_at(*index, walrus::ir::LocalSet {
                            local: *addr
                        });
                        // update index to point to what follows our insertions
                        *index += 1;
                    },
                    None => {
                        return Err(Box::new(ErrorGen::get_type_check_error_from_loc(false,
                           format!("Variable assigned before declared: {}", name), loc)));
                    }
                }
                Ok(true)
            },
            Some(ty) => {
                Err(Box::new(ErrorGen::get_unexpected_error(true, Some(format!("{UNEXPECTED_ERR_MSG} \
                                                Incorrect variable record, expected Record::Var, found: {:?}", ty)), None)))
            },
            None => {
                Err(Box::new(ErrorGen::get_unexpected_error(true, Some(format!("{UNEXPECTED_ERR_MSG} \
                                                Variable symbol does not exist!")), None)))
            }
        }
    } else {
        Err(Box::new(ErrorGen::get_unexpected_error(
            true,
            Some(format!(
                "{UNEXPECTED_ERR_MSG} \
                                        Expected VarId."
            )),
            None,
        )))
    }
}

// Reliant on walrus
fn emit_expr(
    table: &mut SymbolTable,
    module_data: &mut ModuleData,
    expr: &mut Expr,
    instr_builder: &mut InstrSeqBuilder,
    metadata: &mut InsertionMetadata,
    index: &mut usize,
) -> Result<bool, Box<WhammError>> {
    let mut is_success = true;
    match expr {
        Expr::UnOp { op, expr, .. } => {
            is_success &= emit_expr(table, module_data, expr, instr_builder, metadata, index)?;
            is_success &= emit_unop(op, instr_builder, index);
        }
        Expr::BinOp { lhs, op, rhs, .. } => {
            is_success &= emit_expr(table, module_data, lhs, instr_builder, metadata, index)?;
            is_success &= emit_expr(table, module_data, rhs, instr_builder, metadata, index)?;
            is_success &= emit_binop(op, instr_builder, index);
        }
        Expr::Ternary {
            cond: _cond,
            conseq: _conseq,
            alt: _alt,
            ..
        } => {
            return Err(Box::new(ErrorGen::get_unexpected_error(
                true,
                Some(format!(
                    "{UNEXPECTED_ERR_MSG} \
                            Ternary expressions should be handled before this point!"
                )),
                None,
            )));
        }
        Expr::Call {
            fn_target, args, ..
        } => {
            let fn_name = match &**fn_target {
                Expr::VarId { name, .. } => name.clone(),
                _ => return Ok(false),
            };

            // emit the arguments
            if let Some(args) = args {
                for boxed_arg in args.iter_mut() {
                    let arg = &mut **boxed_arg; // unbox
                    is_success &=
                        emit_expr(table, module_data, arg, instr_builder, metadata, index)?;
                }
            }

            let fn_rec_id = table.lookup(&fn_name).copied();

            match fn_rec_id {
                Some(rec_id) => {
                    let fn_rec = table.get_record_mut(&rec_id);
                    match fn_rec {
                        Some(Record::Fn { addr, .. }) => {
                            if let Some(f_id) = addr {
                                instr_builder.instr_at(*index, walrus::ir::Call { func: *f_id });
                                // update index to point to what follows our insertions
                                *index += 1;
                            } else {
                                return Err(Box::new(ErrorGen::get_unexpected_error(
                                    true,
                                    Some(format!(
                                        "{UNEXPECTED_ERR_MSG} \
                                fn_target address not in symbol table, not emitted yet..."
                                    )),
                                    None,
                                )));
                            }
                        }
                        _ => {
                            return Err(Box::new(ErrorGen::get_unexpected_error(
                                true,
                                Some(format!(
                                    "{UNEXPECTED_ERR_MSG} \
                            fn_target not defined in symbol table!"
                                )),
                                None,
                            )));
                        }
                    }
                }
                None => {
                    // Must be defined in the Wasm
                    unimplemented!()
                }
            }
        }
        Expr::VarId { name, .. } => {
            // TODO -- support string vars (unimplemented)
            let var_rec_id = match table.lookup(name) {
                Some(rec_id) => *rec_id,
                _ => {
                    return Err(Box::new(ErrorGen::get_unexpected_error(
                        true,
                        Some(format!(
                            "{UNEXPECTED_ERR_MSG} \
                    VarId '{}' does not exist in this scope!",
                            name
                        )),
                        None,
                    )));
                }
            };
            return match table.get_record_mut(&var_rec_id) {
                Some(Record::Var { addr, .. }) => {
                    // this will be different based on if this is a global or local var
                    match addr {
                        Some(VarAddr::Global { addr }) => {
                            instr_builder.instr_at(*index, walrus::ir::GlobalGet { global: *addr });
                            // update index to point to what follows our insertions
                            *index += 1;
                        }
                        Some(VarAddr::Local { addr }) => {
                            instr_builder.instr_at(*index, walrus::ir::LocalGet { local: *addr });
                            // update index to point to what follows our insertions
                            *index += 1;
                        }
                        None => {
                            return Err(Box::new(ErrorGen::get_unexpected_error(
                                true,
                                Some(format!(
                                    "{UNEXPECTED_ERR_MSG} \
                            Variable does not exist in scope: {}",
                                    name
                                )),
                                None,
                            )));
                        }
                    }
                    Ok(true)
                }
                Some(ty) => Err(Box::new(ErrorGen::get_unexpected_error(
                    true,
                    Some(format!(
                        "{UNEXPECTED_ERR_MSG} \
                    Incorrect variable record, expected Record::Var, found: {:?}",
                        ty
                    )),
                    None,
                ))),
                None => Err(Box::new(ErrorGen::get_unexpected_error(
                    true,
                    Some(format!(
                        "{UNEXPECTED_ERR_MSG} \
                    Variable symbol does not exist!"
                    )),
                    None,
                ))),
            };
        }
        Expr::Primitive { val, .. } => {
            is_success &= emit_value(table, module_data, val, instr_builder, metadata, index)?;
        }
    }
    Ok(is_success)
}

// Reliant on Walrus
fn emit_binop(op: &BinOp, instr_builder: &mut InstrSeqBuilder, index: &mut usize) -> bool {
    match op {
        BinOp::And => {
            // we only support i32's at the moment
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32And,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::Or => {
            // we only support i32's at the moment
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32Or,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::EQ => {
            // we only support i32's at the moment
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32Eq,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::NE => {
            // we only support i32's at the moment
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32Ne,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::GE => {
            // we only support i32's at the moment (assumes signed)
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32GeS,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::GT => {
            // we only support i32's at the moment (assumes signed)
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32GtS,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::LE => {
            // we only support i32's at the moment (assumes signed)
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32LeS,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::LT => {
            // we only support i32's at the moment (assumes signed)
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32LtS,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::Add => {
            // we only support i32's at the moment (assumes signed)
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32Add,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::Subtract => {
            // we only support i32's at the moment (assumes signed)
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32Sub,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::Multiply => {
            // we only support i32's at the moment (assumes signed)
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32Mul,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::Divide => {
            // we only support i32's at the moment (assumes signed)
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32DivS,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
        BinOp::Modulo => {
            // we only support i32's at the moment (assumes signed)
            instr_builder.instr_at(
                *index,
                walrus::ir::Binop {
                    op: BinaryOp::I32RemS,
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
    }
}

// Reliant on walrus
fn emit_unop(op: &UnOp, instr_builder: &mut InstrSeqBuilder, index: &mut usize) -> bool {
    match op {
        UnOp::Not => {
            instr_builder.instr_at(
                *index,
                walrus::ir::Unop {
                    op: walrus::ir::UnaryOp::I32Eqz, // return 1 if 0, return 0 otherwise
                },
            );
            // update index to point to what follows our insertions
            *index += 1;
            true
        }
    }
}

// Reliant on walrus
// Alex: Why is this not in impl Emitter?
fn emit_value(
    table: &mut SymbolTable,
    module_data: &mut ModuleData,
    val: &mut Value,
    instr_builder: &mut InstrSeqBuilder,
    metadata: &mut InsertionMetadata,
    index: &mut usize,
) -> Result<bool, Box<WhammError>> {
    Ok(true)
}

// Reliant on walrus
fn get_func_info(app_wasm: &walrus::Module, func: &walrus::Function) -> (FuncInfo, Vec<ValType>) {
    match &func.kind {
        FunctionKind::Import(ImportedFunction {
            ty: ty_id,
            import: import_id,
        }) => {
            let import = app_wasm.imports.get(*import_id);
            let ty = app_wasm.types.get(*ty_id);

            (
                FuncInfo {
                    func_kind: "import".to_string(),
                    module: import.module.clone(),
                    name: import.name.clone(),
                },
                Vec::from(ty.params()),
            )
        }
        FunctionKind::Local(LocalFunction { args, .. }) => {
            let mut params = vec![];
            args.iter().for_each(|arg_id| {
                let arg = app_wasm.locals.get(*arg_id);
                params.push(arg.ty());
            });

            (
                FuncInfo {
                    func_kind: "local".to_string(),
                    module: "".to_string(),
                    name: func.name.clone().unwrap_or("".to_string()),
                },
                params,
            )
        }
        FunctionKind::Uninitialized(ty_id) => {
            let ty = app_wasm.types.get(*ty_id);

            (
                FuncInfo {
                    func_kind: "uninitialized".to_string(),
                    module: "".to_string(),
                    name: "".to_string(),
                },
                Vec::from(ty.params()),
            )
        }
    }
}

// ==============================
// ==== WasmRewritingEmitter ====
// ==============================

// Reliant on walrus
struct InsertionMetadata {
    // curr_event: String,
    mem_id: Option<MemoryId>,
    curr_mem_offset: u32,
}

#[derive(Debug)]
struct InstrIter {
    instr_locs: Vec<ProbeLoc>,
    curr_loc: usize,
}

// Reliant on walrus
impl InstrIter {
    /// Build out a list of all local functions and their blocks/instruction indexes
    /// to visit while doing instrumentation.
    fn new() -> Self {
        Self {
            instr_locs: vec![],
            curr_loc: 0,
        }
    }
    fn init(&mut self, app_wasm: &orca::ir::Module, instrs_of_interest: &[String]) {
        // Figure out which functions to visit
        eprintln!("{:?}", app_wasm);
        for func in app_wasm.code_sections.iter() {
            // iterate each instruction of the function
            self.init_instr_locs(
                instrs_of_interest,
                app_wasm,
                func,
                // &func.id(),
                // func.name.clone(),
                // func.entry_block(),
            );

            // let func_id = func.id();
            // if let Some(name) = func.name.as_ref() {
            //     // TODO -- get rid of this necessity (probably by removing the need to have
            //     //         functions already present in the app code)
            //     if name.starts_with("instr_") {
            //         continue;
            //     }
            // }

            // in orca, all functions (in code_sections) are local
            // why this get called twice?
            // self.init_instr_locs(
            //     instrs_of_interest,
            //     app_wasm,
            //     func,
            //     &func.id(),
            //     func.name.clone(),
            //     func.entry_block(),
            // );
            // if let FunctionKind::Local(local_func) = &func.kind {
            //     // TODO -- make sure that the id is not any of the injected function IDs (strcmp)
            //     self.init_instr_locs(
            //         instrs_of_interest,
            //         app_wasm,
            //         local_func,
            //         &func_id,
            //         func.name.clone(),
            //         local_func.entry_block(),
            //     );
            // }
        }
        debug!("Finished creating list of instructions to visit");
    }

    // Reliant on walrus
    // instrument each function
    // Alex: I think the naming is a bit confusing
    fn init_instr_locs(
        &mut self,
        instrs_of_interest: &[String],
        app_wasm: &orca::ir::Module,
        func_body: &orca::ir::Body, // func: &LocalFunction,
                                    // func_id: &FunctionId,
                                    // func_name: Option<String>,
                                    // instr_seq_id: InstrSeqId,
    ) {
        // for each instruction in the function, check if it's an instruction of interest
        for instruction in &func_body.instructions {
            // https://docs.rs/wasmparser/latest/wasmparser/enum.Operator.html
            // only care about call instructions now
            match instruction {
                wasmparser::Operator::Call { .. } => {}
                _ => {
                    // do nothing extra for other instructions
                }
            }
        }

        // func.block(instr_seq_id)
        //     .iter()
        //     .enumerate()
        //     .for_each(|(index, (instr, _))| {
        //         let instr_as_str = &format!("{:?}", instr);
        //         let instr_name = instr_as_str.split('(').next().unwrap().to_lowercase();

        //         if instrs_of_interest.contains(&instr_name) {
        //             let (func_info, params) = if let Instr::Call(func) = instr {
        //                 let func = app_wasm.funcs.get(func.func);
        //                 // get information about the function call
        //                 let (func_info, params) = get_func_info(app_wasm, func);
        //                 (Some(func_info), params)
        //             } else {
        //                 (None, vec![])
        //             };

        //             // // add current instr
        //             // self.instr_locs.push(ProbeLoc {
        //             //     // wasm_func_name: func_name.clone(),
        //             //     wasm_func_id: *func_id,
        //             //     instr_seq_id,
        //             //     index,
        //             //     instr_name: instr_name.clone(),
        //             //     instr: instr.clone(),
        //             //     instr_params: params,
        //             //     instr_created_args: vec![],
        //             //     instr_alt_call: None,
        //             //     // instr_symbols: HashMap::new()
        //             //     func_info,
        //             // });
        //         }

        //         // visit nested blocks
        //         match instr {
        //             Instr::Block(block) => {
        //                 self.init_instr_locs(
        //                     instrs_of_interest,
        //                     app_wasm,
        //                     func,
        //                     func_id,
        //                     func_name.clone(),
        //                     block.seq,
        //                 );
        //             }
        //             Instr::Loop(_loop) => {
        //                 self.init_instr_locs(
        //                     instrs_of_interest,
        //                     app_wasm,
        //                     func,
        //                     func_id,
        //                     func_name.clone(),
        //                     _loop.seq,
        //                 );
        //             }
        //             Instr::IfElse(if_else, ..) => {
        //                 println!("IfElse: {:#?}", if_else);
        //                 self.init_instr_locs(
        //                     instrs_of_interest,
        //                     app_wasm,
        //                     func,
        //                     func_id,
        //                     func_name.clone(),
        //                     if_else.consequent,
        //                 );
        //                 self.init_instr_locs(
        //                     instrs_of_interest,
        //                     app_wasm,
        //                     func,
        //                     func_id,
        //                     func_name.clone(),
        //                     if_else.alternative,
        //                 );
        //             }
        //             _ => {
        //                 // do nothing extra for other instructions
        //             }
        //         }
        //     });
    }
    fn has_next(&self) -> bool {
        self.curr_loc + 1 < self.instr_locs.len()
    }
    fn next(&mut self) -> Option<&ProbeLoc> {
        self.curr_loc += 1;
        self.curr()
    }
    fn curr(&self) -> Option<&ProbeLoc> {
        self.instr_locs.get(self.curr_loc)
    }
    fn curr_mut(&mut self) -> Option<&mut ProbeLoc> {
        self.instr_locs.get_mut(self.curr_loc)
    }
}

// Struct to store info on insertion locations for an instruction sequence.
// Note that blocks can be indefinitely nested.

// Reliant on walrus
#[derive(Debug)]
struct ProbeLoc {
    // wasm_func_name: Option<String>,
    // https://docs.rs/wasmparser/latest/wasmparser/enum.Operator.html#variant.Call.field.function_index
    wasm_func_id: u32,
    // instr_seq_id: InstrSeqId,
    // index: usize,
    instr_name: String,
    instr: Instr,
    func_info: Option<FuncInfo>,
    instr_params: Vec<ValType>,
    instr_created_args: Vec<(String, usize)>,

    // Save off the compiler-defined constants for this instruction
    // instr_symbols: HashMap<String, Record>,
    instr_alt_call: Option<FunctionId>,
}
#[derive(Debug)]
struct FuncInfo {
    func_kind: String,
    module: String,
    name: String,
}

struct EmittingInstrTracker {
    // To keep track of the location of the original instruction while we're instrumenting!
    orig_instr_idx: usize,

    curr_seq_id: InstrSeqId,
    curr_idx: usize,

    /// The sequence ID of the main block (containing the instruction of-interest)
    main_seq_id: InstrSeqId,
    /// The current index into the main block (containing the instruction of-interest)
    main_idx: usize,

    /// The sequence ID of the outer block of an injected conditional
    outer_seq_id: Option<InstrSeqId>,
    /// The current index into the outer block of an injected conditional
    outer_idx: Option<usize>,

    /// The sequence ID of the consequent block of an injected conditional
    then_seq_id: Option<InstrSeqId>,
    /// The current index into the consequent block of an injected conditional
    then_idx: Option<usize>,

    /// The sequence ID of the alternate block of an injected conditional
    else_seq_id: Option<InstrSeqId>,
    /// The current index into the alternate block of an injected conditional
    else_idx: Option<usize>,
}

pub struct WasmRewritingEmitter<'a> {
    pub app_wasm: orca::ir::Module<'a>,
    pub table: SymbolTable,

    // whamm! AST traversal bookkeeping
    metadata: InsertionMetadata,
    instr_iter: InstrIter,
    emitting_instr: Option<EmittingInstrTracker>,

    fn_providing_contexts: Vec<String>,
}
impl<'a> WasmRewritingEmitter<'a> {
    pub fn new(app_wasm: orca::ir::Module<'a>, table: SymbolTable) -> Self {
        // not sur what mem_id does
        // let mem_id = app_wasm
        //     .memories
        //     .iter()
        //     .next()
        //     .expect("only single memory is supported")
        //     .id();
        let mem_id = None;

        Self {
            app_wasm,
            table,
            metadata: InsertionMetadata {
                // curr_event: "".to_string(),
                mem_id,
                curr_mem_offset: 1_052_576, // Set default memory base address to DEFAULT + 4KB = 1048576 bytes + 4000 bytes = 1052576 bytes
            },
            instr_iter: InstrIter::new(),
            emitting_instr: None,
            fn_providing_contexts: vec!["whamm".to_string()],
        }
    }

    fn override_var_val(&mut self, rec_id: &usize, val: Option<Value>) {
        let mut rec = self.table.get_record_mut(rec_id);
        if let Some(Record::Var { value, .. }) = &mut rec {
            *value = val;
        }
    }

    fn define_new_target_fn_name(&mut self) -> Result<bool, Box<WhammError>> {
        // TODO -- change this to be an inline call() instead of setting a var
        Ok(true)
    }

    fn define_target_imp_name(&mut self) -> Result<bool, Box<WhammError>> {
        let var_name = "target_imp_name".to_string();

        if let Some(curr_instr) = self.instr_iter.curr() {
            if let Some(func_info) = &curr_instr.func_info {
                if func_info.name.contains("call_perform") {
                    // For debugging, set breakpoint here!
                    println!("{}", func_info.name);
                }

                let rec_id = match self.table.lookup(&var_name) {
                    Some(rec_id) => *rec_id,
                    _ => {
                        return Err(Box::new(ErrorGen::get_unexpected_error(
                            true,
                            Some(format!(
                                "{UNEXPECTED_ERR_MSG} \
                        `{var_name}` symbol does not exist in this scope!"
                            )),
                            None,
                        )));
                    }
                };
                self.override_var_val(
                    &rec_id,
                    Some(Value::Str {
                        ty: DataType::Str,
                        val: func_info.name.to_string(),
                        addr: None,
                    }),
                );
            }
        }
        Ok(true)
    }

    fn define_target_fn_type(&mut self) -> Result<bool, Box<WhammError>> {
        let var_name = "target_fn_type".to_string();

        if let Some(curr_instr) = self.instr_iter.curr() {
            if let Some(func_info) = &curr_instr.func_info {
                // if func_info.name.contains("call_new") {
                //     // For debugging, set breakpoint here!
                //     println!("{}", func_info.name);
                // }
                let rec_id = match self.table.lookup(&var_name) {
                    Some(rec_id) => *rec_id,
                    _ => {
                        return Err(Box::new(ErrorGen::get_unexpected_error(
                            true,
                            Some(format!(
                                "{UNEXPECTED_ERR_MSG} \
                        `{var_name}` symbol does not exist in this scope!"
                            )),
                            None,
                        )));
                    }
                };
                self.override_var_val(
                    &rec_id,
                    Some(Value::Str {
                        ty: DataType::Str,
                        val: func_info.func_kind.to_string(),
                        addr: None,
                    }),
                );
            }
        }
        Ok(true)
    }

    fn define_target_imp_module(&mut self) -> Result<bool, Box<WhammError>> {
        let var_name = "target_imp_module".to_string();
        if let Some(curr_instr) = self.instr_iter.curr() {
            if let Some(func_info) = &curr_instr.func_info {
                // if func_info.name.contains("call_new") {
                //     // For debugging, set breakpoint here!
                //     println!("{}", func_info.name);
                // }
                let rec_id = match self.table.lookup(&var_name) {
                    Some(rec_id) => *rec_id,
                    _ => {
                        return Err(Box::new(ErrorGen::get_unexpected_error(
                            true,
                            Some(format!(
                                "{UNEXPECTED_ERR_MSG} \
                        `{var_name}` symbol does not exist in this scope!"
                            )),
                            None,
                        )));
                    }
                };
                self.override_var_val(
                    &rec_id,
                    Some(Value::Str {
                        ty: DataType::Str,
                        val: func_info.module.to_string(),
                        addr: None,
                    }),
                );
            }
        }
        Ok(true)
    }

    fn emit_provided_fn(&mut self, context: &str, f: &Fn) -> Result<bool, Box<WhammError>> {
        if context == "whamm" && f.name.name == "strcmp" {
            self.emit_whamm_strcmp_fn(f)
        } else {
            Err(Box::new(ErrorGen::get_unexpected_error(
                true,
                Some(format!(
                    "{UNEXPECTED_ERR_MSG} \
            Provided function, but could not find a context to provide the definition, context: {}",
                    context
                )),
                None,
            )))
        }
    }

    // Reliant on walrus
    // do nothing for now
    fn emit_whamm_strcmp_fn(&mut self, f: &Fn) -> Result<bool, Box<WhammError>> {
        Ok(true)
    }

    fn emit_decl_stmt(&mut self, stmt: &mut Statement) -> Result<bool, Box<WhammError>> {
        Ok(true)
    }

    fn emit_assign_stmt(&mut self, stmt: &mut Statement) -> Result<bool, Box<WhammError>> {
        Ok(true)
    }
}

impl Emitter for WasmRewritingEmitter<'_> {
    fn enter_scope(&mut self) -> Result<(), Box<WhammError>> {
        self.table.enter_scope()
    }
    fn enter_named_scope(&mut self, scope_name: &str) -> bool {
        self.table.enter_named_scope(scope_name)
    }
    fn exit_scope(&mut self) -> Result<(), Box<WhammError>> {
        self.table.exit_scope()
    }
    fn reset_children(&mut self) {
        self.table.reset_children();
    }

    fn init_instr_iter(&mut self, instrs_of_interest: &[String]) -> Result<(), Box<WhammError>> {
        self.instr_iter.init(&self.app_wasm, instrs_of_interest);
        Ok(())
    }

    /// bool -> whether there is a next instruction to process
    fn has_next_instr(&self) -> bool {
        self.instr_iter.has_next()
    }

    fn init_first_instr(&mut self) -> bool {
        // if let Some(first) = self.instr_iter.curr() {
        //     self.emitting_instr = Some(EmittingInstrTracker {
        //         orig_instr_idx: first.index,
        //         curr_seq_id: first.instr_seq_id,
        //         curr_idx: first.index,
        //         main_seq_id: first.instr_seq_id,
        //         main_idx: first.index,
        //         outer_seq_id: None,
        //         outer_idx: None,
        //         then_seq_id: None,
        //         then_idx: None,
        //         else_seq_id: None,
        //         else_idx: None,
        //     });
        //     return true;
        // }
        false
    }

    /// bool -> whether it found a next instruction
    fn next_instr(&mut self) -> bool {
        if self.instr_iter.has_next() {
            // if let Some(next) = self.instr_iter.next() {
            //     self.emitting_instr = Some(EmittingInstrTracker {
            //         orig_instr_idx: next.index,
            //         curr_seq_id: next.instr_seq_id,
            //         curr_idx: next.index,
            //         main_seq_id: next.instr_seq_id,
            //         main_idx: next.index,
            //         outer_seq_id: None,
            //         outer_idx: None,
            //         then_seq_id: None,
            //         then_idx: None,
            //         else_seq_id: None,
            //         else_idx: None,
            //     });
            //     return true;
            // }
        }
        false
    }

    /// bool -> whether the current instruction is one of the passed list of types
    fn curr_instr_type(&mut self) -> String {
        if let Some(instr) = self.instr_iter.curr() {
            return instr.instr_name.clone();
        }
        unreachable!()
    }

    fn incr_loc_pointer(&mut self) {
        if let Some(tracker) = &mut self.emitting_instr {
            tracker.curr_idx += 1;
            tracker.main_idx += 1;
        }
    }

    fn has_params(&mut self) -> Result<bool, Box<WhammError>> {
        if let Some(curr_instr) = self.instr_iter.curr_mut() {
            return Ok(!curr_instr.instr_params.is_empty());
        }
        Err(Box::new(ErrorGen::get_unexpected_error(
            true,
            Some(format!(
                "{UNEXPECTED_ERR_MSG} \
        Something went wrong when trying to access the current instruction."
            )),
            None,
        )))
    }

    fn save_params(&mut self) -> bool {
        // if let Some(curr_loc) = self.instr_iter.curr_mut() {
        //     if let Some(tracker) = &mut self.emitting_instr {
        //         let func = self
        //             .app_wasm
        //             .funcs
        //             .get_mut(curr_loc.wasm_func_id)
        //             .kind
        //             .unwrap_local_mut();
        //         let func_builder = func.builder_mut();
        //         let mut instr_builder = func_builder.instr_seq(tracker.curr_seq_id);

        //         // No bytecodes should have been emitted in the module yet!
        //         // So, we can just save off the first * items in the stack as the args
        //         // to the call.
        //         let mut arg_recs = vec![]; // vec to retain order!
        //         curr_loc
        //             .instr_params
        //             .iter()
        //             .enumerate()
        //             .for_each(|(num, param_ty)| {
        //                 // create local for the param in the module
        //                 let arg_local_id = self.app_wasm.locals.add(*param_ty);

        //                 // emit a bytecode in the event to assign the ToS to this new local
        //                 instr_builder.instr_at(
        //                     tracker.curr_idx,
        //                     walrus::ir::LocalSet {
        //                         local: arg_local_id,
        //                     },
        //                 );

        //                 // update index of tracker to point to what follows our insertions
        //                 tracker.curr_idx += 1;

        //                 // also update index to point to new location of instrumented instruction!
        //                 // (saved params go before the original instruction)
        //                 tracker.orig_instr_idx += 1;

        //                 // place in symbol table with var addr for future reference
        //                 let arg_name = format!("arg{}", num);
        //                 let id = self.table.put(
        //                     arg_name.clone(),
        //                     Record::Var {
        //                         ty: DataType::I32, // we only support integers right now.
        //                         name: arg_name.clone(),
        //                         value: None,
        //                         is_comp_provided: false,
        //                         addr: Some(VarAddr::Local { addr: arg_local_id }),
        //                         loc: None,
        //                     },
        //                 );
        //                 arg_recs.push((arg_name, id));
        //             });
        //         curr_loc.instr_created_args = arg_recs;
        //         return true;
        //     }
        // }
        false
    }

    fn emit_params(&mut self) -> Result<bool, Box<WhammError>> {
        //     if let Some(curr_loc) = self.instr_iter.curr_mut() {
        //         if let Some(tracker) = &mut self.emitting_instr {
        //             let func = self
        //                 .app_wasm
        //                 .funcs
        //                 .get_mut(curr_loc.wasm_func_id)
        //                 .kind
        //                 .unwrap_local_mut();
        //             let func_builder = func.builder_mut();
        //             let mut instr_builder = func_builder.instr_seq(tracker.curr_seq_id);

        //             for (_param_name, param_rec_id) in curr_loc.instr_created_args.iter() {
        //                 let param_rec = self.table.get_record_mut(param_rec_id);
        //                 if let Some(Record::Var {
        //                     addr: Some(VarAddr::Local { addr }),
        //                     ..
        //                 }) = param_rec
        //                 {
        //                     // Inject at tracker.orig_instr_idx to make sure that this actually emits the params
        //                     // for the instrumented instruction right before that instruction is called!
        //                     instr_builder.instr_at(
        //                         tracker.orig_instr_idx,
        //                         walrus::ir::LocalGet { local: *addr },
        //                     );

        //                     // update index to point to new location of instrumented instruction!
        //                     // (re-emitted params go before the original instruction)
        //                     tracker.orig_instr_idx += 1;
        //                 } else {
        //                     return Err(Box::new(ErrorGen::get_unexpected_error(
        //                         true,
        //                         Some(format!(
        //                             "{UNEXPECTED_ERR_MSG} \
        //                     Could not emit parameters, something went wrong..."
        //                         )),
        //                         None,
        //                     )));
        //                 }
        //             }
        //             return Ok(true);
        //         }
        //     }
        Ok(false)
    }

    fn define_compiler_var(
        &mut self,
        context: &str,
        var_name: &str,
    ) -> Result<bool, Box<WhammError>> {
        // let regex = Regex::new(r"whamm:script([0-9]+):wasm:bytecode").unwrap();
        // return if let Some(_caps) = regex.captures(context) {
        //     match var_name {
        //         "new_target_fn_name" => self.define_new_target_fn_name(),
        //         "target_imp_name" => self.define_target_imp_name(),
        //         "target_fn_type" => self.define_target_fn_type(),
        //         "target_imp_module" => self.define_target_imp_module(),
        //         _ => {
        //             return Err(Box::new(ErrorGen::get_unexpected_error(
        //                 true,
        //                 Some(format!(
        //                     "{UNEXPECTED_ERR_MSG} \
        //             Current context `{}` does not provide definition for variable `{}`",
        //                     context, var_name
        //                 )),
        //                 None,
        //             )));
        //         }
        //     }
        // } else {
        //     return Err(Box::new(ErrorGen::get_unexpected_error(
        //         true,
        //         Some(format!(
        //             "{UNEXPECTED_ERR_MSG} \
        //     Could not find a context to provide the definition, context: {}",
        //             context
        //         )),
        //         None,
        //     )));
        // };
        Ok(true)
    }

    fn fold_expr(&mut self, expr: &mut Expr) -> bool {
        *expr = ExprFolder::fold_expr(expr, &self.table);
        true
    }
    fn emit_expr(&mut self, expr: &mut Expr) -> Result<bool, Box<WhammError>> {
        let mut is_success = true;
        match expr {
            Expr::Ternary {
                cond, conseq, alt, ..
            } => {
                is_success &= self.emit_if_else();
                is_success &= self.emit_condition();
                is_success &= self.emit_expr(cond)?;
                is_success &= self.emit_consequent();
                is_success &= self.emit_expr(conseq)?;
                is_success &= self.emit_alternate();
                is_success &= self.emit_expr(alt)?;
                is_success &= self.finish_branch();
            }
            Expr::VarId { .. }
            | Expr::UnOp { .. }
            | Expr::BinOp { .. }
            | Expr::Primitive { .. }
            | Expr::Call { .. } => {
                // // Anything else can be emitted as normal
                // if let Some(curr_loc) = self.instr_iter.curr_mut() {
                //     if let Some(tracker) = &mut self.emitting_instr {
                //         let func = self
                //             .app_wasm
                //             .funcs
                //             .get_mut(curr_loc.wasm_func_id)
                //             .kind
                //             .unwrap_local_mut();
                //         let func_builder = func.builder_mut();
                //         let mut instr_builder = func_builder.instr_seq(tracker.curr_seq_id);

                //         is_success &= emit_expr(
                //             &mut self.table,
                //             &mut self.app_wasm.data,
                //             expr,
                //             &mut instr_builder,
                //             &mut self.metadata,
                //             &mut tracker.curr_idx,
                //         )?;
                //     } else {
                //         return Err(Box::new(ErrorGen::get_unexpected_error(
                //             true,
                //             Some(format!(
                //                 "{UNEXPECTED_ERR_MSG} \
                //             Something went wrong while emitting an instruction."
                //             )),
                //             None,
                //         )));
                //     }
                // } else {
                //     return Err(Box::new(ErrorGen::get_unexpected_error(
                //         true,
                //         Some(format!(
                //             "{UNEXPECTED_ERR_MSG} \
                //         Something went wrong while emitting an instruction."
                //         )),
                //         None,
                //     )));
                // }
            }
        }
        Ok(is_success)
    }
    fn emit_fn(&mut self, context: &str, f: &Fn) -> Result<bool, Box<WhammError>> {
        // figure out if this is a provided fn.
        if f.is_comp_provided {
            return if self.fn_providing_contexts.contains(&context.to_string()) {
                self.emit_provided_fn(context, f)
            } else {
                Err(Box::new(ErrorGen::get_unexpected_error(
                    true,
                    Some(format!(
                        "{UNEXPECTED_ERR_MSG} \
                Provided fn, but could not find a context to provide the definition, context: {}",
                        context
                    )),
                    None,
                )))
            };
        }

        // emit non-provided fn
        // only when we're supporting user-defined fns in script...
        unimplemented!();
    }

    fn emit_formal_param(&mut self, _param: &(Expr, DataType)) -> bool {
        // only when we're supporting user-defined fns in script...
        unimplemented!();
    }

    fn emit_global(
        &mut self,
        name: String,
        ty: DataType,
        _val: &Option<Value>,
    ) -> Result<bool, Box<WhammError>> {
        let rec_id = match self.table.lookup(&name) {
            Some(rec_id) => *rec_id,
            _ => {
                return Err(Box::new(ErrorGen::get_unexpected_error(
                    true,
                    Some(format!(
                        "{UNEXPECTED_ERR_MSG} \
                Global variable symbol does not exist in this scope!"
                    )),
                    None,
                )));
            } // Ignore, continue to emit
        };

        let rec = self.table.get_record_mut(&rec_id);
        match rec {
            Some(Record::Var { ref mut addr, .. }) => {
                // emit global variable and set addr in symbol table
                // this is used for user-defined global vars in the script...
                let (walrus_ty, init_expr) = data_type_to_val_type(&ty);
                // let id = self.app_wasm.globals.add_local(walrus_ty, true, init_expr);
                // *addr = Some(VarAddr::Global { addr: id });

                Ok(true)
            }
            Some(&mut ref ty) => Err(Box::new(ErrorGen::get_unexpected_error(
                true,
                Some(format!(
                    "{UNEXPECTED_ERR_MSG} \
                Incorrect global variable record, expected Record::Var, found: {:?}",
                    ty
                )),
                None,
            ))),
            None => Err(Box::new(ErrorGen::get_unexpected_error(
                true,
                Some(format!(
                    "{UNEXPECTED_ERR_MSG} \
                Global variable symbol does not exist!"
                )),
                None,
            ))),
        }
    }

    fn remove_orig(&mut self) -> bool {
        if let Some(curr_loc) = self.instr_iter.curr_mut() {
            // if let Some(tracker) = &self.emitting_instr {
            //     let func = self
            //         .app_wasm
            //         .funcs
            //         .get_mut(curr_loc.wasm_func_id)
            //         .kind
            //         .unwrap_local_mut();
            //     let func_builder = func.builder_mut();
            //     let mut instr_builder = func_builder.instr_seq(tracker.curr_seq_id);

            //     instr_builder.instrs_mut().remove(tracker.curr_idx);
            //     return true;
            // }
        }
        false
    }

    fn emit_orig(&mut self) -> bool {
        // if let Some(curr_loc) = self.instr_iter.curr_mut() {
        //     if let Some(tracker) = &mut self.emitting_instr {
        //         let func = self
        //             .app_wasm
        //             .funcs
        //             .get_mut(curr_loc.wasm_func_id)
        //             .kind
        //             .unwrap_local_mut();
        //         let func_builder = func.builder_mut();
        //         let mut instr_builder = func_builder.instr_seq(tracker.curr_seq_id);

        //         // reset where the "orig instruction" is located in the bytecode
        //         tracker.orig_instr_idx = tracker.curr_idx;
        //         instr_builder.instr_at(tracker.curr_idx, curr_loc.instr.clone());
        //         return true;
        //     }
        // }
        false
    }

    fn emit_if(&mut self) -> bool {
        // if let Some(curr_loc) = self.instr_iter.curr_mut() {
        //     if let Some(tracker) = &mut self.emitting_instr {
        //         // This MUST be `self.app_wasm` so we're mutating what will be the instrumented application.
        //         let func = self
        //             .app_wasm
        //             .funcs
        //             .get_mut(curr_loc.wasm_func_id)
        //             .kind
        //             .unwrap_local_mut();
        //         let func_builder = func.builder_mut();
        //         let mut instr_builder = func_builder.instr_seq(tracker.curr_seq_id);

        //         let mut outer_seq_id = None;
        //         let mut outer_idx = None;
        //         let mut then_seq_id = None;
        //         let mut then_idx = None;

        //         instr_builder.block_at(tracker.curr_idx, None, |outer_block| {
        //             let outer_id = outer_block.id();
        //             outer_seq_id = Some(outer_id);
        //             outer_idx = Some(0usize);

        //             // CONDITION SHOULD BE EMITTED HERE

        //             // If the block evaluates to true (any nonzero value), execute the body.
        //             // If result of predicate equals 0, break out of the probe block
        //             // to continue with the application code.
        //             outer_block
        //                 .i32_const(0)
        //                 .binop(BinaryOp::I32Eq)
        //                 .br_if(outer_id);

        //             outer_block.block(None, |then| {
        //                 then_seq_id = Some(then.id());
        //                 then_idx = Some(0usize);

        //                 // CONSEQUENT SHOULD BE EMITTED HERE
        //             });
        //         });

        //         // Save the block information for future reference
        //         // leave outer_block_idx as 0 to enable injection of condition!
        //         tracker.outer_seq_id = outer_seq_id;
        //         tracker.outer_idx = outer_idx;
        //         tracker.then_seq_id = then_seq_id;
        //         tracker.then_idx = then_idx;
        //         tracker.curr_idx += 1;
        //         return true;
        //     }
        // }
        false
    }

    fn emit_if_else(&mut self) -> bool {
        // if let Some(curr_loc) = self.instr_iter.curr_mut() {
        //     if let Some(tracker) = &mut self.emitting_instr {
        //         // This MUST be `self.app_wasm` so we're mutating what will be the instrumented application.
        //         let func = self
        //             .app_wasm
        //             .funcs
        //             .get_mut(curr_loc.wasm_func_id)
        //             .kind
        //             .unwrap_local_mut();
        //         let func_builder = func.builder_mut();
        //         let mut instr_builder = func_builder.instr_seq(tracker.curr_seq_id);

        //         let mut outer_seq_id = None;
        //         let mut outer_idx = None;
        //         let mut then_seq_id = None;
        //         let mut then_idx = None;
        //         let mut else_seq_id = None;
        //         let mut else_idx = None;

        //         instr_builder.block_at(tracker.curr_idx, None, |outer_block| {
        //             outer_seq_id = Some(outer_block.id());
        //             outer_idx = Some(0usize);
        //             outer_block.if_else(
        //                 None,
        //                 |then| {
        //                     then_seq_id = Some(then.id());
        //                     then_idx = Some(0usize);
        //                 },
        //                 |else_| {
        //                     else_seq_id = Some(else_.id());
        //                     else_idx = Some(0usize);
        //                 },
        //             );
        //         });
        //         // leave outer_block_idx as 0 to enable injection of condition!

        //         // Save the block information for future reference
        //         tracker.outer_seq_id = outer_seq_id;
        //         tracker.outer_idx = outer_idx;
        //         tracker.then_seq_id = then_seq_id;
        //         tracker.then_idx = then_idx;
        //         tracker.else_seq_id = else_seq_id;
        //         tracker.else_idx = else_idx;
        //         tracker.curr_idx += 1;
        //         return true;
        //     }
        // }
        false
    }

    /// Will configure the emitter to emit subsequent expression as the condition of an if or if/else stmt
    /// Then emits the passed condition at that location.
    fn emit_condition(&mut self) -> bool {
        if let Some(tracker) = &mut self.emitting_instr {
            if let Some(outer_seq_id) = &tracker.outer_seq_id {
                if let Some(outer_idx) = &tracker.outer_idx {
                    tracker.curr_seq_id = *outer_seq_id;
                    tracker.curr_idx = *outer_idx;
                }
            }
        }
        false
    }

    /// Will configure the emitter to emit subsequent statements into the consequent body of an if or if/else stmt
    fn emit_consequent(&mut self) -> bool {
        if let Some(tracker) = &mut self.emitting_instr {
            if let Some(then_seq_id) = &tracker.then_seq_id {
                if let Some(then_idx) = &tracker.then_idx {
                    tracker.curr_seq_id = *then_seq_id;
                    tracker.curr_idx = *then_idx;
                }
            }
            return true;
        }
        false
    }

    /// Will configure the emitter to emit subsequent statements into the alternate body of an if/else stmt
    fn emit_alternate(&mut self) -> bool {
        if let Some(tracker) = &mut self.emitting_instr {
            if let Some(else_seq_id) = &tracker.else_seq_id {
                if let Some(else_idx) = &tracker.else_idx {
                    tracker.curr_seq_id = *else_seq_id;
                    tracker.curr_idx = *else_idx;
                    return true;
                }
            }
        }
        false
    }

    /// Will configure the emitter to emit subsequent statements in the outer block of some branching logic
    fn finish_branch(&mut self) -> bool {
        if let Some(tracker) = &mut self.emitting_instr {
            tracker.curr_seq_id = tracker.main_seq_id;
            tracker.curr_idx = tracker.main_idx;

            tracker.outer_seq_id = None;
            tracker.outer_idx = None;
            tracker.then_seq_id = None;
            tracker.then_idx = None;
            tracker.else_seq_id = None;
            tracker.else_idx = None;
            return true;
        }
        true
    }

    // Reliant on walrus
    fn emit_global_stmts(&mut self, stmts: &mut Vec<Statement>) -> Result<bool, Box<WhammError>> {
        // // NOTE: This should be done in the Module entrypoint
        // //       https://docs.rs/walrus/latest/walrus/struct.Module.html

        // if let Some(start_fid) = self.app_wasm.start {
        //     if let FunctionKind::Local(local_func) = &self.app_wasm.funcs.get(start_fid).kind {
        //         self.emitting_instr = Some(EmittingInstrTracker {
        //             orig_instr_idx: 0usize,
        //             curr_seq_id: local_func.entry_block(),
        //             curr_idx: 0usize,
        //             main_seq_id: local_func.entry_block(),
        //             main_idx: 0usize,
        //             outer_seq_id: None,
        //             outer_idx: None,
        //             then_seq_id: None,
        //             then_idx: None,
        //             else_seq_id: None,
        //             else_idx: None,
        //         })
        //     }
        // } else {
        //     for stmt in stmts.iter_mut() {
        //         match stmt {
        //             Statement::Decl { .. } => {
        //                 // This is fine
        //             }
        //             _ => {
        //                 // This is NOT fine...error!
        //                 // Cannot emit this at the moment since there's no entrypoint for our module to emit initialization instructions into
        //                 return Err(Box::new(ErrorGen::get_unexpected_error(
        //                     true,
        //                     Some(
        //                         "This module has no configured entrypoint, \
        //             unable to emit a `script` with global state"
        //                             .to_string(),
        //                     ),
        //                     None,
        //                 )));
        //             }
        //         }
        //     }
        //     return Ok(true);
        // }

        // for stmt in stmts.iter_mut() {
        //     // iterate over statements and emit them
        //     self.emit_stmt(stmt)?;
        // }
        Ok(true)
    }

    fn emit_body(&mut self, body: &mut Vec<Statement>) -> Result<bool, Box<WhammError>> {
        for stmt in body.iter_mut() {
            self.emit_stmt(stmt)?;
        }
        Ok(true)
    }

    fn has_alt_call(&mut self) -> bool {
        // // check if we should inject an alternate call!
        // // At this point the body has been visited, so "new_target_fn_name" would be defined
        // let rec_id = self
        //     .table
        //     .lookup(&"new_target_fn_name".to_string())
        //     .copied();

        // if rec_id.is_none() {
        //     info!("`new_target_fn_name` not configured for this probe.");
        //     return false;
        // } else {
        //     let (name, func_call_id) = match rec_id {
        //         Some(r_id) => {
        //             let rec = self.table.get_record_mut(&r_id);
        //             if let Some(Record::Var {
        //                 value: Some(Value::Str { val, .. }),
        //                 ..
        //             }) = rec
        //             {
        //                 (val.clone(), self.app_wasm.funcs.by_name(val))
        //             } else {
        //                 ("".to_string(), None)
        //             }
        //         }
        //         None => ("".to_string(), None),
        //     };
        //     if func_call_id.is_none() {
        //         info!(
        //             "Could not find function in app Wasm specified by `new_target_fn_name`: {}",
        //             name
        //         );
        //         return false;
        //     }
        //     if let Some(curr_loc) = self.instr_iter.curr_mut() {
        //         curr_loc.instr_alt_call = func_call_id;
        //     } else {
        //         info!("The instruction iterator has not been initialized, we've hit a bug!");
        //         return false;
        //     }
        // }
        true
    }

    fn emit_alt_call(&mut self) -> Result<bool, Box<WhammError>> {
        // if let Some(curr_loc) = self.instr_iter.curr_mut() {
        //     if let Some(tracker) = &mut self.emitting_instr {
        //         if let Some(alt_fn_id) = curr_loc.instr_alt_call {
        //             // we need to inject an alternate call to the specified fn name!
        //             let func = self
        //                 .app_wasm
        //                 .funcs
        //                 .get_mut(curr_loc.wasm_func_id)
        //                 .kind
        //                 .unwrap_local_mut();
        //             let func_builder = func.builder_mut();
        //             let mut instr_builder = func_builder.instr_seq(tracker.curr_seq_id);

        //             // Hack to have emit_params target this new call site!
        //             tracker.orig_instr_idx = tracker.curr_idx;

        //             // inject call
        //             instr_builder.instr_at(tracker.curr_idx, walrus::ir::Call { func: alt_fn_id });
        //             tracker.curr_idx += 1;
        //         } else {
        //             return Err(Box::new(ErrorGen::get_unexpected_error(
        //                 true,
        //                 Some(format!(
        //                     "{UNEXPECTED_ERR_MSG} \
        //             Could not inject alternate call to function, something went wrong..."
        //                 )),
        //                 None,
        //             )));
        //         }
        //     }
        // }
        Ok(true)
    }

    fn emit_stmt(&mut self, stmt: &mut Statement) -> Result<bool, Box<WhammError>> {
        match stmt {
            Statement::Decl { .. } => self.emit_decl_stmt(stmt),
            Statement::Assign { .. } => self.emit_assign_stmt(stmt),
            Statement::Expr { expr, .. } => self.emit_expr(expr),
            Statement::Return { .. } => unimplemented!(),
            Statement::If {
                // cond, conseq, alt, .. -- for eventual implimentation
                ..
            } => {
                unimplemented!()
            }
        }
    }

    fn dump_to_file(&mut self, output_wasm_path: String) -> Result<bool, Box<WhammError>> {
        // clone for now

        let res = self.app_wasm.clone().encode();
        match res {
            Ok(module) => {
                let mut file = std::fs::File::create(&output_wasm_path).unwrap();
                use std::io::Write;
                let bytes = module.finish();
                file.write_all(&bytes).unwrap();

                Ok(true)
            }
            Err(err) => Err(Box::new(ErrorGen::get_unexpected_error(
                true,
                Some(format!(
                    "{UNEXPECTED_ERR_MSG} \
                Failed to dump instrumented wasm to {} from error: {}",
                    &output_wasm_path, err
                )),
                None,
            ))),
        }
    }
}

// =====================
// ==== WasiEmitter ====
// =====================
// unimplemented

// =======================
// ==== VirgilEmitter ====
// =======================
// unimplemented
