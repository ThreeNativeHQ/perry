//! Arbitrary-arity all-`f64` call trampoline.
//!
//! Perry-generated methods and constructors have the C signature
//! `double(double this, double arg0, …, double argN)`. The dynamic vtable
//! dispatch (`object::class_registry::dispatch::call_vtable_method`) must invoke
//! such a function for an arity only known at runtime — and a synthesized
//! capture-stashing constructor can have 130+ params (one per captured
//! enclosing local in a giant minified bundle, e.g. Next.js app-route-turbo's
//! route-module class `rJ`). Hand-writing a `match`-arm-per-arity dispatch caps
//! out (the pre-#5437 64-arm cap silently transmuted a 135-param ctor to a
//! 64-arg signature in release builds, so every param past the 64th read
//! register/stack garbage — a captured function arrived non-callable and the
//! ctor threw "value is not a function").
//!
//! Because EVERY argument is an `f64`, the platform C ABI is fully determined:
//! the first 8 floating-point args go in FP argument registers and the rest are
//! spilled to a 16-byte-aligned stack area. This module implements that call
//! directly with inline assembly for the two hosted architectures (aarch64 +
//! x86-64); other targets fall back to a fixed-arity dispatch good to 16 args
//! (no Perry target other than the two asm ones exercises high-arity dynamic
//! ctor dispatch today).

/// Call `func_ptr` (a `extern "C" double(double, …)` with `args.len()` f64
/// params) passing every element of `args` as an f64 argument. Returns the f64
/// result.
///
/// # Safety
/// `func_ptr` must be a valid code pointer to a function whose C signature is
/// `double(double × args.len())`. All Perry method/ctor params are `f64`.
#[inline]
pub(crate) unsafe fn call_all_f64(func_ptr: usize, args: &[f64]) -> f64 {
    #[cfg(target_arch = "aarch64")]
    {
        call_all_f64_aarch64(func_ptr, args)
    }
    // NOTE: gated to NON-Windows x86-64. The asm below is the SysV ABI (FP args
    // in xmm0..xmm7, no shadow space). The Windows x64 ABI passes FP args in
    // xmm0..xmm3 and requires a 32-byte shadow space, so the SysV asm would
    // mis-pass 5+ args. Win64 falls through to the portable fallback instead.
    // Small arities retain the faster compiled fixed-arity dispatch; wider
    // calls use the frame-pointer asm trampoline with full unwind information.
    #[cfg(all(target_arch = "x86_64", not(target_os = "windows")))]
    {
        if args.len() <= 16 {
            call_all_f64_fallback(func_ptr, args)
        } else {
            call_all_f64_x86_64(func_ptr, args)
        }
    }
    #[cfg(not(any(
        target_arch = "aarch64",
        all(target_arch = "x86_64", not(target_os = "windows"))
    )))]
    {
        call_all_f64_fallback(func_ptr, args)
    }
}

