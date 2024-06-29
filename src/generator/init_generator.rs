// =======================
// ==== CodeGenerator ====
// =======================

use crate::common::error::ErrorGen;
use crate::emitter::Emitter;
use crate::parser::rules::{Event, Package, Probe, Provider};
use crate::parser::types::{
    BinOp, Block, DataType, Expr, Global, ProvidedFunction, ProvidedGlobal, Script, Statement,
    UnOp, Value, Whamm, WhammVisitor,
};
use log::{trace, warn};
use std::collections::HashMap;

/// Serves as the first phase of instrumenting a module by setting up
/// the groundwork.
///
/// The code generator traverses the AST and calls the passed emitter to
/// emit some compiler-provided functions and user-defined globals.
/// This process should ideally be generic, made to perform a specific
/// instrumentation technique by the Emitter field.
pub struct InitGenerator<'a> {
    pub emitter: Box<&'a mut dyn Emitter>,
    pub context_name: String,
    pub err: &'a mut ErrorGen,
}
impl InitGenerator<'_> {
    pub fn run(&mut self, whamm: &Whamm) -> bool {
        // Reset the symbol table in the emitter just in case
        self.emitter.reset_children();
        // Generate globals and fns defined by `whamm` (this should modify the app_wasm)
        self.visit_whamm(whamm)
    }

    // Private helper functions
    fn visit_globals(&mut self, globals: &HashMap<String, Global>) -> bool {
        let mut is_success = true;
        for (name, global) in globals.iter() {
            // do not inject globals into Wasm that are used/defined by the compiler
            if !&global.is_comp_provided {
                match self
                    .emitter
                    .emit_global(name.clone(), global.ty.clone(), &global.value)
                {
                    Err(e) => self.err.add_error(*e),
                    Ok(res) => is_success &= res,
                }
            }
        }

        is_success
    }
    fn visit_provided_globals(&mut self, globals: &HashMap<String, ProvidedGlobal>) -> bool {
        let mut is_success = true;
        for (name, ProvidedGlobal { global, .. }) in globals.iter() {
            // do not inject globals into Wasm that are used/defined by the compiler
            if !global.is_comp_provided {
                match self
                    .emitter
                    .emit_global(name.clone(), global.ty.clone(), &global.value)
                {
                    Err(e) => self.err.add_error(*e),
                    Ok(res) => is_success &= res,
                }
            }
        }

        is_success
    }
}
impl WhammVisitor<'_, bool> for InitGenerator<'_> {
    fn visit_whamm(&mut self, whamm: &Whamm) -> bool {
        trace!("Entering: CodeGenerator::visit_whamm");
        self.context_name = "whamm".to_string();
        let mut is_success = true;

        // visit fns
        whamm
            .fns
            .iter()
            .for_each(|ProvidedFunction { function, .. }| {
                is_success &= self.visit_fn(function);
            });
        // inject globals
        is_success &= self.visit_provided_globals(&whamm.globals);
        // visit scripts
        whamm.scripts.iter().for_each(|script| {
            is_success &= self.visit_script(script);
        });

        trace!("Exiting: CodeGenerator::visit_whamm");
        // Remove from `context_name`
        self.context_name = "".to_string();
        is_success
    }

    fn visit_script(&mut self, script: &Script) -> bool {
        trace!("Entering: CodeGenerator::visit_script");
        if let Err(e) = self.emitter.enter_scope() {
            self.err.add_error(*e)
        }
        self.context_name += &format!(":{}", script.name.clone());
        let mut is_success = true;

        // visit fns
        script.fns.iter().for_each(|f| {
            is_success &= self.visit_fn(f);
        });
        // inject globals
        is_success &= self.visit_globals(&script.globals);
        // visit providers
        script.providers.iter().for_each(|(_name, provider)| {
            is_success &= self.visit_provider(provider);
        });

        trace!("Exiting: CodeGenerator::visit_script");
        if let Err(e) = self.emitter.exit_scope() {
            self.err.add_error(*e)
        }
        // Remove from `context_name`
        self.context_name = self.context_name[..self.context_name.rfind(':').unwrap()].to_string();
        is_success
    }

