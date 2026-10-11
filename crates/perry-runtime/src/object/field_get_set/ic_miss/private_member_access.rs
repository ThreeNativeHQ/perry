fn private_brand_key(declaring_class_id: u32) -> String {
    format!("#<perry:private-brand:{declaring_class_id}>")
}

crate::perry_thread_local! {
    static PRIVATE_METHOD_OWNER_HINT: std::cell::RefCell<Option<(u32, String)>> =
        std::cell::RefCell::new(None);
    static PRIVATE_MEMBER_ACCESS_HINTS: std::cell::RefCell<Vec<PrivateMemberAccessHint>> =
        std::cell::RefCell::new(Vec::new());
}

#[derive(Clone)]
struct PrivateMemberAccessHint {
    class_id: u32,
    name: String,
    kind: u32,
    is_static: bool,
    is_write: bool,
}

pub(crate) fn private_member_access_hints_savepoint() -> usize {
    PRIVATE_MEMBER_ACCESS_HINTS.with(|hints| hints.borrow().len())
}

pub(crate) fn private_member_access_hints_restore(depth: usize) {
    PRIVATE_MEMBER_ACCESS_HINTS.with(|hints| hints.borrow_mut().truncate(depth));
}

pub(crate) fn take_private_method_owner_hint(method_name: &str) -> Option<u32> {
    PRIVATE_METHOD_OWNER_HINT.with(|hint| {
        let mut hint = hint.borrow_mut();
        match hint.as_ref() {
            Some((class_id, name)) if name == method_name => {
                let class_id = *class_id;
                *hint = None;
                Some(class_id)
            }
            _ => None,
        }
    })
}

/// The prefix every private class member's storage name carries.
const PRIVATE_MEMBER_PREFIX: &str = "#<perry:private-member:";

/// Cheap rejection for the overwhelmingly common case: an ordinary property
/// name is not a private-member storage name.
///
/// Callers invoke this at THEIR OWN call site, before calling into the
/// private-member helpers, so an ordinary property operation makes no call at
/// all. Folding the guard inside the helpers (as this originally did) made the
/// work cheap but left the call: `private_member_get_by_name` was still 16.8%
/// of a pure property-read loop, essentially all of it call overhead for keys
/// that are rejected on their length.
///
/// [`private_member_storage_name`] runs at the TOP of both the generic
/// property read (`js_object_get_field_by_name`) and the generic write
/// (`field_set_by_name`), so every property operation in the program pays it.
/// It reaches that verdict via `str_from_string_header`, which UTF-8-validates
/// the WHOLE key before the prefix compare can reject it — measured at ~7.5%
/// self time in a computed-key read loop, plus its share of `from_utf8`, all
/// of it spent proving that `"k123"` does not begin with `#`.
///
/// A storage name always starts with `#` and is at least `PRIVATE_MEMBER_PREFIX`
/// long, so a length compare and one byte settle it for every ordinary key
/// without validating anything. Only keys that pass this filter — private
/// members and the rare `#`-prefixed user key — go on to the real check, so
/// the slow path's behaviour is unchanged.
#[inline(always)]
pub(crate) fn cannot_be_private_member_name(key: *const crate::StringHeader) -> bool {
    if key.is_null() {
        return true;
    }
    // SAFETY: same reads `string_header_as_str` already performs on this
    // pointer (null-checked header, then its payload); no new dereference.
    unsafe {
        if (*key).byte_len as usize <= PRIVATE_MEMBER_PREFIX.len() {
            return true;
        }
        *crate::object::string_header_payload(key) != b'#'
    }
}

fn private_member_storage_name(key: *const crate::StringHeader) -> Option<String> {
    if cannot_be_private_member_name(key) {
        return None;
    }
    let key = unsafe { super::super::has_own_helpers::str_from_string_header(key) }?;
    private_member_storage_name_str(key).map(str::to_string)
}

fn private_member_storage_name_str(key: &str) -> Option<&str> {
    let rest = key.strip_prefix(PRIVATE_MEMBER_PREFIX)?;
    let (_, name) = rest.split_once(':')?;
    name.strip_suffix('>')
}

fn take_private_member_access_hint(name: &str, is_write: bool) -> Option<PrivateMemberAccessHint> {
    PRIVATE_MEMBER_ACCESS_HINTS.with(|hints| {
        let mut hints = hints.borrow_mut();
        let index = hints
            .iter()
            .rposition(|hint| hint.name == name && hint.is_write == is_write)?;
        Some(hints.remove(index))
    })
}

