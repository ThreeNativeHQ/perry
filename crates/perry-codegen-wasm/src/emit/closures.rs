//! Closure collection helpers extracted from emit/mod.rs (#1102 mechanical split).
//!
//! Pure move: `collect_closures_from_stmts` / `collect_closures_from_expr`.

use perry_hir::ir::*;
use perry_hir::types::{FuncId, LocalId};

/// Recursively collect all Expr::Closure nodes from statements
pub(super) fn collect_closures_from_stmts(
    stmts: &[Stmt],
    out: &mut Vec<(FuncId, Vec<Param>, Vec<Stmt>, Vec<LocalId>, Vec<LocalId>)>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let { init, .. } => {
                if let Some(e) = init {
                    collect_closures_from_expr(e, out);
                }
            }
            Stmt::Expr(e) | Stmt::Throw(e) => collect_closures_from_expr(e, out),
            Stmt::Return(e) => {
                if let Some(e) = e {
                    collect_closures_from_expr(e, out);
                }
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                collect_closures_from_expr(condition, out);
                collect_closures_from_stmts(then_branch, out);
                if let Some(eb) = else_branch {
                    collect_closures_from_stmts(eb, out);
                }
            }
            Stmt::While { condition, body } => {
                collect_closures_from_expr(condition, out);
                collect_closures_from_stmts(body, out);
            }
            Stmt::For {
                init,
                condition,
                update,
                body,
            } => {
                if let Some(i) = init {
                    collect_closures_from_stmts(std::slice::from_ref(i.as_ref()), out);
                }
                if let Some(c) = condition {
                    collect_closures_from_expr(c, out);
                }
                if let Some(u) = update {
                    collect_closures_from_expr(u, out);
                }
                collect_closures_from_stmts(body, out);
            }
            Stmt::Try {
                body,
                catch,
                finally,
            } => {
                collect_closures_from_stmts(body, out);
                if let Some(c) = catch {
                    collect_closures_from_stmts(&c.body, out);
                }
                if let Some(f) = finally {
                    collect_closures_from_stmts(f, out);
                }
            }
            Stmt::Switch {
                discriminant,
                cases,
            } => {
                collect_closures_from_expr(discriminant, out);
                for case in cases {
                    if let Some(t) = &case.test {
                        collect_closures_from_expr(t, out);
                    }
                    collect_closures_from_stmts(&case.body, out);
                }
            }
            Stmt::DoWhile { body, condition } => {
                collect_closures_from_stmts(body, out);
                collect_closures_from_expr(condition, out);
            }
            Stmt::Labeled { body, .. } => {
                collect_closures_from_stmts(std::slice::from_ref(body.as_ref()), out);
            }
            Stmt::Break
            | Stmt::Continue
            | Stmt::LabeledBreak(..)
            | Stmt::LabeledContinue(..)
            | Stmt::PreallocateBoxes(..)
            | Stmt::PreallocateTdzBoxes(..)
            | Stmt::ReleaseBoxes(..) => {}
        }
    }
}

/// Capture id that stands for the enclosing `this` of a closure with `captures_this`; a closure
/// reads `this` from this parameter instead of local 0 (PRD-582).
pub(super) const THIS_CAPTURE: LocalId = LocalId::MAX;

/// Recursively collect Expr::Closure from an expression tree. Descends through
/// `perry_hir::walker`, which is exhaustive over `Expr`: a hand-written match here
/// missed closures inside template literals, method-call args and array pushes, so
/// they were never registered and evaluated to `undefined` (PRD-582).
pub(super) fn collect_closures_from_expr(
    expr: &Expr,
    out: &mut Vec<(FuncId, Vec<Param>, Vec<Stmt>, Vec<LocalId>, Vec<LocalId>)>,
) {
    if let Expr::Closure {
        func_id,
        params,
        body,
        captures,
        mutable_captures,
        captures_this,
        ..
    } = expr
    {
        // `this` rides as one more capture, after the plain captures (see THIS_CAPTURE).
        let mut captures = captures.clone();
        if *captures_this {
            captures.push(THIS_CAPTURE);
        }
        out.push((*func_id, params.clone(), body.clone(), captures, mutable_captures.clone()));
        collect_closures_from_stmts(body, out);
    }
    perry_hir::walker::walk_expr_children(expr, &mut |child| collect_closures_from_expr(child, out));
}
