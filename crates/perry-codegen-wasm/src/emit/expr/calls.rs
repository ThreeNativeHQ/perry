//! Generic function calls (Expr::Call) including method-call sugar.
//!
//! Mechanically extracted from emit/expr.rs (#1102 follow-up split).
//! See `mod.rs` for the dispatcher that calls each `try_emit_expr_*`.

use super::*;

impl<'a> FuncEmitCtx<'a> {
    pub(super) fn try_emit_expr_calls(&mut self, func: &mut Function, expr: &Expr) -> bool {
        match expr {
            Expr::Call { callee, args, .. } => {
                // Check for method call patterns: obj.method(args)
                if let Expr::PropertyGet {
                    object, property, ..
                } = callee.as_ref()
                {
                    // Namespace-import member call (`import * as W from "./mod";
                    // W.fn(args)`): resolve to a DIRECT wasm call of the source
                    // module's function — the same lowering `fn(args)` gets via
                    // a named import. Without this the callee fell through to
                    // the class-dispatch fallback with an undefined receiver
                    // and silently returned undefined (never executing fn).
                    if let Expr::ExternFuncRef { name, .. } = object.as_ref() {
                        let key = (
                            self.emitter.current_mod_idx,
                            format!("{}.{}", name, property),
                        );
                        if let Some(&idx) =
                            self.emitter.imported_ns_funcs.get(&key).copied().as_ref()
                        {
                            for arg in args {
                                self.emit_expr(func, arg);
                            }
                            // Pad-up / drop-excess — see the FuncRef arm below (#183).
                            if let Some(&expected) = self.emitter.func_param_counts.get(&idx) {
                                for _ in args.len()..expected {
                                    func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
                                }
                                for _ in expected..args.len() {
                                    func.instruction(&Instruction::Drop);
                                }
                            }
                            func.instruction(&Instruction::Call(idx));
                            if self.emitter.void_funcs.contains(&idx) {
                                func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
                            }
                            return true;
                        }
                    }
                    // console.log/warn/error
                    if let Expr::GlobalGet(_) = object.as_ref() {
                        match property.as_str() {
                            "log" => {
                                self.emit_console(func, "log", args);
                                return true;
                            }
                            "warn" => {
                                self.emit_console(func, "warn", args);
                                return true;
                            }
                            "error" => {
                                self.emit_console(func, "error", args);
                                return true;
                            }
                            _ => {}
                        }
                    }
                    // String/Array method calls: expr.method(args)
                    if self.emit_method_call(func, object, property, args) {
                        return true;
                    }

                    // Fallback: class/UI method dispatch via mem_call with stack-based buffer.
                    {
                        let method_name = property.as_str();
                        // Slot 0 = object, slots 1..N = args
                        self.emit_frame_begin(func, (args.len() + 1) as u32);
                        self.emit_store_arg(func, 0, object);
                        for (i, arg) in args.iter().enumerate() {
                            self.emit_store_arg(func, (i + 1) as u32, arg);
                        }
                        self.emit_memcall(func, method_name, (args.len() + 1) as u32);
                        return true;
                    }
                }

                // Built-in timer globals (setTimeout/setInterval/clearTimeout/
                // clearInterval). These resolve to `Expr::ExternFuncRef` with no
                // entry in `func_name_map`, so without this intercept they fall
                // through to the ExternFuncRef arm below, which drops the args and
                // pushes `undefined` — the timer is never scheduled and the
                // callback never fires (#1323). Route them through the mem_call
                // bridge to `__memDispatch.set_timeout` / `set_interval` /
                // `clear_timeout` / `clear_interval` in wasm_runtime.js, mirroring
                // how fetch/closure_call dispatch. The set_* bridges take
                // (closure, delay) and return the timer id; the clear_* bridges
                // take (id). Trailing setTimeout(fn, delay, ...args) extras are
                // dropped (Node passes them to the callback — out of scope here).
                if let Expr::ExternFuncRef { name, .. } = callee.as_ref() {
                    let timer = match name.as_str() {
                        "setTimeout" => Some(("set_timeout", 2u32)),
                        "setInterval" => Some(("set_interval", 2u32)),
                        "clearTimeout" => Some(("clear_timeout", 1u32)),
                        "clearInterval" => Some(("clear_interval", 1u32)),
                        _ => None,
                    };
                    if let Some((bridge, arity)) = timer {
                        self.emit_frame_begin(func, arity);
                        for slot in 0..arity {
                            match args.get(slot as usize) {
                                Some(arg) => self.emit_store_arg(func, slot, arg),
                                None => {
                                    // Pad missing arg (e.g. setTimeout(fn)) with undefined.
                                    self.emit_slot_addr(func, slot);
                                    func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
                                    func.instruction(&Instruction::I64Store(
                                        wasm_encoder::MemArg {
                                            offset: 0,
                                            align: 3,
                                            memory_index: 0,
                                        },
                                    ));
                                }
                            }
                        }
                        // emit_memcall leaves the i64 result (timer id for set_*,
                        // NaN-boxed undefined for clear_*) on the stack, satisfying
                        // the call expression's value requirement.
                        self.emit_memcall(func, bridge, arity);
                        return true;
                    }
                }

                // Evaluate arguments first
                for arg in args {
                    self.emit_expr(func, arg);
                }
                // Call the function — resolve target and pad missing optional args with undefined
                match callee.as_ref() {
                    Expr::FuncRef(id) => {
                        if let Some(&idx) = self.emitter.func_map.get(id) {
                            // Reconcile source arg count with callee arity. JS semantics
                            // allow a call to pass any number of args, but WASM `call`
                            // consumes exactly the declared param count. Pad up with
                            // `undefined` for missing optional args and drop excess
                            // evaluated args from the top of the operand stack, which
                            // would otherwise accumulate past the call and trip the
                            // validator at the enclosing `end` (#183).
                            if let Some(&expected) = self.emitter.func_param_counts.get(&idx) {
                                for _ in args.len()..expected {
                                    func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
                                }
                                for _ in expected..args.len() {
                                    func.instruction(&Instruction::Drop);
                                }
                            }
                            func.instruction(&Instruction::Call(idx));
                            // Void functions don't push a return value; push undefined
                            // so the caller always has a value on the stack.
                            if self.emitter.void_funcs.contains(&idx) {
                                func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
                            }
                        } else {
                            // Unknown function — push undefined
                            for _ in args {
                                func.instruction(&Instruction::Drop);
                            }
                            func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
                        }
                    }
                    Expr::ExternFuncRef {
                        name, return_type, ..
                    } => {
                        // Cross-module or FFI function call. The consumer's
                        // own import table wins (resolved through re-export
                        // chains); the whole-program name map is only a
                        // fallback, since its bare-name keys collide across
                        // modules. See FuncRef arm above for why both pad-up
                        // and drop-excess are required (#183).
                        let consumer_key = (self.emitter.current_mod_idx, name.clone());
                        if let Some(&idx) = self
                            .emitter
                            .imported_func_indices
                            .get(&consumer_key)
                            .or_else(|| self.emitter.func_name_map.get(name))
                        {
                            if let Some(&expected) = self.emitter.func_param_counts.get(&idx) {
                                for _ in args.len()..expected {
                                    func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
                                }
                                for _ in expected..args.len() {
                                    func.instruction(&Instruction::Drop);
                                }
                            }
                            func.instruction(&Instruction::Call(idx));
                            // Void functions don't push a return value, but call
                            // expressions always need a value on the stack. Push undefined.
                            if matches!(return_type, perry_hir::types::Type::Void)
                                || self.emitter.void_funcs.contains(&idx)
                            {
                                func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
                            }
                        } else {
                            for _ in args {
                                func.instruction(&Instruction::Drop);
                            }
                            if name == "js_for_of_next" && args.len() == 1 {
                                // The sync for-of advance: `iter.next()` through method dispatch.
                                self.emit_frame_begin(func, 1);
                                self.emit_store_arg(func, 0, &args[0]);
                                self.emit_memcall(func, "for_of_next", 1);
                            } else if self
                                .emitter
                                .imported_var_globals
                                .contains_key(&(self.emitter.current_mod_idx, name.clone()))
                            {
                                // An imported `const` function (three/tsl's `vec3`): call the value.
                                self.emit_dynamic_call(func, callee, args);
                            } else if name.starts_with("js_")
                                && crate::WASM_RUNTIME_JS.contains(&format!("\n  {name}: ("))
                            {
                                // A perry-runtime helper with a same-named bridge in wasm_runtime.js.
                                self.emit_frame_begin(func, args.len() as u32);
                                for (i, arg) in args.iter().enumerate() {
                                    self.emit_store_arg(func, i as u32, arg);
                                }
                                self.emit_memcall(func, name, args.len() as u32);
                            } else {
                                *self.emitter.unsupported.borrow_mut()
                                    .entry(format!("UnresolvedExtern({name})")).or_default() += 1;
                                func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
                            }
                        }
                    }
                    _ => {
                        // Dynamic call via closure bridge
                        // Stack has: [arg0, arg1, ..., argN] but callee not yet pushed
                        // We need callee first for closure_call. Restructure:
                        // Drop the args we already pushed, re-emit callee first, then args
                        for _ in args {
                            func.instruction(&Instruction::Drop);
                        }
                        self.emit_dynamic_call(func, callee, args);
                    }
                }
            }

            _ => return false,
        }
        true
    }