fn private_member_receiver(obj: *const ObjectHeader) -> f64 {
    let bits = obj as u64;
    if matches!(bits >> 48, 0x7FFD | 0x7FFE) {
        f64::from_bits(bits)
    } else {
        f64::from_bits(crate::value::js_nanbox_pointer(obj as i64).to_bits())
    }
}

pub(crate) fn private_member_get_by_name(
    obj: *const ObjectHeader,
    key: *const crate::StringHeader,
) -> Option<f64> {
    let name = private_member_storage_name(key)?;
    let hint = take_private_member_access_hint(&name, false)?;
    let receiver = private_member_receiver(obj);
    unsafe {
        match hint.kind {
            1 => {
                if hint.is_static {
                    let _ = take_private_method_owner_hint(&name);
                    let brand = current_private_lexical_brand(hint.class_id)
                        .map(f64::from_bits)
                        .or_else(|| {
                            private_evaluation_brand(receiver, hint.class_id).map(f64::from_bits)
                        })
                        .unwrap_or(receiver);
                    return Some(
                        super::super::native_module::class_private_static_method_value_for_name(
                            hint.class_id,
                            &name,
                            brand,
                        ),
                    );
                }
                let stable_name = super::super::native_module::intern_class_method_name(
                    hint.class_id,
                    &name,
                );
                Some(super::super::js_class_method_bind(
                    receiver,
                    stable_name.as_ptr(),
                    stable_name.len(),
                ))
            }
            2 | 4 if hint.is_static => {
                super::super::class_registry::class_static_accessor_getter_value(
                    hint.class_id,
                    &name,
                    receiver,
                )
            }
            2 | 4 => super::super::class_registry::class_private_instance_getter_value(
                hint.class_id,
                &name,
                receiver,
            ),
            _ => None,
        }
    }
}

/// Invoke a guarded private method from the fused `receiver.#method(args)`
/// lowering. The ordinary dynamic-method tower cannot resolve the internal
/// storage key, while the preceding guard has already validated its brand.
pub(crate) unsafe fn private_member_call_by_name(
    receiver: f64,
    storage_name: &str,
    args_ptr: *const f64,
    args_len: usize,
) -> Option<f64> {
    let (class_id, is_static, name) = take_private_method_call_hint(storage_name)?;

    let scope = crate::gc::RuntimeHandleScope::new();
    let receiver = scope.root_nanbox_f64(receiver);
    if is_static {
        return Some(super::super::class_registry::js_class_static_method_call(
            receiver.get_nanbox_f64(),
            name.as_ptr(),
            name.len(),
            args_ptr,
            args_len,
        ));
    }

    let (func_ptr, param_count, has_synthetic_arguments, has_rest) =
        super::super::class_registry::lookup_class_method_in_chain(class_id, name)?;
    let receiver_value = receiver.get_nanbox_f64();
    let private_brand = current_private_lexical_brand(class_id)
        .map(f64::from_bits)
        .or_else(|| private_evaluation_brand_value(receiver_value))
        .unwrap_or_else(|| f64::from_bits(crate::value::TAG_UNDEFINED));
    let this_bits = receiver_value.to_bits();
    let this = if (this_bits >> 48) == 0x7FFD {
        (this_bits & crate::value::POINTER_MASK) as i64
    } else {
        this_bits as i64
    };
    Some(
        super::super::class_registry::call_vtable_method_with_private_brand(
            func_ptr,
            this,
            args_ptr,
            args_len,
            param_count,
            has_synthetic_arguments,
            has_rest,
            private_brand,
        ),
    )
}

pub(crate) fn take_private_method_call_hint(storage_name: &str) -> Option<(u32, bool, &str)> {
    let name = private_member_storage_name_str(storage_name)?;
    let hint = take_private_member_access_hint(name, false)?;
    if hint.kind != 1 {
        return None;
    }
    let _ = take_private_method_owner_hint(name);
    Some((hint.class_id, hint.is_static, name))
}

pub(crate) fn private_member_set_by_name(
    obj: *mut ObjectHeader,
    key: *const crate::StringHeader,
    value: f64,
) -> bool {
    let Some(name) = private_member_storage_name(key) else {
        return false;
    };
    let Some(hint) = take_private_member_access_hint(&name, true) else {
        return false;
    };
    let receiver = private_member_receiver(obj);
    let applied = unsafe {
        if hint.is_static {
            super::super::class_registry::class_static_accessor_setter_apply(
                hint.class_id,
                &name,
                receiver,
                value,
            )
        } else {
            super::super::class_registry::class_private_instance_setter_apply(
                hint.class_id,
                &name,
                receiver,
                value,
            )
        }
    };
    if !applied {
        throw_private_type_error("Private setter is unavailable");
    }
    true
}

