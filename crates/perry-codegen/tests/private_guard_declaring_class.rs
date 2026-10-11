//! Regression: a private-member brand guard must use the DECLARING CLASS'S
//! UNIQUE id, not a name lookup.
//!
//! `js_private_guard` brand-checks the receiver against a declaring class id.
//! Codegen used to derive that id by resolving `Expr::PrivateGuard.class_name`
//! through `class_ids` — a `name -> id` map built by
//! `hir.classes.map(|c| (c.name, c.id)).collect()`, i.e. last-writer-wins.
//! Minified programs reuse class names, so two distinct classes with the same
//! name collapse to one entry: a `this.#x` inside the class that LOSES the
//! collision then brand-checks against the OTHER same-named class, whose id is
//! not in the receiver's class-id chain, and throws
//!   `Cannot access private member from an object whose class did not declare it`
//! for a legal access.
//!
//! A faithful end-to-end repro needs the specific duplicate-name shape a
//! minifier produces, so this asserts the codegen contract directly on the
//! emitted IR: `Expr::PrivateGuard` now carries the declaring class's unique
//! id, and codegen passes THAT to `js_private_guard` rather than the collided
//! `class_ids[name]`.

use perry_codegen::{compile_module, CompileOptions};
use perry_hir::types::Type;
use perry_hir::{Class, Expr, Function, Module, ModuleInitKind, Stmt};

fn ir_opts() -> CompileOptions {
    CompileOptions {
        is_entry_module: true,
        emit_ir_only: true,
        output_type: "executable".to_string(),
        ..Default::default()
    }
}

fn class(id: u32, name: &str) -> Class {
    Class {
        id,
        name: name.to_string(),
        type_params: Vec::new(),
        extends: None,
        extends_name: None,
        native_extends: None,
        extends_expr: None,
        heritage_lexically_shadowed: false,
        fields: Vec::new(),
        constructor: None,
        methods: Vec::new(),
        getters: Vec::new(),
        setters: Vec::new(),
        static_accessor_names: Vec::new(),
        static_accessor_fn_ids: Vec::new(),
        computed_members: Vec::new(),
        static_fields: Vec::new(),
        static_methods: Vec::new(),
        decorators: Vec::new(),
        is_exported: false,
        aliases: Vec::new(),
        is_nested: false,
        alloc_width_hint: 0,
        specialized_from: None,
    }
}

fn module_with(classes: Vec<Class>, body: Vec<Stmt>) -> Module {
    Module {
        name: "private_guard_declaring_class.ts".to_string(),
        imports: Vec::new(),
        exports: Vec::new(),
        classes,
        interfaces: Vec::new(),
        type_aliases: Vec::new(),
        enums: Vec::new(),
        globals: Vec::new(),
        functions: vec![Function {
            id: 1,
            name: "probe".to_string(),
            type_params: Vec::new(),
            params: Vec::new(),
            return_type: Type::Number,
            body,
            is_async: false,
            is_generator: false,
            is_strict: false,
            is_exported: false,
            captures: Vec::new(),
            decorators: Vec::new(),
            was_plain_async: false,
            was_unrolled: false,
        }],
        init_is_strict: false,
        init: Vec::new(),
        classic_for_lexical_bindings: std::collections::HashSet::new(),
        exported_native_instances: Vec::new(),
        exported_func_return_native_instances: Vec::new(),
        exported_objects: Vec::new(),
        exported_functions: Vec::new(),
        script_global_functions: Vec::new(),
        references_global_this: false,
        annexb_global_undefined_names: Vec::new(),
        widgets: Vec::new(),
        uses_fetch: false,
        uses_webassembly: false,
        extern_funcs: Vec::new(),
        init_was_unrolled: false,
        has_top_level_await: false,
        init_kind: ModuleInitKind::Eager,
        async_step_closures: std::collections::HashSet::new(),
        closure_display_names: std::collections::HashMap::new(),
        class_display_names: std::collections::HashMap::new(),
        closure_source_text: std::collections::HashMap::new(),
        class_source_text: std::collections::HashMap::new(),
        async_generator_funcs: std::collections::HashSet::new(),
        local_source_spans: std::collections::HashMap::new(),
        gen_param_prologue_len: std::collections::HashMap::new(),
    }
}

