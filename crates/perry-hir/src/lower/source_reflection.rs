//! Strict builds refuse proven callable reflection; dynamic values use native form.

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

fn provably_callable(ctx: &LoweringContext, expr: &ast::Expr) -> bool {
    let expr = unwrap_transparent(expr);
    match expr {
        ast::Expr::Fn(_) | ast::Expr::Arrow(_) | ast::Expr::Class(_) => true,
        ast::Expr::Ident(id) => {
            if let Some(local) = ctx.lookup_local(id.sym.as_ref()) {
                !ctx.source_reflection_dynamic_locals.contains(&local)
                    && (ctx.function_valued_locals.contains(&local)
                        || ctx.source_reflection_class_locals.contains(&local)
                        || matches!(
                            ctx.lookup_local_type(id.sym.as_ref()),
                            Some(Type::Function(_))
                        ))
            } else {
                ctx.lookup_func(id.sym.as_ref()).is_some()
                    || ctx.lookup_class(id.sym.as_ref()).is_some()
            }
        }
        ast::Expr::Member(member) => {
            let Some(prop) = static_member_prop_name(&member.prop) else {
                return false;
            };
            let ty = infer_type_from_expr(unwrap_transparent(&member.obj), ctx);
            match ty {
                Type::Object(object) => object
                    .properties
                    .get(&prop)
                    .is_some_and(|property| matches!(property.ty, Type::Function(_))),
                Type::Named(name) => {
                    !ctx.lookup_class_accessor_names(&name)
                        .is_some_and(|accessors| accessors.contains_any(&prop))
                        && ctx.lookup_class_method_return_type(&name, &prop).is_some()
                }
                _ => {
                    if let ast::Expr::Ident(id) = unwrap_transparent(&member.obj) {
                        if ctx.lookup_local(id.sym.as_ref()).is_none()
                            && ctx.lookup_class(id.sym.as_ref()).is_some()
                        {
                            return ctx.has_static_method(id.sym.as_ref(), &prop);
                        }
                    }
                    false
                }
            }
        }
        _ => matches!(infer_type_from_expr(expr, ctx), Type::Function(_)),
    }
}

fn refuse_callable(ctx: &LoweringContext, expr: &ast::Expr, surface: &str) -> Result<()> {
    if provably_callable(ctx, expr) {
        crate::lower_bail!(
            expr.span(),
            "ERR_STRICT_SOURCE_REFLECTION: {} reflects function or class source; \
             --strict-unimplemented does not retain callable source",
            surface
        );
    }
    Ok(())
}

fn check_member(ctx: &LoweringContext, member: &ast::MemberExpr) -> Result<()> {
    let mut current = Some(member);
    while let Some(member) = current {
        let prop = static_member_prop_name(&member.prop);
        if prototype(&member.obj, "Function")
            && !ctx.shadows_unqualified_global("Function")
            && prop.as_deref() == Some("toString")
        {
            crate::lower_bail!(member.span(),
                "ERR_STRICT_SOURCE_REFLECTION: Function.prototype.toString access or alias reflects callable source");
        }
        if prop.as_deref() == Some("toString")
            && !(prototype(&member.obj, "Object") && !ctx.shadows_unqualified_global("Object"))
        {
            refuse_callable(ctx, &member.obj, "toString access")?;
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
                            refuse_callable(ctx, &arg.expr, "String(value)")?;
                        }
                    }
                    _ => {}
                }
            }
        }
        ast::Expr::Tpl(tpl) => {
            for value in &tpl.exprs {
                refuse_callable(ctx, value, "template interpolation")?;
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
                        refuse_callable(ctx, &arg.expr, "String(value)")?;
                    }
                }
            }
        },
        _ => {}
    }
    Ok(())
}
