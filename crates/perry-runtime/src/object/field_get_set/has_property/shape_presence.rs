//! Presence on ordinary objects, from their own shape records.
//!
//! No property value is read: an accessor and a data slot containing
//! `undefined` are both present. Every hop supplies its own current shape,
//! including its prototype edge, so mutations need no invalidation state.

use super::*;

// Builtin, wrapper and synthetic runtime class owners can expose virtual
// own keys that their ordinary layout does not enumerate. As in hasOwn,
// only codegen-assigned class ids may take a shape-only presence answer.
const FIRST_RUNTIME_CLASS_ID: u32 = 0x7FFF_FF00;

/// An allocation-free `[[HasProperty]]` for a string key and a chain whose
/// objects have ordinary shapes. Declines at a dictionary or exotic receiver,
/// a class's virtual surface, or a not-yet-materialized default intrinsic.
/// The generic operation remains responsible for coercion and proxy traps.
#[inline(never)]
pub(super) unsafe fn try_shape_has_property(receiver: f64, key: f64) -> Option<bool> {
    let value = JSValue::from_bits(receiver.to_bits());
    let key_value = JSValue::from_bits(key.to_bits());
    if !value.is_pointer() || !key_value.is_any_string() {
        return None;
    }
    let mut scratch = [0u8; crate::value::SHORT_STRING_MAX_LEN];
    let name = crate::string::js_string_key_bytes(key_value, &mut scratch)?;
    let mut obj = value.as_pointer::<ObjectHeader>();
    for _ in 0..32 {
        let header = crate::value::addr_class::try_read_gc_header(obj as usize)?;
        if header.obj_type != crate::gc::GC_TYPE_OBJECT
            || header.gc_flags & crate::gc::GC_FLAG_FORWARDED != 0
            || header._reserved & crate::gc::OBJ_FLAG_TYPED_ARRAY_PROTO != 0
        {
            return None;
        }
        let shape = super::super::super::shapes::object_shape_descriptor(obj)?;
        let proto_kind = shape.proto_id & super::super::super::shapes::PROTO_ID_UNIQUE;
        // Dictionary shapes publish no keys; their private list lives on the
        // receiver. Their disjoint ShapeId band cannot prove presence/absence.
        if !super::super::super::shapes::is_site_matchable_shape_id(
            super::super::super::shapes::object_shape_stamp(obj),
        ) || !shape.object_kind.is_ordinary_layout()
            || (*obj).class_id >= FIRST_RUNTIME_CLASS_ID
            // A null/unique edge does not encode a class's virtual surface.
            // Decline such class-bearing owners rather than infer absence.
            || (proto_kind == super::super::super::shapes::PROTO_ID_UNIQUE
                && (*obj).class_id != 0)
            || shape.proto_id == super::super::super::shapes::PROTO_ID_PER_OBJECT
            || proto_kind == super::super::super::shapes::PROTO_ID_CLASS
            || proto_kind == super::super::super::shapes::PROTO_ID_MIXED
            || shape.summary & crate::object::key_attrs::SUMMARY_PRIVATE != 0
        {
            return None;
        }
        let meta = (*obj).meta;
        if !meta.is_null()
            && ((*meta).elements != 0
                || (*meta).flags & crate::object::OBJECT_META_FLAG_EXOTIC_READ_RECEIVER != 0)
        {
            return None;
        }
        // The shape owns this resolved key list, including accessor entries.
        // Deletion removes/tombstones a key in the list; presence never
        // depends on the corresponding value lane or calls its getter.
        if crate::object::keys_find_slot_by_bytes_resolved(
            shape.keys as usize as *const crate::array::ArrayHeader,
            shape.logical_key_count,
            name,
        )
        .is_some()
        {
            return Some(true);
        }
        let next = match shape.proto_id {
            super::super::super::shapes::PROTO_ID_NULL => return Some(false),
            super::super::super::shapes::PROTO_ID_DEFAULT => {
                let addr = crate::array::object_prototype_addr_if_resolved();
                if addr == 0 {
                    return None;
                }
                addr as *const ObjectHeader
            }
            _ => {
                let word = super::super::super::shapes::object_prototype_word(obj);
                let prototype = JSValue::from_bits(word);
                if !prototype.is_pointer() {
                    return None;
                }
                prototype.as_pointer::<ObjectHeader>() as *const ObjectHeader
            }
        };
        obj = next as *mut ObjectHeader;
    }
    None
}