    fn visit_provider(&mut self, provider: &Box<dyn Provider>) -> bool {
        trace!("Entering: CodeGenerator::visit_provider");
        if let Err(e) = self.emitter.enter_scope() {
            self.err.add_error(*e)
        }
        self.context_name += &format!(":{}", provider.name());
        let mut is_success = true;

        // visit fns
        provider
            .get_provided_fns()
            .iter()
            .for_each(|ProvidedFunction { function, .. }| {
                is_success &= self.visit_fn(function);
            });
        // inject globals
        is_success &= self.visit_provided_globals(provider.get_provided_globals());
        // visit the packages
        provider.packages().for_each(|package| {
            is_success &= self.visit_package(package);
        });

        trace!("Exiting: CodeGenerator::visit_provider");
        if let Err(e) = self.emitter.exit_scope() {
            self.err.add_error(*e)
        }
        // Remove this package from `context_name`
        self.context_name = self.context_name[..self.context_name.rfind(':').unwrap()].to_string();
        is_success
    }

    fn visit_package(&mut self, package: &dyn Package) -> bool {
        trace!("Entering: CodeGenerator::visit_package");
        if let Err(e) = self.emitter.enter_scope() {
            self.err.add_error(*e)
        }
        let mut is_success = true;
        self.context_name += &format!(":{}", package.name());

        // visit fns
        package
            .get_provided_fns()
            .iter()
            .for_each(|ProvidedFunction { function, .. }| {
                is_success &= self.visit_fn(function);
            });
        // inject globals
        is_success &= self.visit_provided_globals(package.get_provided_globals());
        // visit the events
        package.events().for_each(|event| {
            is_success &= self.visit_event(event);
        });

        trace!("Exiting: CodeGenerator::visit_package");
        if let Err(e) = self.emitter.exit_scope() {
            self.err.add_error(*e)
        }
        // Remove this package from `context_name`
        self.context_name = self.context_name[..self.context_name.rfind(':').unwrap()].to_string();
        is_success
    }

    fn visit_event(&mut self, event: &dyn Event) -> bool {
        trace!("Entering: CodeGenerator::visit_event");
        if let Err(e) = self.emitter.enter_scope() {
            self.err.add_error(*e)
        }
        // let mut is_success = self.emitter.emit_event(event);
        self.context_name += &format!(":{}", event.name());
        let mut is_success = true;

        // visit fns
        event
            .get_provided_fns()
            .iter()
            .for_each(|ProvidedFunction { function, .. }| {
                is_success &= self.visit_fn(function);
            });
        // inject globals
        is_success &= self.visit_provided_globals(&event.get_provided_globals());

        // 1. visit the BEFORE probes
        if let Some(probes) = event.probes().get(&"before".to_string()) {
            probes.iter().for_each(|probe| {
                is_success &= self.visit_probe(probe);
            });
        }
        // 2. visit the ALT probes
        if let Some(probes) = event.probes().get(&"alt".to_string()) {
            // only will emit one alt probe!
            // The last alt probe in the list will be emitted.
            if probes.len() > 1 {
                warn!("Detected multiple `alt` probes, will only emit the last one and ignore the rest!")
            }
            if let Some(probe) = probes.last() {
                is_success &= self.visit_probe(probe);
            }
        }
        // 3. visit the AFTER probes
        if let Some(probes) = event.probes().get(&"after".to_string()) {
            probes.iter().for_each(|probe| {
                is_success &= self.visit_probe(probe);
            });
        }

        trace!("Exiting: CodeGenerator::visit_event");
        if let Err(e) = self.emitter.exit_scope() {
            self.err.add_error(*e)
        }
        // Remove this event from `context_name`
        self.context_name = self.context_name[..self.context_name.rfind(':').unwrap()].to_string();
        is_success
    }