/// If the lexical class evaluation can be recovered from `brand_owner`,
/// compare `obj` against that exact evaluation. `None` asks callers to retain
/// the existing template-class check for ordinary (single-evaluation) classes.
fn private_evaluation_brand_matches(
    obj: f64,
    brand_owner: f64,
    declaring_class_id: u32,
) -> Option<bool> {
    // A bare class REF receiver names the class's lexical self-binding
    // (`class c { static create() { c.#o = ... } }` — the lru-cache guard
    // pattern, minified into pi's bundle). Codegen hands the runtime the
    // template ref, which carries no per-evaluation brand, so both brand
    // paths below compared None against Some(_) and every closure-nested
    // class expression's static private access threw "did not declare it".
    // A private access with the ref as receiver can only be emitted from
    // inside the class's own body (outside it, `c.#o` is a syntax error),
    // where the self-binding denotes the CURRENT evaluation — the exact
    // verdict the pre-brand static fallback (`class_ref_id(obj) == id`)
    // always gave.
    if crate::object::native_module::class_ref_id(obj) == Some(declaring_class_id) {
        return Some(true);
    }

    if let Some(expected) = current_private_lexical_brand(declaring_class_id) {
        let actual = private_evaluation_brand(obj, declaring_class_id);
        return Some(actual == Some(expected));
    }

    let brand_owner = if super::super::class_registry::is_class_object_value(brand_owner) {
        let captured_owner = super::super::static_private_owner_current().unwrap_or(brand_owner);
        if super::super::class_registry::is_class_object_value(captured_owner)
            && private_evaluation_brand(captured_owner, declaring_class_id).is_some()
        {
            captured_owner
        } else {
            brand_owner
        }
    } else {
        brand_owner
    };
    let expected = private_evaluation_brand(brand_owner, declaring_class_id)?;
    Some(private_evaluation_brand(obj, declaring_class_id) == Some(expected))
}

#[no_mangle]
pub extern "C" fn js_private_brand_check(
    obj: f64,
    brand_owner: f64,
    declaring_class_id: u32,
    field_name_ptr: *const u8,
    field_name_len: u32,
    kind: u32,
    is_static: u32,
) -> f64 {
    let false_value = f64::from_bits(crate::value::TAG_FALSE);
    let true_value = f64::from_bits(crate::value::TAG_TRUE);
    if declaring_class_id == 0 || field_name_ptr.is_null() || field_name_len == 0 {
        return false_value;
    }
    if is_static != 0 && crate::proxy::js_proxy_is_proxy(obj) != 0 {
        return false_value;
    }

    let has_declaring_brand =
        private_evaluation_brand_matches(obj, brand_owner, declaring_class_id).unwrap_or_else(
            || {
                if is_static != 0 {
                    super::super::class_ref_id(obj) == Some(declaring_class_id)
                } else {
                    private_instance_element_is_present(
                        crate::proxy::private_element_receiver(obj),
                        declaring_class_id,
                        field_name_ptr,
                        field_name_len,
                        kind,
                    )
                }
            },
        );
    if !has_declaring_brand {
        return false_value;
    }

    if is_static == 0 {
        let storage = crate::proxy::private_element_receiver(obj);
        if !private_instance_element_is_present(
            storage,
            declaring_class_id,
            field_name_ptr,
            field_name_len,
            kind,
        ) {
            return false_value;
        }
    }

    true_value
}

/// Throw a `TypeError` with `msg` through Perry's exception machinery so a
/// surrounding `try { ... } catch (e) { ... }` catches it. Diverges.
fn throw_private_type_error(msg: &str) -> ! {
    let scope = crate::gc::RuntimeHandleScope::new();
    let s = scope.root_string_ptr(crate::string::js_string_from_bytes(
        msg.as_ptr(),
        msg.len() as u32,
    ));
    let err = s.with_mut_ptr::<crate::StringHeader, _>(|s| crate::error::js_typeerror_new(s));
    let v = crate::value::JSValue::pointer(err as *const u8).bits();
    crate::exception::js_throw(f64::from_bits(v))
}

