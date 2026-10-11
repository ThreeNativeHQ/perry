//! RegExp constructor/test and global builtins (parseInt/parseFloat/Number/isNaN/isFinite/BigInt).
//!
//! Mechanically extracted from emit/expr.rs (#1102 follow-up split).
//! See `mod.rs` for the dispatcher that calls each `try_emit_expr_*`.

use super::*;

impl<'a> FuncEmitCtx<'a> {
    pub(super) fn try_emit_expr_regex_globals(&mut self, func: &mut Function, expr: &Expr) -> bool {
        match expr {
            Expr::RegExp { pattern, flags } => {
                let pat_id = self
                    .emitter
                    .string_map
                    .get(pattern.as_str())
                    .copied()
                    .unwrap_or(0);
                let pat_bits = (STRING_TAG << 48) | (pat_id as u64);
                let flags_id = self
                    .emitter
                    .string_map
                    .get(flags.as_str())
                    .copied()
                    .unwrap_or(0);
                let flags_bits = (STRING_TAG << 48) | (flags_id as u64);
                self.emit_frame_begin(func, 2);
                self.emit_store_const(func, 0, f64::from_bits(pat_bits));
                self.emit_store_const(func, 1, f64::from_bits(flags_bits));
                self.emit_memcall(func, "regexp_new", 2);
            }
            Expr::RegExpTest { regex, string } => {
                self.emit_frame_begin(func, 2);
                self.emit_store_arg(func, 0, regex);
                self.emit_store_arg(func, 1, string);
                self.emit_memcall_i32(func, "regexp_test", 2);
                func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    ValType::I64,
                )));
                func.instruction(&Instruction::I64Const(TAG_TRUE as i64));
                func.instruction(&Instruction::Else);
                func.instruction(&Instruction::I64Const(TAG_FALSE as i64));
                func.instruction(&Instruction::End);
            }

            // --- Global builtins ---
            Expr::ParseInt { string, radix } => {
                self.emit_expr(func, string);
                let _ = radix; // TODO: radix support
                self.emit_frame_begin(func, 1);
                func.instruction(&Instruction::LocalSet(self.temp_local));
                self.emit_slot_addr(func, 0);
                func.instruction(&Instruction::LocalGet(self.temp_local));
                func.instruction(&Instruction::I64Store(wasm_encoder::MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                self.emit_memcall(func, "parse_int", 1);
            }
            Expr::ParseFloat(val) => {
                self.emit_frame_begin(func, 1);
                self.emit_store_arg(func, 0, val);
                self.emit_memcall(func, "parse_float", 1);
            }
            Expr::NumberCoerce(val) => {
                self.emit_frame_begin(func, 1);
                self.emit_store_arg(func, 0, val);
                self.emit_memcall(func, "number_coerce", 1);
            }
            Expr::IsNaN(val) => {
                self.emit_frame_begin(func, 1);
                self.emit_store_arg(func, 0, val);
                self.emit_memcall_i32(func, "is_nan", 1);
                func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    ValType::I64,
                )));
                func.instruction(&Instruction::I64Const(TAG_TRUE as i64));
                func.instruction(&Instruction::Else);
                func.instruction(&Instruction::I64Const(TAG_FALSE as i64));
                func.instruction(&Instruction::End);
            }
            Expr::IsUndefinedOrBareNan(val) => {
                // WASM fallback: delegate to is_nan (close enough for most cases)
                self.emit_frame_begin(func, 1);
                self.emit_store_arg(func, 0, val);
                self.emit_memcall_i32(func, "is_nan", 1);
                func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    ValType::I64,
                )));
                func.instruction(&Instruction::I64Const(TAG_TRUE as i64));
                func.instruction(&Instruction::Else);
                func.instruction(&Instruction::I64Const(TAG_FALSE as i64));
                func.instruction(&Instruction::End);
            }
            Expr::IsFinite(val) => {
                self.emit_frame_begin(func, 1);
                self.emit_store_arg(func, 0, val);
                self.emit_memcall_i32(func, "is_finite", 1);
                func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    ValType::I64,
                )));
                func.instruction(&Instruction::I64Const(TAG_TRUE as i64));
                func.instruction(&Instruction::Else);
                func.instruction(&Instruction::I64Const(TAG_FALSE as i64));
                func.instruction(&Instruction::End);
            }
            Expr::NumberIsFinite(val) => self.emit_number_predicate(func, val, 0),
            Expr::NumberIsInteger(val) => self.emit_number_predicate(func, val, 1),
            Expr::NumberIsSafeInteger(val) => self.emit_number_predicate(func, val, 2),
            Expr::BigIntCoerce(_) => {
                func.instruction(&Instruction::I64Const(TAG_UNDEFINED as i64));
            }

            _ => return false,
        }
        true
    }

    /// Number.isFinite (0), isInteger (1) and isSafeInteger (2) in Wasm, without coercion:
    /// an int32-tagged value passes all three, any other tag fails, and an f64 is tested.
    fn emit_number_predicate(&mut self, func: &mut Function, val: &Expr, level: u8) {
        let t = self.temp_local;
        let tag = |func: &mut Function| {
            func.instruction(&Instruction::LocalGet(t));
            func.instruction(&Instruction::I64Const(48));
            func.instruction(&Instruction::I64ShrU);
        };
        let num = |func: &mut Function| {
            func.instruction(&Instruction::LocalGet(t));
            func.instruction(&Instruction::F64ReinterpretI64);
        };
        let bool_result = |func: &mut Function| {
            func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::I64)));
            func.instruction(&Instruction::I64Const(TAG_TRUE as i64));
            func.instruction(&Instruction::Else);
            func.instruction(&Instruction::I64Const(TAG_FALSE as i64));
            func.instruction(&Instruction::End);
        };
        self.emit_expr(func, val);
        func.instruction(&Instruction::LocalSet(t));
        tag(func);
        func.instruction(&Instruction::I64Const(0x7FFE)); // INT32 tag
        func.instruction(&Instruction::I64Eq);
        func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::I64)));
        func.instruction(&Instruction::I64Const(TAG_TRUE as i64));
        func.instruction(&Instruction::Else);
        tag(func);
        func.instruction(&Instruction::I64Const(0x7FFC)); // 0x7FFC..=0x7FFF are non-number tags
        func.instruction(&Instruction::I64GeU);
        tag(func);
        func.instruction(&Instruction::I64Const(0x7FFF));
        func.instruction(&Instruction::I64LeU);
        func.instruction(&Instruction::I32And);
        func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::I64)));
        func.instruction(&Instruction::I64Const(TAG_FALSE as i64));
        func.instruction(&Instruction::Else);
        // finite: x - x == 0 (NaN and ±Infinity give NaN)
        num(func);
        num(func);
        func.instruction(&Instruction::F64Sub);
        func.instruction(&Instruction::F64Const(0.0.into()));
        func.instruction(&Instruction::F64Eq);
        if level >= 1 {
            num(func);
            func.instruction(&Instruction::F64Trunc);
            num(func);
            func.instruction(&Instruction::F64Eq);
            func.instruction(&Instruction::I32And);
        }
        if level >= 2 {
            num(func);
            func.instruction(&Instruction::F64Abs);
            func.instruction(&Instruction::F64Const(9007199254740991.0.into()));
            func.instruction(&Instruction::F64Le);
            func.instruction(&Instruction::I32And);
        }
        bool_result(func);
        func.instruction(&Instruction::End);
        func.instruction(&Instruction::End);
    }
}