    fn visit_probe(&mut self, probe: &Box<dyn Probe>) -> bool {
        trace!("Entering: CodeGenerator::visit_probe");
        if let Err(e) = self.emitter.enter_scope() {
            self.err.add_error(*e)
        }
        // let mut is_success = self.emitter.emit_probe(probe);
        self.context_name += &format!(":{}", probe.mode_name());
        let mut is_success = true;

        // visit fns
        probe
            .get_mode_provided_fns()
            .iter()
            .for_each(|ProvidedFunction { function, .. }| {
                is_success &= self.visit_fn(function);
            });
        // inject globals
        is_success &= self.visit_provided_globals(&probe.get_mode_provided_globals());

        trace!("Exiting: CodeGenerator::visit_probe");
        if let Err(e) = self.emitter.exit_scope() {
            self.err.add_error(*e)
        }
        // Remove this probe from `context_name`
        self.context_name = self.context_name[..self.context_name.rfind(':').unwrap()].to_string();
        is_success
    }

    fn visit_fn(&mut self, f: &crate::parser::types::Fn) -> bool {
        trace!("Entering: CodeGenerator::visit_fn");
        if let Err(e) = self.emitter.enter_scope() {
            self.err.add_error(*e)
        }
        let mut is_success = true;
        if f.is_comp_provided {
            match self.emitter.emit_fn(&self.context_name, f) {
                Err(e) => self.err.add_error(*e),
                Ok(res) => is_success = res,
            }
        }
        trace!("Exiting: CodeGenerator::visit_fn");
        if let Err(e) = self.emitter.exit_scope() {
            self.err.add_error(*e)
        }
        is_success
    }

    fn visit_formal_param(&mut self, _param: &(Expr, DataType)) -> bool {
        // never called
        unreachable!();
        // trace!("Entering: CodeGenerator::visit_formal_param");
        // let is_success = self.emitter.emit_formal_param(param);
        // trace!("Exiting: CodeGenerator::visit_formal_param");
        // is_success
    }

    fn visit_block(&mut self, _block: &Block) -> bool {
        unreachable!();
    }

    fn visit_stmt(&mut self, _stmt: &Statement) -> bool {
        // never called
        unreachable!();
        // trace!("Entering: CodeGenerator::visit_stmt");
        // let is_success = self.emitter.emit_stmt(stmt);
        // trace!("Exiting: CodeGenerator::visit_stmt");
        // is_success
    }

    fn visit_expr(&mut self, _expr: &Expr) -> bool {
        // never called
        unreachable!();
        // trace!("Entering: CodeGenerator::visit_expr");
        // let is_success = self.emitter.emit_expr(expr);
        // trace!("Exiting: CodeGenerator::visit_expr");
        // is_success
    }

    fn visit_unop(&mut self, _unop: &UnOp) -> bool {
        // never called
        unreachable!();
    }

    fn visit_binop(&mut self, _binop: &BinOp) -> bool {
        // never called
        unreachable!();
        // trace!("Entering: CodeGenerator::visit_binop");
        // let is_success = self.emitter.emit_binop(binop);
        // trace!("Exiting: CodeGenerator::visit_binop");
        // is_success
    }

    fn visit_datatype(&mut self, _datatype: &DataType) -> bool {
        // never called
        unreachable!();
        // trace!("Entering: CodeGenerator::visit_datatype");
        // let is_success = self.emitter.emit_datatype(datatype);
        // trace!("Exiting: CodeGenerator::visit_datatype");
        // is_success
    }

    fn visit_value(&mut self, _val: &Value) -> bool {
        // never called
        unreachable!();
        // trace!("Entering: CodeGenerator::visit_value");
        // let is_success = self.emitter.emit_value(val);
        // trace!("Exiting: CodeGenerator::visit_value");
        // is_success
    }
}