#[test]
fn private_guard_uses_declaring_class_id_not_name_collided_id() {
    // Two DISTINCT classes both named "Box": ids 5 and 7. `class_ids` is a
    // name-keyed map, so "Box" collapses to a single entry (id 7 — the last
    // one collected). The guard below declares class id 5 (the OTHER "Box").
    let classes = vec![class(5, "Box"), class(7, "Box")];
    let guard = Expr::PrivateGuard {
        class_name: "Box".to_string(),
        class_id: 5,
        field_name: "#v".to_string(),
        kind: 0, // field
        op: 0,   // instance read
        // Not a named class expression's self-binding: this guard's receiver
        // is a plain expression, so the declaring class id stays the brand
        // owner — which is exactly what this test pins.
        receiver_is_brand_owner: false,
        object: Box::new(Expr::Integer(0)),
    };
    let ir = String::from_utf8(
        compile_module(&module_with(classes, vec![Stmt::Expr(guard)]), ir_opts()).unwrap(),
    )
    .expect("LLVM IR should be UTF-8");

    let call = ir
        .lines()
        .find(|l| l.contains("call") && l.contains("@js_private_guard"))
        .unwrap_or_else(|| panic!("no js_private_guard CALL emitted:\n{ir}"));

    // The brand must use the declaring class's own id (5), carried on the
    // PrivateGuard node — NOT `class_ids["Box"]` (7), which resolves to the
    // wrong same-named class.
    assert!(
        call.contains("i32 5"),
        "brand must use the declaring class id 5 carried on the node:\n{call}"
    );
    assert!(
        !call.contains("i32 7"),
        "brand must NOT use the name-collided class_ids[\"Box\"] id 7:\n{call}"
    );
}

#[test]
fn ordinary_private_field_uses_guarded_fixed_slot_after_public_fields() {
    let mut box_class = class(5, "Box");
    for (name, is_private) in [("#v", true), ("public", false)] {
        box_class.fields.push(perry_hir::ClassField {
            name: name.to_string(),
            key_expr: None,
            ty: Type::Number,
            init: Some(Expr::Integer(1)),
            is_private,
            is_readonly: false,
            decorators: Vec::new(),
        });
    }
    let guard = |op| Expr::PrivateGuard {
        class_name: "Box".to_string(),
        class_id: 5,
        field_name: "#v".to_string(),
        kind: 0,
        op,
        receiver_is_brand_owner: false,
        object: Box::new(Expr::This),
    };
    let body = vec![
        Stmt::Expr(Expr::PropertyGet {
            object: Box::new(guard(0)),
            property: "#<perry:private-value:5:#v>".to_string(),
            byte_offset: 0,
        }),
        Stmt::Expr(Expr::PropertySet {
            object: Box::new(guard(1)),
            property: "#<perry:private-value:5:#v>".to_string(),
            value: Box::new(Expr::Integer(2)),
        }),
    ];
    // Field owners often mix methods/accessors; their marker must be reserved too.
    let mut private_method = module_with(Vec::new(), Vec::new()).functions.remove(0);
    private_method.id = 44;
    private_method.name = "#touch".to_string();
    private_method.body = vec![Stmt::Return(Some(Expr::Integer(0)))];
    box_class.methods.push(private_method.clone());
    private_method.id = 45;
    private_method.name = "#current".to_string();
    box_class
        .getters
        .push(("#current".to_string(), private_method));
    let ir =
        String::from_utf8(compile_module(&module_with(vec![box_class], body), ir_opts()).unwrap())
            .unwrap();
    assert!(
        ir.contains("#<perry:private-brand:5>"),
        "mixed private owner must reserve its existing brand marker"
    );
    let calls: Vec<_> = ir
        .lines()
        .filter(|line| line.contains("call") && line.contains("@js_private_slot_admit"))
        .collect();
    assert!(
        calls.len() >= 2,
        "ordinary private reads and writes need fixed-slot admission:\n{ir}"
    );
    assert!(
        calls
            .iter()
            .all(|line| line.contains("i32 5") && line.contains("i32 1")),
        "declaring identity and exact presence slot must be passed: {calls:?}"
    );
    assert!(
        ir.contains("private_field_get.fast") && ir.contains("private_field_set.fast"),
        "fixed-slot arms missing:\n{ir}"
    );
    assert!(
        ir.contains("@js_private_guard"),
        "cold fallback must retain the semantic guard"
    );
}