    /// Call a closure value through the mem_call bridge (Firefox NaN safety); four or more
    /// arguments travel as one array (closure_call_spread).
    pub(super) fn emit_dynamic_call(&mut self, func: &mut Function, callee: &Expr, args: &[Expr]) {
        if args.len() > 3 {
            self.emit_frame_begin(func, 2);
            self.emit_store_arg(func, 0, callee);
            self.emit_store_arg(func, 1, &Expr::Array(args.to_vec()));
            self.emit_memcall(func, "closure_call_spread", 2);
            return;
        }
        self.emit_frame_begin(func, (args.len() + 1) as u32);
        self.emit_store_arg(func, 0, callee);
        for (i, arg) in args.iter().enumerate() {
            self.emit_store_arg(func, (i + 1) as u32, arg);
        }
        let name = ["closure_call_0", "closure_call_1", "closure_call_2", "closure_call_3"][args.len()];
        self.emit_memcall(func, name, args.len() as u32 + 1);
    }

    /// Store `args` in a fresh frame and call one runtime bridge.
    fn emit_bridge(&mut self, func: &mut Function, bridge: &str, args: &[&Box<Expr>]) {
        self.emit_frame_begin(func, args.len() as u32);
        for (i, arg) in args.iter().enumerate() {
            self.emit_store_arg(func, i as u32, arg);
        }
        self.emit_memcall(func, bridge, args.len() as u32);
    }