/// The non-shape facts [`try_shape_has_property`] reads at one hop: its GC
/// header, class id and element/exotic meta. Everything else it reads is a
/// fact of the hop's ShapeId, so a caller holding a primed ShapeId and class id
/// re-asks only these.
#[inline]
unsafe fn hop_plain(obj: *const ObjectHeader, shape: u32, class_id: u32) -> bool {
    let Some(header) = crate::value::addr_class::try_read_gc_header(obj as usize) else {
        return false;
    };
    if header.obj_type != crate::gc::GC_TYPE_OBJECT
        || header.gc_flags & crate::gc::GC_FLAG_FORWARDED != 0
        // A tombstoning delete keeps the ShapeId; a later re-add could too.
        || header._reserved
            & (crate::gc::OBJ_FLAG_TYPED_ARRAY_PROTO | crate::gc::OBJ_FLAG_STABLE_TOMBSTONES)
            != 0
        || super::super::super::shapes::object_shape_stamp(obj) != shape
        || (*obj).class_id != class_id
    {
        return false;
    }
    let meta = (*obj).meta;
    meta.is_null()
        || ((*meta).elements == 0
            && (*meta).flags & crate::object::OBJECT_META_FLAG_EXOTIC_READ_RECEIVER == 0)
}

/// A negative `in` answer as shape facts: `(receiver ShapeId, receiver class,
/// Object.prototype ShapeId, its class)` when [`try_shape_has_property`]
/// proved `key` absent on exactly `receiver -> Object.prototype -> null`.
/// Each ShapeId pins its own key list and [[Prototype]] edge, and ShapeIds
/// are never reused, so [`absent_still_holds`] needs no invalidation state —
/// the same argument as the read site's ABSENT holder entry.
pub(in crate::object::field_get_set) unsafe fn absent_two_hop_facts(
    receiver: f64,
    key: f64,
) -> Option<[u32; 4]> {
    use super::super::super::shapes::{object_shape_descriptor, object_shape_stamp};
    if try_shape_has_property(receiver, key) != Some(false) {
        return None;
    }
    let obj = JSValue::from_bits(receiver.to_bits()).as_pointer::<ObjectHeader>();
    if object_shape_descriptor(obj)?.proto_id != super::super::super::shapes::PROTO_ID_DEFAULT {
        return None;
    }
    let proto = crate::array::object_prototype_addr_if_resolved() as *const ObjectHeader;
    if proto.is_null()
        || object_shape_descriptor(proto)?.proto_id != super::super::super::shapes::PROTO_ID_NULL
    {
        return None;
    }
    Some([object_shape_stamp(obj), (*obj).class_id, object_shape_stamp(proto), (*proto).class_id])
}