#[test]
fn imported_private_keys_match_source_and_legacy_identity_fails_closed() {
    use perry_codegen::ImportedClass;
    let import = |identified: bool| ImportedClass {
        name: "Box".into(),
        local_alias: Some("ImportedBox".into()),
        namespace: None,
        source_prefix: "producer".into(),
        constructor_param_count: 0,
        has_own_constructor: false,
        constructor_has_rest: false,
        has_instance_fields: true,
        method_names: vec!["#touch".into()],
        proven_this_method_names: vec![],
        proven_this_tower_method_names: vec![],
        method_return_types: vec![Type::Number],
        method_param_counts: vec![0],
        method_has_rest: vec![false],
        method_has_synthetic_arguments: vec![false],
        method_arguments_length_only: vec![false],
        static_field_names: vec![],
        static_method_names: vec![],
        static_method_return_types: vec![],
        static_method_param_counts: vec![],
        static_method_has_rest: vec![],
        static_method_has_user_rest: vec![],
        static_method_has_synthetic_arguments: vec![],
        getter_names: vec![],
        getter_return_types: vec![],
        setter_names: vec![],
        parent_name: None,
        field_names: vec![
            if identified {
                perry_codegen::private_field_metadata_name(5, "#v")
            } else {
                "#v".into()
            },
            "public".into(),
        ],
        field_types: vec![Type::Number, Type::Number],
        source_class_id: identified.then_some(5),
        return_shape_imports: vec![],
        object_literal: None,
    };
    let mut source = class(5, "Box");
    for (name, is_private) in [("#v", true), ("public", false)] {
        source.fields.push(perry_hir::ClassField {
            name: name.into(),
            key_expr: None,
            ty: Type::Number,
            init: Some(Expr::Integer(1)),
            is_private,
            is_readonly: false,
            decorators: vec![],
        });
    }
    let mut method = module_with(vec![], vec![]).functions.remove(0);
    method.id = 66;
    method.name = "#touch".into();
    method.body = vec![Stmt::Return(Some(Expr::Integer(0)))];
    source.methods.push(method);
    let producer =
        String::from_utf8(compile_module(&module_with(vec![source], vec![]), ir_opts()).unwrap())
            .unwrap();
    let consumer = |identified| {
        let mut options = ir_opts();
        options.imported_classes.push(import(identified));
        String::from_utf8(compile_module(&module_with(vec![], vec![]), options).unwrap()).unwrap()
    };
    let identified = consumer(true);
    let packed = |ir: &str| {
        ir.lines()
            .find(|line| {
                line.contains(" c\"")
                    && line.contains("#<perry:private-field:5:#v>")
                    && line.contains("#<perry:private-brand:5>")
            })
            .expect("canonical packed public + private keys")
            .split(" c\"")
            .nth(1)
            .unwrap()
            .to_owned()
    };
    assert_eq!(
        packed(&producer),
        packed(&identified),
        "all allocations of the authored class must share exact keys and declaring ids"
    );
    let legacy = consumer(false);
    assert!(
        !legacy.contains("#<perry:private-field:"),
        "missing identity must not guess private slots"
    );
    assert!(
        !legacy.contains("#<perry:private-brand:"),
        "missing method identity must retain generic initialization"
    );
}

#[test]
fn private_slots_refuse_dynamic_ancestor_even_for_known_leaf() {
    let mut parent = class(8, "DynamicParent");
    parent.extends_expr = Some(Box::new(Expr::Integer(0)));
    let mut leaf = class(9, "Leaf");
    leaf.extends_name = Some("DynamicParent".into());
    leaf.fields.push(perry_hir::ClassField {
        name: "#v".into(),
        key_expr: None,
        ty: Type::Number,
        init: Some(Expr::Integer(1)),
        is_private: true,
        is_readonly: false,
        decorators: vec![],
    });
    let read = Expr::PropertyGet {
        object: Box::new(Expr::PrivateGuard {
            class_name: "Leaf".into(),
            class_id: 9,
            field_name: "#v".into(),
            kind: 0,
            op: 0,
            receiver_is_brand_owner: false,
            object: Box::new(Expr::This),
        }),
        property: "#<perry:private-value:9:#v>".into(),
        byte_offset: 0,
    };
    let ir = String::from_utf8(
        compile_module(
            &module_with(vec![parent, leaf], vec![Stmt::Expr(read)]),
            ir_opts(),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        !ir.lines()
            .any(|line| line.contains("call") && line.contains("@js_private_slot_admit")),
        "known leaf must retain generic semantics for dynamic ancestry"
    );
}