/// Admit only an initialized private field in the compiler's exact canonical
/// ordinary shape. This call cannot allocate or forward the receiver.
#[no_mangle]
pub extern "C" fn js_private_slot_admit(
    receiver: f64,
    brand_owner: f64,
    declaring_class_id: u32,
    concrete_class_id: u32,
    expected_shape_id: u32,
    presence_slot: u32,
) -> i64 {
    let value = JSValue::from_bits(receiver.to_bits());
    if !value.is_pointer() || declaring_class_id == 0 || expected_shape_id == 0 {
        return 0;
    }
    let raw = (receiver.to_bits() & crate::value::POINTER_MASK) as usize;
    if !crate::value::addr_class::is_plausible_heap_addr(raw) {
        return 0;
    }
    // B17 exception windows reject foreign registered storage before any header read.
    // Debug proof functions assert an already-proven ordinary receiver, so use
    // the authoritative registries until the ordinary header proof below.
    #[cfg(debug_assertions)]
    if crate::buffer::is_registered_buffer(raw)
        || crate::typedarray::lookup_typed_array_kind(raw).is_some()
    {
        return 0;
    }
    #[cfg(not(debug_assertions))]
    if !crate::buffer::ordinary_object_has_no_buffer_brand(raw)
        || !crate::typedarray::ordinary_object_has_no_typed_array_brand(raw)
    {
        return 0;
    }
    unsafe {
        if !crate::value::addr_class::ordinary_object_without_view_brand(raw) {
            return 0;
        }
        let Some(header) = crate::value::addr_class::try_read_gc_header(raw) else {
            return 0;
        };
        if header.obj_type != crate::gc::GC_TYPE_OBJECT
            || header.gc_flags & crate::gc::GC_FLAG_FORWARDED != 0
        {
            return 0;
        }
        let object = raw as *const ObjectHeader;
        if (*object).class_id != concrete_class_id
            || crate::object::shapes::object_shape_stamp(object) != expected_shape_id
        {
            return 0;
        }
        let Some(shape) = crate::object::shapes::object_shape_descriptor(object) else {
            return 0;
        };
        if shape.object_kind != crate::object::shapes::ShapeObjectKind::Ordinary
            || shape.semantic_generation != 0
            || shape.keys != crate::object::object_keys_array(object) as u64
            || presence_slot.checked_add(1).is_none_or(|slot| {
                slot >= shape.logical_key_count
                    || slot >= shape.live_inline_slot_count
                    || slot >= crate::object::object_live_slot_count(object)
            })
        {
            return 0;
        }
        let slots = (object as *const u8).add(std::mem::size_of::<ObjectHeader>()) as *const u64;
        if *slots.add(presence_slot as usize) != crate::value::TAG_TRUE {
            return 0;
        }
        // Template identity alone never admits an uninitialized field.
        if !private_evaluation_brand_matches(receiver, brand_owner, declaring_class_id)
            .unwrap_or(true)
        {
            return 0;
        }
        raw as i64
    }
}

/// The semantic guard has already proved the private element. Ordinary
/// property attributes (including freeze) do not govern its hidden value.
#[no_mangle]
pub extern "C" fn js_private_field_set_by_property_id(receiver: f64, property_id: i64, value: f64) {
    let scope = crate::gc::RuntimeHandleScope::new();
    let receiver = scope.root_nanbox_f64(receiver);
    let value = scope.root_nanbox_f64(value);
    let mut scratch = [0u8; crate::value::SHORT_STRING_MAX_LEN];
    let Some(key_ref) = crate::string::perry_string_ref_from_dispatch_id(property_id, &mut scratch)
    else {
        throw_private_type_error("Invalid private storage key");
    };
    let key = scope.root_string_ptr(crate::string::materialize_dispatch_key(key_ref));
    let object = JSValue::from_bits(receiver.get_nanbox_f64().to_bits())
        .as_pointer::<ObjectHeader>() as *mut ObjectHeader;
    key.with_const_ptr::<crate::StringHeader, _>(|key| unsafe {
        let raw = object as usize;
        // Stamped arrays/functions and foreign buffers/views keep generic storage.
        // Reject registered foreign storage before touching a possible GC header.
        let ordinary = !crate::buffer::is_registered_buffer(raw)
            && crate::typedarray::lookup_typed_array_kind(raw).is_none()
            && crate::value::addr_class::try_read_gc_header(raw).is_some_and(|header| {
                header.obj_type == crate::gc::GC_TYPE_OBJECT
                    && header.gc_flags & crate::gc::GC_FLAG_FORWARDED == 0
            });
        if !ordinary {
            super::js_object_set_field_by_name(object, key, value.get_nanbox_f64());
            return;
        }
        let keys = crate::object::object_keys_array(object);
        let count = crate::array::keys_array_len_capped_to_capacity(keys) as u32;
        let Some(slot) = crate::object::keys_find_slot_by_key_ptr(keys, count, key) else {
            throw_private_type_error("Private field storage is unavailable");
        };
        if slot < crate::object::object_live_slot_count(object) {
            crate::object::store_object_field_slot(
                object,
                slot as usize,
                value.get_nanbox_f64().to_bits(),
            );
        } else {
            crate::object::overflow_set(
                object as usize,
                slot as usize,
                value.get_nanbox_f64().to_bits(),
            );
        }
    });
}