/// AAPCS64: the first 8 f64 args go in v0–v7; args 9+ are spilled to the stack
/// in order, each occupying 8 bytes, with the stack 16-byte aligned at the call.
#[cfg(target_arch = "aarch64")]
#[inline(never)]
unsafe fn call_all_f64_aarch64(func_ptr: usize, args: &[f64]) -> f64 {
    use core::arch::asm;

    let n = args.len();
    // Register args (up to 8); pad missing with 0.0 (callee won't read them).
    let mut reg = [0.0f64; 8];
    for (i, slot) in reg.iter_mut().enumerate() {
        if i < n {
            *slot = args[i];
        }
    }

    // Stack-spilled args: args[8..]. Bytes = stacked_count * 8, rounded up to a
    // 16-byte multiple so `sp` stays 16-aligned across the `blr`.
    let stacked = if n > 8 { &args[8..] } else { &[][..] };
    let stacked_count = stacked.len();
    let raw_bytes = stacked_count * 8;
    let stack_bytes = (raw_bytes + 15) & !15;

    let ret: f64;
    asm!(
        // Stash the current sp in a CALLEE-SAVED register (x20, declared as a
        // clobber below so the compiler saves/restores it around this asm). The
        // callee preserves callee-saved registers, so x20 survives the `blr` —
        // restoring sp from a caller-saved copy or by re-adding a clobbered
        // `stack_bytes` would corrupt the stack.
        "mov x20, sp",
        // Reserve aligned stack space for the spilled args.
        "sub sp, sp, {stack_bytes}",
        // Copy spilled args from the source pointer into [sp + i*8].
        "mov {i}, xzr",
        "cbz {cnt}, 3f",
        "2:",
        "ldr {tmp}, [{src}, {i}, lsl #3]",
        "str {tmp}, [sp, {i}, lsl #3]",
        "add {i}, {i}, #1",
        "cmp {i}, {cnt}",
        "b.lo 2b",
        "3:",
        "blr {func}",
        // Restore sp from the callee-saved copy.
        "mov sp, x20",
        func = in(reg) func_ptr,
        src = in(reg) stacked.as_ptr(),
        cnt = in(reg) stacked_count,
        stack_bytes = in(reg) stack_bytes,
        i = out(reg) _,
        tmp = out(reg) _,
        out("x20") _,
        // FP argument registers v0–v7.
        inout("d0") reg[0] => ret,
        inout("d1") reg[1] => _,
        inout("d2") reg[2] => _,
        inout("d3") reg[3] => _,
        inout("d4") reg[4] => _,
        inout("d5") reg[5] => _,
        inout("d6") reg[6] => _,
        inout("d7") reg[7] => _,
        // Caller-saved registers the callee may clobber (AAPCS64). x0–x17 and
        // x30(lr) are call-clobbered GPRs; v8–v15 lower 64 bits are
        // callee-saved (preserved), v16–v31 are caller-saved.
        lateout("x0") _, lateout("x1") _, lateout("x2") _, lateout("x3") _,
        lateout("x4") _, lateout("x5") _, lateout("x6") _, lateout("x7") _,
        lateout("x8") _, lateout("x9") _, lateout("x10") _, lateout("x11") _,
        lateout("x12") _, lateout("x13") _, lateout("x14") _, lateout("x15") _,
        lateout("x16") _, lateout("x17") _, lateout("x30") _,
        lateout("v16") _, lateout("v17") _, lateout("v18") _, lateout("v19") _,
        lateout("v20") _, lateout("v21") _, lateout("v22") _, lateout("v23") _,
        lateout("v24") _, lateout("v25") _, lateout("v26") _, lateout("v27") _,
        lateout("v28") _, lateout("v29") _, lateout("v30") _, lateout("v31") _,
    );
    ret
}

/// SysV x86-64: the first 8 f64 args go in xmm0–xmm7; args 9+ are spilled to the
/// stack (each 8 bytes), with the stack 16-byte aligned at the `call`. `al` must
/// hold the number of vector registers used for a (possibly) variadic callee;
/// Perry callees are non-variadic, but setting `al` is harmless and matches the
/// ABI requirement for safety.
#[cfg(all(target_arch = "x86_64", not(target_os = "windows")))]
#[inline(never)]
unsafe fn call_all_f64_x86_64(func_ptr: usize, args: &[f64]) -> f64 {
    let n = args.len();
    let mut reg = [0.0f64; 8];
    for (i, slot) in reg.iter_mut().enumerate() {
        if i < n {
            *slot = args[i];
        }
    }

    let stacked = if n > 8 { &args[8..] } else { &[][..] };
    call_all_f64_x86_64_asm(func_ptr, reg.as_ptr(), stacked.as_ptr(), stacked.len())
}

// Own the entire frame so CFI never depends on a compiler-generated prologue.
#[cfg(all(target_arch = "x86_64", not(target_os = "windows")))]
#[unsafe(naked)]
unsafe extern "C-unwind" fn call_all_f64_x86_64_asm(
    func_ptr: usize,
    reg: *const f64,
    stacked: *const f64,
    stacked_count: usize,
) -> f64 {
    core::arch::naked_asm!(
        ".cfi_startproc",
        "push rbp",
        ".cfi_def_cfa_offset 16",
        ".cfi_offset rbp, -16",
        "mov rbp, rsp",
        ".cfi_def_cfa_register rbp",
        // CFA = rbp + 16 throughout the dynamic allocation and callee call.
        // SysV inputs: rdi = callee, rsi = reg, rdx = stacked, rcx = count.
        "lea rax, [rcx*8 + 15]",
        "and rax, -16",
        "sub rsp, rax",
        "xor r8d, r8d",
        "test rcx, rcx",
        "jz 3f",
        "2:",
        "mov r9, qword ptr [rdx + r8*8]",
        "mov qword ptr [rsp + r8*8], r9",
        "inc r8",
        "cmp r8, rcx",
        "jb 2b",
        "3:",
        "movsd xmm0, qword ptr [rsi]",
        "movsd xmm1, qword ptr [rsi + 8]",
        "movsd xmm2, qword ptr [rsi + 16]",
        "movsd xmm3, qword ptr [rsi + 24]",
        "movsd xmm4, qword ptr [rsi + 32]",
        "movsd xmm5, qword ptr [rsi + 40]",
        "movsd xmm6, qword ptr [rsi + 48]",
        "movsd xmm7, qword ptr [rsi + 56]",
        // al carries the vector-register count for variadic callees.
        "mov eax, 8",
        "call rdi",
        "mov rsp, rbp",
        ".cfi_def_cfa_register rsp",
        "pop rbp",
        ".cfi_def_cfa_offset 8",
        ".cfi_restore rbp",
        "ret",
        ".cfi_endproc",
    );
}

