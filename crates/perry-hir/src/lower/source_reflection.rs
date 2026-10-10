//! Strict native builds cannot reconstruct source they do not retain.

use anyhow::Result;
use swc_common::Spanned;
use swc_ecma_ast as ast;

use super::expr_member::{static_member_prop_name, unwrap_transparent};
use super::LoweringContext;
use crate::lower_types::infer_type_from_expr;
use crate::types::Type;

fn named(expr: &ast::Expr, name: &str) -> bool {
    matches!(unwrap_transparent(expr), ast::Expr::Ident(id) if id.sym == name)
}

fn prototype(expr: &ast::Expr, name: &str) -> bool {
    matches!(unwrap_transparent(expr), ast::Expr::Member(m)
        if named(&m.obj, name) && static_member_prop_name(&m.prop).as_deref() == Some("prototype"))
}

fn noncallable(ty: &Type) -> bool {
    match ty {
        Type::Union(types) => !types.is_empty() && types.iter().all(noncallable),
        _ => ty.is_primitive() || matches!(ty, Type::Never),
    }
}

fn require_noncallable(ctx: &LoweringContext, expr: &ast::Expr, surface: &str) -> Result<()> {
    // Unknown/any and structural objects may hold callables: refuse rather than
    // silently return a synthesized native-code body in a source-free build.
    if !noncallable(&infer_type_from_expr(unwrap_transparent(expr), ctx)) {
        crate::lower_bail!(
            expr.span(),
            "ERR_STRICT_SOURCE_REFLECTION: {} may reflect function or class source; \
             --strict-unimplemented requires a provably noncallable value",
            surface
        );
    }
    Ok(())
}

fn check_member(ctx: &LoweringContext, member: &ast::MemberExpr) -> Result<()> {
    let mut current = Some(member);
    while let Some(member) = current {
        let prop = static_member_prop_name(&member.prop);
        if named(&member.obj, "Function")
            && !ctx.shadows_unqualified_global("Function")
            && prop.as_deref() == Some("prototype")
        {
            // Refuse the prototype value too: destructuring/aliasing can otherwise
            // hide Function.prototype.toString from subsequent member checks.
            crate::lower_bail!(member.span(),
                "ERR_STRICT_SOURCE_REFLECTION: Function.prototype access or alias may expose Function.prototype.toString");
        }
        if prop.as_deref() == Some("toString")
            && !(prototype(&member.obj, "Object") && !ctx.shadows_unqualified_global("Object"))
        {
            require_noncallable(ctx, &member.obj, "toString access")?;
        }
        current = match unwrap_transparent(&member.obj) {
            ast::Expr::Member(object) => Some(object),
            _ => None,
        };
    }
    Ok(())
}

pub(super) fn check(ctx: &LoweringContext, expr: &ast::Expr) -> Result<()> {
    match expr {
        ast::Expr::Member(member) => check_member(ctx, member)?,
        ast::Expr::Call(call) => {
            if let ast::Callee::Expr(callee) = &call.callee {
                match unwrap_transparent(callee) {
                    ast::Expr::Member(member) => check_member(ctx, member)?,
                    ast::Expr::OptChain(_) => check(ctx, callee)?,
                    ast::Expr::Ident(id)
                        if id.sym == "String" && !ctx.shadows_unqualified_global("String") =>
                    {
                        if let Some(arg) = call.args.first() {
                            require_noncallable(ctx, &arg.expr, "String(value)")?;
                        }
                    }
                    _ => {}
                }
            }
        }
        ast::Expr::Tpl(tpl) => {
            for value in &tpl.exprs {
                require_noncallable(ctx, value, "template interpolation")?;
            }
        }
        ast::Expr::OptChain(chain) => match chain.base.as_ref() {
            ast::OptChainBase::Member(member) => check_member(ctx, member)?,
            ast::OptChainBase::Call(call) => {
                if let ast::Expr::OptChain(_) = unwrap_transparent(&call.callee) {
                    check(ctx, &call.callee)?;
                }
                if let ast::Expr::Member(member) = unwrap_transparent(&call.callee) {
                    check_member(ctx, member)?;
                }
                if named(&call.callee, "String") && !ctx.shadows_unqualified_global("String") {
                    if let Some(arg) = call.args.first() {
                        require_noncallable(ctx, &arg.expr, "String(value)")?;
                    }
                }
            }
        },
        ast::Expr::Ident(id) if id.sym == "String" && !ctx.shadows_unqualified_global("String") => {
            crate::lower_bail!(
                expr.span(),
                "ERR_STRICT_SOURCE_REFLECTION: String alias may reflect function or class source"
            );
        }
        _ => {}
    }
    Ok(())
}