    /// console.log/warn/error: one argument goes straight through; any other count prints one
    /// space-joined line from an args array, as the host console does (PRD-582).
    pub(super) fn emit_console(&mut self, func: &mut Function, level: &str, args: &[Expr]) {
        self.emit_frame_begin(func, 1);
        if let [arg] = args {
            self.emit_store_arg(func, 0, arg);
            self.emit_memcall_void(func, &format!("console_{level}"), 1);
        } else {
            self.emit_store_arg(func, 0, &Expr::Array(args.to_vec()));
            self.emit_memcall_void(func, &format!("console_{level}_multi"), 1);
        }
    }

    /// The async-step state machine (`transform_async_to_generator` + `transform_generators`):
    /// the body runs in Wasm step closures; the runtime only chains promises (PRD-582).
    pub(super) fn try_emit_expr_async_step(&mut self, func: &mut Function, expr: &Expr) -> bool {
        match expr {
            // A private access on a legal receiver evaluates to the receiver unchanged.
            // ponytail: no brand check, so an illegal receiver reads a field instead of throwing;
            // add the check when a program relies on the TypeError.
            Expr::PrivateGuard { object, .. } => self.emit_expr(func, object),
            Expr::LinkGeneratorPrototype { obj, .. } => self.emit_expr(func, obj),
            // The emitter never patches Array.prototype[Symbol.iterator].
            Expr::ArrayIterationPatched => {
                func.instruction(&Instruction::I64Const(TAG_FALSE as i64));
            }
            Expr::MapEntryKeyAt { map, idx } => self.emit_bridge(func, "map_entry_key_at", &[map, idx]),
            Expr::MapEntryValueAt { map, idx } => self.emit_bridge(func, "map_entry_value_at", &[map, idx]),
            Expr::SetValueAt { set, idx } => self.emit_bridge(func, "set_value_at", &[set, idx]),
            Expr::ArrayFlatMap { array, callback } => self.emit_bridge(func, "array_flat_map", &[array, callback]),
            Expr::ObjectHasOwn(object, key) => self.emit_bridge(func, "js_object_has_own", &[object, key]),
            Expr::JsonStringifyFull(value, replacer, space) => {
                self.emit_bridge(func, "json_stringify_full", &[value, replacer, space])
            }
            Expr::ArraySome { array, callback } | Expr::ArrayEvery { array, callback } => {
                let bridge = if matches!(expr, Expr::ArraySome { .. }) { "array_some" } else { "array_every" };
                self.emit_frame_begin(func, 2);
                self.emit_store_arg(func, 0, array);
                self.emit_store_arg(func, 1, callback);
                self.emit_memcall_i32(func, bridge, 2);
                func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::I64)));
                func.instruction(&Instruction::I64Const(TAG_TRUE as i64));
                func.instruction(&Instruction::Else);
                func.instruction(&Instruction::I64Const(TAG_FALSE as i64));
                func.instruction(&Instruction::End);
            }
            Expr::RegisterClassComputedMethod { class_name, key_expr, method_name, is_static, .. } => {
                let methods = if *is_static { &self.emitter.class_static_map } else { &self.emitter.class_method_map };
                let table_idx = methods
                    .get(class_name)
                    .and_then(|m| m.get(method_name))
                    .map(|f| self.emitter.func_to_table_idx.get(f).copied().unwrap_or(*f));
                let Some(table_idx) = table_idx else {
                    *self.emitter.unsupported.borrow_mut()
                        .entry("RegisterClassComputedMethod".into()).or_default() += 1;
                    func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
                    return true;
                };
                let class_id = self.emitter.string_map.get(class_name.as_str()).copied().unwrap_or(0);
                self.emit_frame_begin(func, 3);
                self.emit_store_const(func, 0, f64::from_bits((STRING_TAG << 48) | class_id as u64));
                self.emit_store_arg(func, 1, key_expr);
                self.emit_store_const(func, 2, table_idx as f64);
                self.emit_memcall_void(func, "class_set_method", 3);
                func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
            }
            Expr::SymbolFor(key) => self.emit_bridge(func, "symbol_for", &[key]),
            Expr::SymbolNew(description) => match description {
                Some(d) => self.emit_bridge(func, "symbol_new", &[d]),
                None => self.emit_bridge(func, "symbol_new", &[]),
            },
            Expr::GetIterator(value) => {
                self.emit_frame_begin(func, 1);
                self.emit_store_arg(func, 0, value);
                self.emit_memcall(func, "get_iterator", 1);
            }
            Expr::IterResultSet(value, done) => {
                self.emit_frame_begin(func, 2);
                self.emit_store_arg(func, 0, value);
                self.emit_store_const(func, 1, if *done { 1.0 } else { 0.0 });
                self.emit_memcall(func, "iter_result_set", 2);
            }
            Expr::IterResultGetValue => {
                self.emit_frame_begin(func, 0);
                self.emit_memcall(func, "iter_result_get_value", 0);
            }
            Expr::IterResultGetDone => {
                self.emit_frame_begin(func, 0);
                self.emit_memcall(func, "iter_result_get_done", 0);
            }
            Expr::CurrentStepClosure => {
                self.emit_frame_begin(func, 0);
                self.emit_memcall(func, "current_step_closure", 0);
            }
            Expr::AsyncFirstCall { step_closure } => {
                self.emit_frame_begin(func, 1);
                self.emit_store_arg(func, 0, step_closure);
                self.emit_memcall(func, "async_first_call", 1);
            }
            Expr::AsyncStepChain { value, step_closure } | Expr::AsyncStepDone { value, step_closure } => {
                let name = if matches!(expr, Expr::AsyncStepChain { .. }) { "async_step_chain" } else { "async_step_done" };
                self.emit_frame_begin(func, 2);
                self.emit_store_arg(func, 0, value);
                self.emit_store_arg(func, 1, step_closure);
                self.emit_memcall(func, name, 2);
            }
            _ => return false,
        }
        true
    }
}