/// Portable fallback for non-asm targets (incl. Windows x64, whose ABI differs
/// from the SysV asm above): fixed-arity dispatch up to 16 f64 args. No current
/// Perry host other than SysV aarch64/x86-64 exercises high-arity dynamic ctor
/// dispatch, so this bound is sufficient there. Arities > 16 FAIL CLOSED: a
/// fixed 16-arg `transmute` would mis-call the fn pointer with the wrong
/// signature (reading register/stack garbage for the missing params), the exact
/// silent-miscompile class that motivated #5437 — so we panic instead.
#[cfg(not(target_arch = "aarch64"))]
#[inline(never)]
unsafe fn call_all_f64_fallback(func_ptr: usize, args: &[f64]) -> f64 {
    #[inline(always)]
    fn a(args: &[f64], i: usize) -> f64 {
        args.get(i)
            .copied()
            .unwrap_or_else(|| f64::from_bits(crate::value::TAG_UNDEFINED))
    }
    macro_rules! arm {
        ($($i:expr),*) => {{
            let f: extern "C" fn($(replace_expr!($i, f64)),*) -> f64 =
                std::mem::transmute(func_ptr);
            f($(a(args, $i)),*)
        }};
    }
    macro_rules! replace_expr {
        ($_t:expr, $sub:ty) => {
            $sub
        };
    }
    // args already includes `this` as element 0.
    match args.len() {
        0 => 0.0,
        1 => arm!(0),
        2 => arm!(0, 1),
        3 => arm!(0, 1, 2),
        4 => arm!(0, 1, 2, 3),
        5 => arm!(0, 1, 2, 3, 4),
        6 => arm!(0, 1, 2, 3, 4, 5),
        7 => arm!(0, 1, 2, 3, 4, 5, 6),
        8 => arm!(0, 1, 2, 3, 4, 5, 6, 7),
        9 => arm!(0, 1, 2, 3, 4, 5, 6, 7, 8),
        10 => arm!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9),
        11 => arm!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10),
        12 => arm!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11),
        13 => arm!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12),
        14 => arm!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13),
        15 => arm!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14),
        16 => arm!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15),
        // FAIL CLOSED: do NOT transmute a >16-arg call to a 16-arg signature —
        // the extra params would read register/stack garbage (#5437). This
        // target has no asm trampoline; high-arity dynamic dispatch is
        // unsupported here.
        n => panic!(
            "abi_trampoline: unsupported arity {n} on this target \
             (no asm trampoline; portable fallback caps at 16 f64 args)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A 70-param all-f64 callee: returns arg0*1 + arg1*2 + ... weighted sum so
    // a misplaced/garbage arg is detectable, plus marker on the last few.
    #[cfg(any(
        target_arch = "aarch64",
        all(target_arch = "x86_64", not(target_os = "windows"))
    ))]
    extern "C" fn sum70(
        a0: f64,
        a1: f64,
        a2: f64,
        a3: f64,
        a4: f64,
        a5: f64,
        a6: f64,
        a7: f64,
        a8: f64,
        a9: f64,
        a10: f64,
        a11: f64,
        a12: f64,
        a13: f64,
        a14: f64,
        a15: f64,
        a16: f64,
        a17: f64,
        a18: f64,
        a19: f64,
        a20: f64,
        a21: f64,
        a22: f64,
        a23: f64,
        a24: f64,
        a25: f64,
        a26: f64,
        a27: f64,
        a28: f64,
        a29: f64,
        a30: f64,
        a31: f64,
        a32: f64,
        a33: f64,
        a34: f64,
        a35: f64,
        a36: f64,
        a37: f64,
        a38: f64,
        a39: f64,
        a40: f64,
        a41: f64,
        a42: f64,
        a43: f64,
        a44: f64,
        a45: f64,
        a46: f64,
        a47: f64,
        a48: f64,
        a49: f64,
        a50: f64,
        a51: f64,
        a52: f64,
        a53: f64,
        a54: f64,
        a55: f64,
        a56: f64,
        a57: f64,
        a58: f64,
        a59: f64,
        a60: f64,
        a61: f64,
        a62: f64,
        a63: f64,
        a64: f64,
        a65: f64,
        a66: f64,
        a67: f64,
        a68: f64,
        a69: f64,
    ) -> f64 {
        let xs = [
            a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18,
            a19, a20, a21, a22, a23, a24, a25, a26, a27, a28, a29, a30, a31, a32, a33, a34, a35,
            a36, a37, a38, a39, a40, a41, a42, a43, a44, a45, a46, a47, a48, a49, a50, a51, a52,
            a53, a54, a55, a56, a57, a58, a59, a60, a61, a62, a63, a64, a65, a66, a67, a68, a69,
        ];
        let mut acc = 0.0;
        for (i, x) in xs.iter().enumerate() {
            acc += x * (i as f64 + 1.0);
        }
        acc
    }

    // High-arity (>16) dynamic dispatch only works on the asm targets; the
    // portable fallback fails closed (panics) above 16 args, so this test is
    // gated to the SysV asm targets.
    #[cfg(any(
        target_arch = "aarch64",
        all(target_arch = "x86_64", not(target_os = "windows"))
    ))]
    #[test]
    fn trampoline_passes_70_args_in_order() {
        // args = [this=100, then 69 values 1..=69]. call_all_f64 takes the full
        // arg list including `this` as element 0 → 70 total → sum70.
        let mut args = Vec::with_capacity(70);
        args.push(100.0); // a0 (this)
        for i in 1..70 {
            args.push(i as f64);
        }
        let got = unsafe { call_all_f64(sum70 as *const () as usize, &args) };
        // expected = sum(args[i]*(i+1))
        let expected: f64 = args
            .iter()
            .enumerate()
            .map(|(i, x)| x * (i as f64 + 1.0))
            .sum();
        assert_eq!(got, expected, "trampoline mis-ordered args");
    }

    extern "C" fn pick(
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
        g: f64,
        h: f64,
        i: f64,
        j: f64,
    ) -> f64 {
        // beyond-register-window pick: returns the 9th and 10th (stack) args
        // combined so a stack-spill bug is caught.
        let _ = (a, b, c, d, e, f, g, h);
        i * 1000.0 + j
    }

    #[test]
    fn trampoline_stack_spill_args_9_and_10() {
        let args = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 42.0, 7.0];
        let got = unsafe { call_all_f64(pick as *const () as usize, &args) };
        assert_eq!(got, 42.0 * 1000.0 + 7.0);
        #[cfg(all(target_arch = "x86_64", not(target_os = "windows")))]
        {
            let got = unsafe { call_all_f64_x86_64(pick as *const () as usize, &args) };
            assert_eq!(got, 42.0 * 1000.0 + 7.0);
        }
    }

    #[inline(never)]
    extern "C" fn walk13(
        a: f64, b: f64, c: f64, d: f64, e: f64, f: f64, g: f64,
        h: f64, i: f64, j: f64, k: f64, l: f64, m: f64,
    ) -> f64 {
        let _ = (a, b, c, d, e, f, g, h, i, j, k, l, m);
        crate::eh::selfcheck_frame_b() as f64
    }

    #[inline(never)]
    extern "C" fn walk20(
        a: f64, b: f64, c: f64, d: f64, e: f64, f: f64, g: f64,
        h: f64, i: f64, j: f64, k: f64, l: f64, m: f64, n: f64,
        o: f64, p: f64, q: f64, r: f64, s: f64, t: f64,
    ) -> f64 {
        let _ = (a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t);
        crate::eh::selfcheck_frame_b() as f64
    }

    // The moving GC scans the stack with `_Unwind_Backtrace` from inside
    // dynamically dispatched ctors. A 13-arg ctor (ThreeNative's engine
    // facade) made the walk read a wrong CFA through the rsp-moving asm and
    // fault; the walk must see at least as many frames as a direct call does.
    #[cfg(all(target_arch = "x86_64", not(target_os = "windows")))]
    #[test]
    fn trampoline_frame_is_walkable_by_the_unwinder() {
        let args13 = [1.0; 13];
        let direct13 = walk13(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let through = unsafe { call_all_f64(walk13 as *const () as usize, &args13) };
        assert!(through >= direct13, "unwinder saw {through} frames through the trampoline, {direct13} direct");

        // Above B7's fixed-arity ceiling, this must exercise the asm frame.
        let args = [1.0; 20];
        let direct = walk20(
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        );
        let through = unsafe { call_all_f64(walk20 as *const () as usize, &args) };
        assert!(through >= direct, "20-arg unwinder saw {through} frames through the trampoline, {direct} direct");

        // Also cover the asm's odd spill count despite the small-arity routing.
        let through = unsafe { call_all_f64_x86_64(walk13 as *const () as usize, &args13) };
        assert!(through >= direct13, "13-arg asm unwinder saw {through} frames through the trampoline, {direct13} direct");
    }
}