/// Whether facts from [`absent_two_hop_facts`] still decide `false` for
/// `receiver`: both hops carry the primed ShapeIds and pass the non-shape
/// checks [`try_shape_has_property`] makes.
#[inline]
pub(in crate::object::field_get_set) unsafe fn absent_still_holds(
    receiver: f64,
    facts: [u32; 4],
) -> bool {
    let value = JSValue::from_bits(receiver.to_bits());
    if !value.is_pointer() {
        return false;
    }
    let proto = crate::array::object_prototype_addr_if_resolved() as *const ObjectHeader;
    !proto.is_null()
        && hop_plain(value.as_pointer::<ObjectHeader>(), facts[0], facts[1])
        && hop_plain(proto, facts[2], facts[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_presence_falls_back_for_own_and_inherited_keys() {
        let _lock = crate::gc::global_side_table_test_lock();
        let _no_gc = crate::gc::GcSuppressScope::new();
        unsafe {
            crate::object::ensure_object_intrinsics();
            let obj = crate::object::object_alloc_plain(0);
            for i in 0..6 {
                let name = format!("dictionary_presence_{i}");
                let key = crate::string::js_string_from_bytes(name.as_ptr(), name.len() as u32);
                crate::object::js_object_set_field_by_name(obj, key, i as f64);
            }
            let key = crate::string::js_string_from_bytes(b"x".as_ptr(), 1);
            let undefined = f64::from_bits(crate::value::TAG_UNDEFINED);
            crate::object::js_object_set_field_by_name(obj, key, undefined);
            assert!(crate::object::dictionary::latch_object_to_dictionary(obj));
            assert_eq!(
                crate::object::shapes::object_shape_descriptor(obj)
                    .unwrap()
                    .keys,
                0,
                "the dictionary shape omits its private key list"
            );
            let receiver = crate::value::js_nanbox_pointer(obj as i64);
            let boxed_key = crate::value::js_nanbox_string(key as i64);
            let child = crate::object::object_alloc_plain(0);
            let child_value = crate::value::js_nanbox_pointer(child as i64);
            crate::object::js_object_set_prototype_of(child_value, receiver);
            for value in [receiver, child_value] {
                assert_eq!(try_shape_has_property(value, boxed_key), None);
                assert_ne!(
                    crate::value::js_is_truthy(js_in_operator(value, boxed_key)),
                    0
                );
                assert_ne!(
                    crate::value::js_is_truthy(js_object_has_property(value, boxed_key)),
                    0
                );
            }
            crate::object::js_object_delete_field(obj, key);
            for value in [receiver, child_value] {
                assert_eq!(
                    crate::value::js_is_truthy(js_in_operator(value, boxed_key)),
                    0
                );
            }
            crate::object::js_object_set_field_by_name(obj, key, undefined);
            for value in [receiver, child_value] {
                assert_ne!(
                    crate::value::js_is_truthy(js_in_operator(value, boxed_key)),
                    0
                );
            }
        }
    }

    #[test]
    fn implicit_prototype_presence_does_not_read_undefined_or_invoke_getters() {
        let _lock = crate::gc::global_side_table_test_lock();
        let _no_gc = crate::gc::GcSuppressScope::new();
        let proto = crate::object::ensure_object_intrinsics().1;
        let obj = crate::object::object_alloc_plain(0);
        let key = crate::string::js_string_from_bytes(b"presence10497".as_ptr(), 13);
        let receiver = crate::value::js_nanbox_pointer(obj as i64);
        let boxed_key = crate::value::js_nanbox_string(key as i64);
        crate::object::js_object_set_field_by_name(
            proto,
            key,
            f64::from_bits(crate::value::TAG_UNDEFINED),
        );
        assert_ne!(
            crate::value::js_is_truthy(js_object_has_property(receiver, boxed_key)),
            0
        );

        static GETS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        extern "C" fn getter(
            _closure: *const crate::closure::ClosureHeader,
            _this: crate::closure::JsThis,
        ) -> f64 {
            GETS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            f64::from_bits(crate::value::TAG_UNDEFINED)
        }
        let get = crate::closure::js_closure_alloc(crate::fn_info!(getter, 0), 0);
        crate::object::set_builtin_accessor_descriptor(
            proto as usize,
            "presence10497".to_string(),
            crate::object::AccessorDescriptor {
                get: crate::value::js_nanbox_pointer(get as i64).to_bits(),
                set: 0,
            },
            crate::object::PropertyAttrs::new(true, false, true),
        );
        GETS.store(0, std::sync::atomic::Ordering::Relaxed);
        assert_ne!(
            crate::value::js_is_truthy(js_in_operator(receiver, boxed_key)),
            0
        );
        assert_eq!(GETS.load(std::sync::atomic::Ordering::Relaxed), 0);
        crate::object::js_object_delete_field(proto, key);
        assert_eq!(
            crate::value::js_is_truthy(js_object_has_property(receiver, boxed_key)),
            0
        );
    }
}
