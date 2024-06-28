pub mod rewriting;

#[cfg(test)]
pub mod tests;

use crate::common::error::WhammError;
use crate::parser::types::{DataType, Expr, Fn, Statement, Value};

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
    fn curr_instr_is_of_type(&mut self, instr_names: &[String]) -> bool;
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
