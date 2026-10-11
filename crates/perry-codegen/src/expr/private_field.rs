//! Tagged private slots use exact shape, element-initialization and evaluation proofs.
use super::{emit_string_literal_global, lower_expr, FnCtx};
use crate::types::{DOUBLE, I32, I64, I8, PTR};
use anyhow::Result;
use perry_hir::Expr;

fn candidate(
    ctx: &FnCtx<'_>,
    class_name: &str,
    declaring_id: u32,
    field: &str,
    object: &Expr,
) -> Option<(String, String, u32, u32)> {
    let concrete = crate::type_analysis::receiver_class_name(ctx, object)
        .unwrap_or_else(|| class_name.to_string());
    let class = *ctx.classes.get(&concrete)?;
    // Every ancestor must author a complete, static canonical layout. Imported
    // stubs do not carry a dynamic-heritage completeness capability.
    let mut ancestor: Option<&perry_hir::Class> = Some(class);
    for _ in 0..64 {
        let Some(current) = ancestor else { break };
        if current.is_imported_stub()
            || current.extends_expr.is_some()
            || current.native_extends.is_some()
        {
            return None;
        }
        ancestor = match current.extends_name.as_ref() {
            Some(name) => Some(*ctx.classes.get(name)?),
            None => None,
        };
    }
    if ancestor.is_some() {
        return None;
    }
    let concrete_id = *ctx.class_ids.get(&concrete)?;
    if concrete_id == 0 || class.id != concrete_id {
        return None;
    }
    let slot = ctx
        .class_private_slots
        .get(&concrete)?
        .iter()
        .find(|slot| slot.declaring_class_id == declaring_id && slot.field_name == field)?;
    let keys = ctx.class_keys_globals.get(&concrete)?.clone();
    Some((concrete, keys, concrete_id, slot.presence_slot))
}

fn brand_owner(
    ctx: &mut FnCtx<'_>,
    receiver: &str,
    receiver_is_brand_owner: bool,
) -> Result<String> {
    if receiver_is_brand_owner {
        Ok(receiver.to_string())
    } else {
        lower_expr(ctx, &Expr::This)
    }
}

pub(crate) fn admit(
    ctx: &mut FnCtx<'_>,
    class_name: &str,
    declaring_id: u32,
    field: &str,
    object: &Expr,
    receiver: &str,
    owner: &str,
) -> Option<(String, u32)> {
    let (concrete, keys, cid, slot) = candidate(ctx, class_name, declaring_id, field, object)?;
    let shape = crate::typed_shape::load_class_shape_id(ctx, &concrete, &keys);
    let handle = ctx.block().call(
        I64,
        "js_private_slot_admit",
        &[
            (DOUBLE, receiver),
            (DOUBLE, owner),
            (I32, &declaring_id.to_string()),
            (I32, &cid.to_string()),
            (I32, &shape),
            (I32, &slot.to_string()),
        ],
    );
    Some((handle, slot + 1))
}

fn semantic_guard(
    ctx: &mut FnCtx<'_>,
    receiver: &str,
    owner: &str,
    declaring_id: u32,
    field: &str,
    op: u8,
) -> String {
    let key = emit_string_literal_global(ctx, field);
    ctx.block().call(
        DOUBLE,
        "js_private_guard",
        &[
            (DOUBLE, receiver),
            (DOUBLE, owner),
            (I32, &declaring_id.to_string()),
            (PTR, &key),
            (I32, &field.len().to_string()),
            (I32, "0"),
            (I32, &op.to_string()),
        ],
    )
}

fn value_slot(ctx: &mut FnCtx<'_>, handle: &str, slot: u32) -> String {
    let skip = crate::target_layout::object_header_size_bytes(ctx.target_triple).to_string();
    let blk = ctx.block();
    let object = blk.inttoptr(I64, handle);
    let fields = blk.gep(I8, &object, &[(I64, &skip)]);
    blk.gep(DOUBLE, &fields, &[(I64, &slot.to_string())])
}

pub(crate) fn get(ctx: &mut FnCtx<'_>, object: &Expr, property: &str) -> Result<Option<String>> {
    let Expr::PrivateGuard {
        class_name,
        class_id,
        field_name,
        kind: 0,
        op: 0,
        receiver_is_brand_owner,
        object: receiver_expr,
    } = object
    else {
        return Ok(None);
    };
    if candidate(ctx, class_name, *class_id, field_name, receiver_expr).is_none() {
        return Ok(None);
    }
    let receiver = lower_expr(ctx, receiver_expr)?;
    let owner = brand_owner(ctx, &receiver, *receiver_is_brand_owner)?;
    let (handle, slot) = admit(
        ctx,
        class_name,
        *class_id,
        field_name,
        receiver_expr,
        &receiver,
        &owner,
    )
    .unwrap();
    let fast = ctx.new_block("private_field_get.fast");
    let slow = ctx.new_block("private_field_get.slow");
    let merge = ctx.new_block("private_field_get.merge");
    let (fast_label, slow_label, merge_label) = (
        ctx.block_label(fast),
        ctx.block_label(slow),
        ctx.block_label(merge),
    );
    let miss = ctx.block().icmp_eq(I64, &handle, "0");
    ctx.block().cond_br(&miss, &slow_label, &fast_label);
    ctx.current_block = fast;
    let ptr = value_slot(ctx, &handle, slot);
    let value = ctx.block().load(DOUBLE, &ptr);
    ctx.block().br(&merge_label);
    ctx.current_block = slow;
    let checked = semantic_guard(ctx, &receiver, &owner, *class_id, field_name, 0);
    let key_idx = ctx.strings.intern(property);
    let dispatch = ctx.strings.static_dispatch_global(key_idx);
    let blk = ctx.block();
    let bits = blk.bitcast_double_to_i64(&checked);
    let raw = blk.and(I64, &bits, crate::nanbox::POINTER_MASK_I64);
    let property_id = crate::strings::emit_static_dispatch_id(blk, &dispatch);
    let fallback = blk.call(
        DOUBLE,
        "js_object_get_field_by_property_id_f64",
        &[(I64, &raw), (I64, &property_id)],
    );
    let slow_pred = ctx.block_label(ctx.current_block);
    ctx.block().br(&merge_label);
    ctx.current_block = merge;
    Ok(Some(ctx.block().phi(
        DOUBLE,
        &[(&value, &fast_label), (&fallback, &slow_pred)],
    )))
}

