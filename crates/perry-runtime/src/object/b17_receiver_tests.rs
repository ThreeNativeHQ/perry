use super::*;
unsafe extern "C-unwind" {
    #[link_name = "js_typed_feedback_native_call_method_by_id"]
    fn feedback_by_id(site: u64, receiver: f64, method: i64, args: *const f64, len: usize) -> f64;
}

fn probes() -> (u64, u64, u64, u64) {
    (
        crate::buffer::test_buffer_bitmap_probe_count(),
        crate::buffer::test_buffer_registry_probe_count(),
        crate::typedarray::test_typed_array_registry_probe_count(),
        crate::array::test_element_accessor_calls(),
    )
}

extern "C" fn seven(_closure: *const crate::closure::ClosureHeader) -> f64 {
    7.0
}

fn receiver(class_id: u32) -> f64 {
    // Populate both sides of the ordinary receiver with actual registered views.
    crate::buffer::js_buffer_alloc(8, 0);
    crate::typedarray::typed_array_alloc(crate::typedarray::KIND_FLOAT32, 8);
    let obj = js_object_alloc(class_id, 8);
    let scope = crate::gc::RuntimeHandleScope::new();
    let obj = scope.root_raw_mut_ptr(obj);
    let closure = scope.root_raw_mut_ptr(crate::closure::js_closure_alloc(seven as *const u8, 0));
    crate::closure::js_register_closure_arity(seven as *const u8, 0);
    for name in ["step", "then", "catch", "finally"] {
        let key = scope.root_string_ptr(crate::string::js_string_from_str(name));
        obj.with_mut_ptr(|ptr| {
            key.with_const_ptr(|key| {
                closure.with_mut_ptr(|closure: *mut crate::closure::ClosureHeader| {
                    js_object_set_field_by_name(
                        ptr,
                        key,
                        crate::value::js_nanbox_pointer(closure as i64),
                    );
                })
            })
        });
    }
    crate::buffer::js_buffer_alloc(8, 0);
    crate::typedarray::typed_array_alloc(crate::typedarray::KIND_FLOAT32, 8);
    obj.with_mut_ptr(|ptr: *mut ObjectHeader| crate::value::js_nanbox_pointer(ptr as i64))
}

#[test]
fn b17_private_guard_ordinary_receiver_skips_probes() {
    let obj = receiver(171701);
    let key = crate::string::js_string_from_str("#value");
    js_private_field_add(obj, 171701, crate::value::js_nanbox_string(key as i64), 7.0);
    let before = probes();
    assert_eq!(
        js_private_guard(
            obj,
            f64::from_bits(crate::value::TAG_UNDEFINED),
            171701,
            b"#value".as_ptr(),
            6,
            0,
            0
        )
        .to_bits(),
        obj.to_bits()
    );
    assert_eq!(
        probes(),
        before,
        "ordinary private read must skip bitmap, registries and key-array accessors"
    );
}

#[test]
fn b17_native_by_id_ordinary_receiver_skips_probes() {
    unsafe {
        for class_id in [171702, 0] {
            let obj = receiver(class_id);
            for obj in [
                obj,
                f64::from_bits(obj.to_bits() & crate::value::POINTER_MASK),
            ] {
                for method in ["step", "then", "catch", "finally"] {
                    let name = crate::string::js_string_from_str(method);
                    let before = probes();
                    assert_eq!(
                        js_native_call_method_by_id(obj, name as i64, std::ptr::null(), 0),
                        7.0
                    );
                    assert_eq!(
                        probes(),
                        before,
                        "ordinary dispatch must skip bitmap, registries and key-array accessors"
                    );
                }
            }
        }
    }
}

#[test]
fn b17_typed_feedback_by_id_ordinary_receiver_skips_probes() {
    unsafe {
        for class_id in [0, 171704] {
            let obj = receiver(class_id);
            for obj in [
                obj,
                f64::from_bits(obj.to_bits() & crate::value::POINTER_MASK),
            ] {
                for method in ["step", "then", "catch", "finally"] {
                    let name = crate::string::js_string_from_str(method);
                    let before = probes();
                    assert_eq!(
                        feedback_by_id(0, obj, name as i64, std::ptr::null(), 0),
                        7.0
                    );
                    assert_eq!(
                    probes(),
                    before,
                    "ordinary feedback dispatch must skip bitmap, registries and key-array accessors"
                );
                }
            }
        }
    }
}

#[test]
fn b17_registration_exceptions_keep_the_slow_answer() {
    let obj = js_object_alloc(171703, 8) as usize;
    // Stale bitmap bits must not suppress registration-exception admission.
    crate::arena::record_arena_object_start(
        obj - crate::gc::GC_HEADER_SIZE,
        crate::gc::GC_TYPE_BUFFER,
    );
    // The public registration APIs deliberately accept non-buffer GC storage.
    crate::buffer::register_buffer(obj as *const crate::buffer::BufferHeader);
    assert!(crate::buffer::is_registered_buffer(obj));
    assert!(!unsafe { crate::value::addr_class::ordinary_object_without_view_brand(obj) });
    crate::typedarray::register_typed_array(
        obj as *const crate::typedarray::TypedArrayHeader,
        crate::typedarray::KIND_FLOAT32,
    );
    assert_eq!(
        crate::typedarray::lookup_typed_array_kind(obj),
        Some(crate::typedarray::KIND_FLOAT32)
    );
    crate::typedarray::unregister_typed_array(obj as *const crate::typedarray::TypedArrayHeader);
    crate::buffer::finalize_collected_dead_buffer(obj);
    let buffer = crate::buffer::js_buffer_alloc(8, 0) as usize;
    let typed = crate::typedarray::typed_array_alloc(crate::typedarray::KIND_FLOAT32, 8) as usize;
    assert!(crate::buffer::is_registered_buffer(buffer));
    assert_eq!(
        crate::typedarray::lookup_typed_array_kind(typed),
        Some(crate::typedarray::KIND_FLOAT32)
    );
    let sab = crate::shared_sab::alloc_shared_sab(8) as usize;
    assert!(crate::buffer::is_registered_buffer(sab));
}
