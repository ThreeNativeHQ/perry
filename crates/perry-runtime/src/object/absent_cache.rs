//! SPIKE: negative property-read cache for the inline-cache miss path.
//!
//! The PIC only primes positive own-slot hits, so `obj.absentKey` on an object
//! literal re-runs the whole generic ladder (own-key rescan, receiver-kind
//! probes, a rooted recursive lookup on `Object.prototype`) on every read.
//! This memoizes "the read of `key` on a receiver of ShapeId S answered
//! `undefined`", computed by the real lookup and guarded like
//! `promise::then_probe`'s `Object.prototype` verdict:
//!   * ShapeIds are never reused, so S pins the receiver's own keys;
//!   * the receiver's flags word, class id and per-instance prototype are
//!     re-checked on every hit;
//!   * `Object.prototype` is re-signed on every hit (address, keys array,
//!     flags, semantic epoch, vtable generation);
//!   * the key is compared by bytes, so GC moving the key string is harmless.
//! ponytail: direct-mapped 256 entries, keys <= 32 bytes; grow if games thrash it.

use crate::object::ObjectHeader;

const KEY_MAX: usize = 32;
const SIZE: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Sig {
    proto_addr: usize,
    keys_addr: usize,
    keys_len: u32,
    proto_flags: u16,
    proto_class: u32,
    epoch: u64,
    vtable_gen: u64,
}

#[derive(Clone, Copy)]
struct Entry {
    shape: u32,
    class_id: u32,
    flags: u16,
    key_len: u8,
    key: [u8; KEY_MAX],
    sig: Sig,
}

crate::perry_thread_local! {
    static TABLE: std::cell::UnsafeCell<Box<[Option<Entry>]>> =
        std::cell::UnsafeCell::new(vec![None; SIZE].into_boxed_slice());
}

fn current_sig() -> Option<Sig> {
    let proto_addr = crate::array::object_prototype_addr();
    if proto_addr == 0 {
        return None;
    }
    let (keys_addr, keys_len, proto_flags, proto_class) =
        unsafe { crate::promise::then_probe::proto_signature(proto_addr) }?;
    Some(Sig {
        proto_addr,
        keys_addr,
        keys_len,
        proto_flags,
        proto_class,
        epoch: crate::object::prop_plan::prop_plan_semantic_epoch(),
        vtable_gen: crate::object::vtable_generation(),
    })
}

unsafe fn key_bytes<'a>(key: *const crate::StringHeader) -> &'a [u8] {
    std::slice::from_raw_parts(crate::string::string_data(key), (*key).byte_len as usize)
}

fn slot(shape: u32, key: &[u8]) -> usize {
    let mut h = shape as u64 ^ ((key.len() as u64) << 32);
    for &b in key {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    (h.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as usize & (SIZE - 1)
}

/// Per-receiver state the cached answer depends on. `None` = do not cache.
unsafe fn receiver_state(obj: *const ObjectHeader) -> Option<(u32, u32, u16)> {
    let shape = crate::object::shapes::object_shape_stamp(obj);
    if shape == 0 || crate::object::prototype_chain::object_static_prototype(obj as usize).is_some()
    {
        return None;
    }
    let gc = (obj as *const u8).sub(crate::gc::GC_HEADER_SIZE) as *const crate::gc::GcHeader;
    Some((shape, (*obj).class_id, (*gc)._reserved))
}

/// Called by the IC miss handler after its own-key walk found no match on a
/// regular, descriptor-free receiver. Returns the read's value.
pub(crate) unsafe fn absent_read(
    obj: *const ObjectHeader,
    key: *const crate::StringHeader,
) -> f64 {
    let slow = |obj, key| {
        f64::from_bits(super::field_get_set::js_object_get_field_by_name(obj, key).bits())
    };
    let kb = key_bytes(key);
    let Some((shape, class_id, flags)) = receiver_state(obj) else {
        if std::env::var_os("SPIKE_DIAG").is_some() { eprintln!("absent diag: no receiver_state"); }
        return slow(obj, key);
    };
    if kb.len() > KEY_MAX {
        return slow(obj, key);
    }
    let Some(sig) = current_sig() else {
        if std::env::var_os("SPIKE_DIAG").is_some() { eprintln!("absent diag: no sig"); }
        return slow(obj, key);
    };
    let i = slot(shape, kb);
    let table = TABLE.with(|t| t.get());
    if let Some(e) = (*table)[i] {
        if e.shape == shape
            && e.class_id == class_id
            && e.flags == flags
            && e.sig == sig
            && &e.key[..e.key_len as usize] == kb
        {
            return f64::from_bits(crate::value::TAG_UNDEFINED);
        }
    }
    // Cold: ask the real lookup, then record only a plain `undefined` that no
    // accessor produced and that left the world unchanged.
    let cacheable = (class_id == 0 || super::is_anon_shape_class_id(class_id))
        && std::str::from_utf8(kb).is_ok_and(|name| {
            crate::object::descriptor_state::get_accessor_descriptor(sig.proto_addr, name)
                .is_none()
        });
    let mut key_copy = [0u8; KEY_MAX];
    key_copy[..kb.len()].copy_from_slice(kb);
    let key_len = kb.len() as u8;
    let value = slow(obj, key);
    if std::env::var_os("SPIKE_DIAG").is_some() {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if N.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 4 {
            eprintln!(
                "absent diag shape={shape} class={class_id} anon={} flags={flags:#x} cacheable={cacheable} undef={} sig_same={} hit_entry={}",
                super::is_anon_shape_class_id(class_id),
                value.to_bits() == crate::value::TAG_UNDEFINED,
                current_sig() == Some(sig),
                (*table)[i].is_some()
            );
        }
    }
    if cacheable
        && value.to_bits() == crate::value::TAG_UNDEFINED
        && current_sig() == Some(sig)
    {
        (*table)[i] = Some(Entry { shape, class_id, flags, key_len, key: key_copy, sig });
    }
    value
}