/// Prove a private read/reference without allocating; write PutValue is below.
pub(crate) fn guard(ctx: &mut FnCtx<'_>, expr: &Expr) -> Result<Option<String>> {
    let Expr::PrivateGuard {
        class_name,
        class_id,
        field_name,
        kind: 0,
        op,
        receiver_is_brand_owner,
        object,
    } = expr
    else {
        return Ok(None);
    };
    if *op >= 2 || candidate(ctx, class_name, *class_id, field_name, object).is_none() {
        return Ok(None);
    }
    let receiver = lower_expr(ctx, object)?;
    let owner = brand_owner(ctx, &receiver, *receiver_is_brand_owner)?;
    let (handle, _) = admit(
        ctx, class_name, *class_id, field_name, object, &receiver, &owner,
    )
    .unwrap();
    let slow = ctx.new_block("private_field_guard.slow");
    let done = ctx.new_block("private_field_guard.done");
    let (slow_label, done_label) = (ctx.block_label(slow), ctx.block_label(done));
    let fast_pred = ctx.block_label(ctx.current_block);
    let miss = ctx.block().icmp_eq(I64, &handle, "0");
    ctx.block().cond_br(&miss, &slow_label, &done_label);
    ctx.current_block = slow;
    let checked = semantic_guard(ctx, &receiver, &owner, *class_id, field_name, *op);
    ctx.block().br(&done_label);
    ctx.current_block = done;
    Ok(Some(ctx.block().phi(
        DOUBLE,
        &[(&receiver, &fast_pred), (&checked, &slow_label)],
    )))
}

pub(crate) fn set(
    ctx: &mut FnCtx<'_>,
    object: &Expr,
    property: &str,
    value: &Expr,
) -> Result<Option<String>> {
    let Expr::PrivateGuard {
        class_name,
        class_id,
        field_name,
        kind: 0,
        op: 1,
        receiver_is_brand_owner,
        object: receiver_expr,
    } = object
    else {
        return Ok(None);
    };
    if *class_id == 0 {
        return Ok(None);
    }
    crate::rooting::with_operands_rooted(ctx, &[receiver_expr, value], |ctx, values| {
        let receiver = &values[0];
        let value = &values[1];
        let owner = brand_owner(ctx, receiver, *receiver_is_brand_owner)?;
        let admission = admit(
            ctx,
            class_name,
            *class_id,
            field_name,
            receiver_expr,
            receiver,
            &owner,
        );
        let merge = ctx.new_block("private_field_set.merge");
        let merge_label = ctx.block_label(merge);
        if let Some((handle, slot)) = admission {
            let fast = ctx.new_block("private_field_set.fast");
            let slow = ctx.new_block("private_field_set.slow");
            let (fast_label, slow_label) = (ctx.block_label(fast), ctx.block_label(slow));
            let miss = ctx.block().icmp_eq(I64, &handle, "0");
            ctx.block().cond_br(&miss, &slow_label, &fast_label);
            ctx.current_block = fast;
            let ptr = value_slot(ctx, &handle, slot);
            let addr = ctx.block().ptrtoint(&ptr, I64);
            let bits = ctx.block().bitcast_double_to_i64(receiver);
            super::emit_jsvalue_slot_store_pointer_tested(
                ctx,
                &ptr,
                value,
                &handle,
                &slot.to_string(),
                true,
                true,
                &bits,
                &addr,
                true,
                true,
                "private_field_set",
            );
            ctx.block().br(&merge_label);
            ctx.current_block = slow;
        }
        let checked = semantic_guard(ctx, receiver, &owner, *class_id, field_name, 1);
        let key_idx = ctx.strings.intern(property);
        let dispatch = ctx.strings.static_dispatch_global(key_idx);
        let blk = ctx.block();
        let property_id = crate::strings::emit_static_dispatch_id(blk, &dispatch);
        blk.call_void(
            "js_private_field_set_by_property_id",
            &[(DOUBLE, &checked), (I64, &property_id), (DOUBLE, value)],
        );
        blk.br(&merge_label);
        ctx.current_block = merge;
        Ok(Some(value.clone()))
    })
}
