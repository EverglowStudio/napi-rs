use napi_family_core::{HostFlavor, RuntimeEntrypoint};
use napi_uniffi_engine::{
  generate_napi_module, ArgumentBinding, ErrorBinding, ReturnBinding, RustArgumentPlan,
  RustBridgePlan, RustOperationPlan, RustOperationTarget, RustReceiverPlan, RustResourceHook,
  RustResourceHooks, BACKEND_FACTORY_EXPORT,
};
use proc_macro2::{Ident, Span};
use uniffi_js_abi::{
  ArgumentDefinition, AsyncKind, ComponentDefinition, ComponentId, ComponentKey, EnumVariant,
  FieldDefinition, IdentifiedComponent, IdentifiedOperation, IdentifiedType, NamedTypeKind,
  OperationDefinition, OperationId, OperationKind, OperationOwner, OperationSignature,
  OperationSourceKey, Ownership, ScalarType, TypeDefinition, TypeId, TypeSourceKey, ValueType,
};
use uniffi_js_engine_schema::{
  BridgePlan, BridgePlanInput, CallbackContract, CallbackReentrancy, CallbackRetention,
  CallbackThreading, CallbackUseSite, PlannedOperation, StreamContract, StreamUseSite, ValuePath,
};

fn ident(name: &str) -> Ident {
  Ident::new(name, Span::call_site())
}

fn argument(name: &str, binding: ArgumentBinding) -> RustArgumentPlan {
  RustArgumentPlan {
    name: ident(name),
    binding,
  }
}

fn direct(ty: syn::Type) -> ArgumentBinding {
  ArgumentBinding::Direct { carrier_type: ty }
}

fn lower(ty: syn::Type, path: syn::Path) -> ArgumentBinding {
  ArgumentBinding::LowerWith {
    carrier_type: ty,
    lower: path,
  }
}

fn rust_operation(
  id: u32,
  call: syn::Path,
  arguments: Vec<RustArgumentPlan>,
  return_binding: ReturnBinding,
  error_binding: ErrorBinding,
) -> RustOperationPlan {
  RustOperationPlan {
    operation_id: OperationId::new(id),
    target: RustOperationTarget::Native { call },
    receiver: None,
    arguments,
    return_binding,
    error_binding,
  }
}

fn host_operation(id: u32, target: RustOperationTarget) -> RustOperationPlan {
  RustOperationPlan {
    operation_id: OperationId::new(id),
    target,
    receiver: None,
    arguments: Vec::new(),
    return_binding: ReturnBinding::Unit,
    error_binding: ErrorBinding::Infallible,
  }
}

fn with_receiver(
  mut operation: RustOperationPlan,
  name: &str,
  binding: ArgumentBinding,
) -> RustOperationPlan {
  operation.receiver = Some(RustReceiverPlan {
    name: ident(name),
    binding,
  });
  operation
}

fn type_key(component: &ComponentKey, name: &str) -> TypeSourceKey {
  TypeSourceKey::new(component.clone(), name).unwrap()
}

#[expect(clippy::too_many_arguments)]
fn operation(
  component: &ComponentKey,
  id: u32,
  owner: OperationOwner,
  kind: OperationKind,
  name: &str,
  arguments: Vec<ArgumentDefinition>,
  return_type: Option<ValueType>,
  async_kind: AsyncKind,
  throws: Option<TypeSourceKey>,
) -> PlannedOperation {
  PlannedOperation::new(IdentifiedOperation {
    id: OperationId::new(id),
    definition: OperationDefinition::new(
      OperationSourceKey::new(component.clone(), owner, kind, name).unwrap(),
      name,
      format!("fixture::{name}"),
      format!("fixture_private_{id}"),
      OperationSignature {
        arguments,
        return_type,
        async_kind,
        throws,
      },
    )
    .unwrap(),
  })
}

fn arg(name: &str, ty: ValueType) -> ArgumentDefinition {
  ArgumentDefinition::new(name, ty, Ownership::Owned).unwrap()
}

fn bridge(flavor: HostFlavor) -> BridgePlan {
  let component = ComponentKey::new("fixture").unwrap();
  let payload = type_key(&component, "Payload");
  let failure = type_key(&component, "Failure");
  let object = type_key(&component, "Thing");
  let callback = type_key(&component, "Observer");
  let output_stream = type_key(&component, "ByteStream");
  let types = vec![
    IdentifiedType {
      id: TypeId::new(0),
      definition: TypeDefinition::new(
        payload.clone(),
        "Payload",
        NamedTypeKind::Record {
          fields: vec![FieldDefinition::new("value", ValueType::Scalar(ScalarType::I64)).unwrap()],
        },
      )
      .unwrap(),
    },
    IdentifiedType {
      id: TypeId::new(1),
      definition: TypeDefinition::new(
        failure.clone(),
        "Failure",
        NamedTypeKind::Error {
          variants: vec![EnumVariant::new(
            "Bad",
            vec![FieldDefinition::new("message", ValueType::Scalar(ScalarType::String)).unwrap()],
          )
          .unwrap()],
        },
      )
      .unwrap(),
    },
    IdentifiedType {
      id: TypeId::new(2),
      definition: TypeDefinition::new(object.clone(), "Thing", NamedTypeKind::Object).unwrap(),
    },
    IdentifiedType {
      id: TypeId::new(3),
      definition: TypeDefinition::new(callback.clone(), "Observer", NamedTypeKind::Callback)
        .unwrap(),
    },
    IdentifiedType {
      id: TypeId::new(4),
      definition: TypeDefinition::new(output_stream.clone(), "ByteStream", NamedTypeKind::Object)
        .unwrap(),
    },
  ];
  let operations = vec![
    operation(
      &component,
      0,
      OperationOwner::Namespace,
      OperationKind::Function,
      "numbers",
      vec![
        arg("signed", ValueType::Scalar(ScalarType::I64)),
        arg("unsigned", ValueType::Scalar(ScalarType::U64)),
        arg("payload", ValueType::Named(payload.clone())),
        arg("bytes", ValueType::Scalar(ScalarType::Bytes)),
      ],
      Some(ValueType::Scalar(ScalarType::I64)),
      AsyncKind::Sync,
      Some(failure.clone()),
    ),
    operation(
      &component,
      1,
      OperationOwner::Namespace,
      OperationKind::Function,
      "makeThing",
      Vec::new(),
      Some(ValueType::Named(object.clone())),
      AsyncKind::Async,
      None,
    ),
    operation(
      &component,
      2,
      OperationOwner::Object(object),
      OperationKind::Method,
      "add",
      vec![arg("delta", ValueType::Scalar(ScalarType::I64))],
      Some(ValueType::Scalar(ScalarType::I64)),
      AsyncKind::Sync,
      None,
    ),
    operation(
      &component,
      3,
      OperationOwner::Callback(callback.clone()),
      OperationKind::CallbackMethod,
      "onValue",
      vec![arg("payload", ValueType::Named(payload))],
      Some(ValueType::Scalar(ScalarType::I64)),
      AsyncKind::Async,
      Some(failure),
    ),
    operation(
      &component,
      4,
      OperationOwner::Namespace,
      OperationKind::Function,
      "observe",
      vec![arg("observer", ValueType::Named(callback))],
      None,
      AsyncKind::Sync,
      None,
    ),
    operation(
      &component,
      5,
      OperationOwner::Namespace,
      OperationKind::OutputStreamStart,
      "read",
      Vec::new(),
      Some(ValueType::output_stream(ValueType::Scalar(
        ScalarType::Bytes,
      ))),
      AsyncKind::Sync,
      None,
    ),
    operation(
      &component,
      6,
      OperationOwner::Object(output_stream.clone()),
      OperationKind::OutputStreamNext,
      "next",
      Vec::new(),
      Some(ValueType::optional(ValueType::Scalar(ScalarType::Bytes))),
      AsyncKind::Async,
      None,
    ),
    operation(
      &component,
      7,
      OperationOwner::Object(output_stream),
      OperationKind::OutputStreamCancel,
      "cancel",
      Vec::new(),
      None,
      AsyncKind::Async,
      None,
    ),
    operation(
      &component,
      8,
      OperationOwner::Namespace,
      OperationKind::Function,
      "write",
      vec![arg(
        "source",
        ValueType::input_stream(ValueType::Scalar(ScalarType::Bytes)),
      )],
      None,
      AsyncKind::Async,
      None,
    ),
    operation(
      &component,
      9,
      OperationOwner::Namespace,
      OperationKind::InputStreamPull,
      "pullInput",
      vec![arg("streamId", ValueType::Scalar(ScalarType::U32))],
      Some(ValueType::optional(ValueType::Scalar(ScalarType::Bytes))),
      AsyncKind::Async,
      None,
    ),
    operation(
      &component,
      10,
      OperationOwner::Namespace,
      OperationKind::InputStreamCancel,
      "cancelInput",
      vec![arg("streamId", ValueType::Scalar(ScalarType::U32))],
      None,
      AsyncKind::Async,
      None,
    ),
  ];
  BridgePlan::build(BridgePlanInput {
    components: vec![IdentifiedComponent {
      id: ComponentId::new(0),
      definition: ComponentDefinition::new(component, "fixture").unwrap(),
    }],
    types,
    operations,
    callbacks: vec![CallbackUseSite {
      operation_id: OperationId::new(4),
      callback_type: TypeId::new(3),
      path: ValuePath::argument(0),
      contract: CallbackContract {
        retention: CallbackRetention::Retained,
        threading: CallbackThreading::MayCrossThread,
        reentrancy: CallbackReentrancy::Forbidden,
      },
    }],
    streams: vec![
      StreamUseSite {
        operation_id: OperationId::new(5),
        path: ValuePath::return_value(),
        contract: StreamContract::output(),
      },
      StreamUseSite {
        operation_id: OperationId::new(8),
        path: ValuePath::argument(0),
        contract: StreamContract::input(),
      },
    ],
    targets: vec![flavor.capabilities()],
  })
  .unwrap()
}

fn rust_plan(bridge: &BridgePlan) -> RustBridgePlan {
  RustBridgePlan::build_with_resource_hooks(
    bridge,
    vec![
      rust_operation(
        0,
        syn::parse_quote!(fixture::numbers),
        vec![
          argument("signed", ArgumentBinding::I64BigInt),
          argument("unsigned", ArgumentBinding::U64BigInt),
          argument(
            "payload",
            lower(
              syn::parse_quote!(fixture::PayloadCarrier),
              syn::parse_quote!(fixture::lower_payload),
            ),
          ),
          argument(
            "bytes",
            lower(
              syn::parse_quote!(napi::bindgen_prelude::Uint8Array),
              syn::parse_quote!(fixture::lower_bytes),
            ),
          ),
        ],
        ReturnBinding::I64BigInt,
        ErrorBinding::Descriptor {
          map: syn::parse_quote!(fixture::map_declared_error),
        },
      ),
      rust_operation(
        1,
        syn::parse_quote!(fixture::make_thing),
        Vec::new(),
        ReturnBinding::ObjectLease {
          carrier_type: syn::parse_quote!(fixture::ObjectHandle),
          lift: syn::parse_quote!(fixture::lift_object),
        },
        ErrorBinding::Infallible,
      ),
      with_receiver(
        rust_operation(
          2,
          syn::parse_quote!(fixture::thing_add),
          vec![argument("delta", ArgumentBinding::I64BigInt)],
          ReturnBinding::I64BigInt,
          ErrorBinding::Infallible,
        ),
        "thing",
        ArgumentBinding::ObjectLease {
          carrier_type: syn::parse_quote!(fixture::ObjectHandle),
          lower: syn::parse_quote!(fixture::lower_object),
          ownership: Ownership::Borrowed,
        },
      ),
      host_operation(3, RustOperationTarget::CallbackHost),
      rust_operation(
        4,
        syn::parse_quote!(fixture::observe),
        vec![argument(
          "observer",
          ArgumentBinding::CallbackProxy {
            rust_type: syn::parse_quote!(fixture::CallbackHandle),
            build: syn::parse_quote!(fixture::build_callback_proxy),
          },
        )],
        ReturnBinding::Unit,
        ErrorBinding::Infallible,
      ),
      rust_operation(
        5,
        syn::parse_quote!(fixture::read),
        Vec::new(),
        ReturnBinding::OutputStreamLease {
          carrier_type: syn::parse_quote!(fixture::OutputStreamHandle),
          lift: syn::parse_quote!(fixture::lift_output_stream),
        },
        ErrorBinding::Infallible,
      ),
      with_receiver(
        rust_operation(
          6,
          syn::parse_quote!(fixture::next_output_stream),
          Vec::new(),
          ReturnBinding::LiftWith {
            carrier_type: syn::parse_quote!(Option<napi::bindgen_prelude::Uint8Array>),
            lift: syn::parse_quote!(fixture::lift_optional_bytes),
          },
          ErrorBinding::Infallible,
        ),
        "stream",
        ArgumentBinding::OutputStreamLease {
          carrier_type: syn::parse_quote!(fixture::OutputStreamHandle),
          lower: syn::parse_quote!(fixture::lower_output_stream),
          ownership: Ownership::Borrowed,
        },
      ),
      with_receiver(
        rust_operation(
          7,
          syn::parse_quote!(fixture::cancel_output_stream),
          Vec::new(),
          ReturnBinding::Unit,
          ErrorBinding::Infallible,
        ),
        "stream",
        ArgumentBinding::OutputStreamLease {
          carrier_type: syn::parse_quote!(fixture::OutputStreamHandle),
          lower: syn::parse_quote!(fixture::lower_output_stream),
          ownership: Ownership::Borrowed,
        },
      ),
      rust_operation(
        8,
        syn::parse_quote!(fixture::write),
        vec![argument(
          "source",
          ArgumentBinding::InputStreamProxy {
            rust_type: syn::parse_quote!(fixture::InputStreamHandle),
            build: syn::parse_quote!(fixture::build_input_stream_proxy),
          },
        )],
        ReturnBinding::Unit,
        ErrorBinding::Infallible,
      ),
      host_operation(9, RustOperationTarget::InputStreamHostPull),
      host_operation(10, RustOperationTarget::InputStreamHostCancel),
    ],
    resource_hooks(),
  )
  .unwrap()
}

fn resource_hooks() -> RustResourceHooks {
  RustResourceHooks {
    release_object: Some(RustResourceHook {
      call: syn::parse_quote!(fixture::release_object),
      carrier_type: syn::parse_quote!(fixture::ObjectHandle),
    }),
    cancel_output_stream: Some(RustResourceHook {
      call: syn::parse_quote!(fixture::cancel_output_stream_resource),
      carrier_type: syn::parse_quote!(fixture::OutputStreamHandle),
    }),
    release_output_stream: Some(RustResourceHook {
      call: syn::parse_quote!(fixture::release_output_stream),
      carrier_type: syn::parse_quote!(fixture::OutputStreamHandle),
    }),
  }
}

fn rebuild(
  bridge: &BridgePlan,
  operations: Vec<RustOperationPlan>,
) -> Result<RustBridgePlan, napi_uniffi_engine::EngineError> {
  RustBridgePlan::build_with_resource_hooks(bridge, operations, resource_hooks())
}

#[test]
fn generates_dense_private_operations_and_one_factory() {
  let bridge = bridge(HostFlavor::Node);
  let generated = generate_napi_module(&bridge, &rust_plan(&bridge), HostFlavor::Node).unwrap();
  let source = generated.source().to_string();

  assert_eq!(generated.raw_operation_names().len(), 8);
  assert_eq!(
    generated.public_exports().collect::<Vec<_>>(),
    vec![BACKEND_FACTORY_EXPORT]
  );
  // napi-rs emits native and wasm-target registration branches for the same
  // logical export.  No raw operation is present in either branch.
  assert_eq!(source.matches("register_module_export (").count(), 2);
  assert!(!source.contains("register_module_export ( None , \"__uniffi_raw_operation_"));
  assert!(source.contains("__uniffi_backend_factory"));
  assert!(source.contains("create_backend_session"));
  assert!(source.contains("SessionOperationDispatch :: CallbackHostAsync"));
  assert!(source.contains("SessionOperationDispatch :: InputStreamHostPull"));
  assert!(source.contains("get_i64"));
  assert!(source.contains("get_u64"));
  assert!(source.contains("require_lossless_i64"));
  assert!(source.contains("require_lossless_u64"));
  assert!(source.contains("map_declared_error"));
  assert!(source.contains("build_callback_proxy"));
  assert!(source.contains("lift_output_stream"));
  assert!(source.contains("build_input_stream_proxy"));
  assert!(source.contains("lower_object"));
  assert!(source.contains(". await"));
  assert!(syn::parse2::<syn::File>(generated.source().clone()).is_ok());
}

#[test]
fn exposes_object_callback_and_stream_runtime_hooks() {
  let bridge = bridge(HostFlavor::Node);
  let generated = generate_napi_module(&bridge, &rust_plan(&bridge), HostFlavor::Node).unwrap();
  let entrypoints = generated.family().runtime_entrypoints().collect::<Vec<_>>();
  for expected in [
    RuntimeEntrypoint::ReleaseObject,
    RuntimeEntrypoint::RetainCallback,
    RuntimeEntrypoint::ReleaseCallback,
    RuntimeEntrypoint::InvokeCallbackAsync,
    RuntimeEntrypoint::PullInputStream,
    RuntimeEntrypoint::CancelInputStream,
    RuntimeEntrypoint::ReleaseInputStream,
    RuntimeEntrypoint::NextOutputStream,
    RuntimeEntrypoint::CancelOutputStream,
    RuntimeEntrypoint::ReleaseOutputStream,
  ] {
    assert!(entrypoints.contains(&expected), "missing {expected:?}");
  }

  let object_method = &generated.family().operations()[2];
  assert_eq!(object_method.kind, OperationKind::Method);
  assert_eq!(
    object_method.receiver.as_ref().unwrap().ownership,
    Ownership::Borrowed
  );

  let callback_method = &generated.family().operations()[3];
  assert!(matches!(
    callback_method.target,
    napi_family_core::FamilyOperationTarget::CallbackHost(method)
      if method.callback_type == TypeId::new(3) && method.method_id == 0
  ));
  let callback_use = &generated.family().operations()[4].callbacks[0];
  assert_eq!(callback_use.contract.retention, CallbackRetention::Retained);
  assert_eq!(
    callback_use.contract.threading,
    CallbackThreading::MayCrossThread
  );
  assert_eq!(
    callback_use.contract.reentrancy,
    CallbackReentrancy::Forbidden
  );
  assert_eq!(callback_method.async_kind, AsyncKind::Async);
  assert_eq!(callback_method.declared_error, Some(TypeId::new(1)));
  assert!(generated
    .source()
    .to_string()
    .contains("SessionCallbackReentrancy :: Forbidden"));
}

#[test]
fn callback_method_dispatch_uses_each_method_signature() {
  let component = ComponentKey::new("mixed_callback_fixture").unwrap();
  let callback_key = TypeSourceKey::new(component.clone(), "Observer").unwrap();
  let failure_key = TypeSourceKey::new(component.clone(), "Failure").unwrap();
  let methods = [
    ("syncInfallible", AsyncKind::Sync, None),
    ("syncFallible", AsyncKind::Sync, Some(failure_key.clone())),
    ("asyncInfallible", AsyncKind::Async, None),
    ("asyncFallible", AsyncKind::Async, Some(failure_key.clone())),
  ];
  let operations = methods
    .iter()
    .enumerate()
    .map(|(id, (name, async_kind, throws))| {
      operation(
        &component,
        id as u32,
        OperationOwner::Callback(callback_key.clone()),
        OperationKind::CallbackMethod,
        name,
        Vec::new(),
        None,
        *async_kind,
        throws.clone(),
      )
    })
    .collect();
  let bridge = BridgePlan::build(BridgePlanInput {
    components: vec![IdentifiedComponent {
      id: ComponentId::new(0),
      definition: ComponentDefinition::new(component, "mixedCallbackFixture").unwrap(),
    }],
    types: vec![
      IdentifiedType {
        id: TypeId::new(0),
        definition: TypeDefinition::new(callback_key, "Observer", NamedTypeKind::Callback).unwrap(),
      },
      IdentifiedType {
        id: TypeId::new(1),
        definition: TypeDefinition::new(
          failure_key,
          "Failure",
          NamedTypeKind::Error {
            variants: Vec::new(),
          },
        )
        .unwrap(),
      },
    ],
    operations,
    callbacks: Vec::new(),
    streams: Vec::new(),
    targets: vec![HostFlavor::Node.capabilities()],
  })
  .unwrap();

  let family = napi_family_core::FamilyPlan::build(&bridge, HostFlavor::Node).unwrap();
  let methods = family
    .operations()
    .iter()
    .map(|operation| {
      let napi_family_core::FamilyOperationTarget::CallbackHost(method) = operation.target else {
        panic!("expected callback host operation")
      };
      (
        method.method_id,
        operation.async_kind,
        operation.declared_error,
      )
    })
    .collect::<Vec<_>>();
  assert_eq!(
    methods,
    vec![
      (0, AsyncKind::Sync, None),
      (1, AsyncKind::Sync, Some(TypeId::new(1))),
      (2, AsyncKind::Async, None),
      (3, AsyncKind::Async, Some(TypeId::new(1))),
    ]
  );
  let entrypoints = family.runtime_entrypoints().collect::<Vec<_>>();
  assert!(entrypoints.contains(&RuntimeEntrypoint::InvokeCallbackSync));
  assert!(entrypoints.contains(&RuntimeEntrypoint::InvokeCallbackAsync));
}

#[test]
fn node_and_ohos_share_the_plan_but_select_different_hooks() {
  let node_bridge = bridge(HostFlavor::Node);
  let ohos_bridge = bridge(HostFlavor::Ohos);
  let node =
    generate_napi_module(&node_bridge, &rust_plan(&node_bridge), HostFlavor::Node).unwrap();
  let ohos =
    generate_napi_module(&ohos_bridge, &rust_plan(&ohos_bridge), HostFlavor::Ohos).unwrap();

  assert_eq!(node.family().operations(), ohos.family().operations());
  assert_ne!(node.family().hooks(), ohos.family().hooks());
  assert!(node.source().to_string().contains("\"node\""));
  assert!(ohos.source().to_string().contains("\"ohos\""));
}

#[test]
fn rejects_lossy_integer_and_error_shortcuts() {
  let bridge = bridge(HostFlavor::Node);
  let mut operations = rust_plan(&bridge).operations().to_vec();
  operations[0].arguments[0].binding = direct(syn::parse_quote!(i64));
  let error = rebuild(&bridge, operations).unwrap_err();
  assert!(error.to_string().contains("lossless signed BigInt"));

  let mut operations = rust_plan(&bridge).operations().to_vec();
  operations[0].error_binding = ErrorBinding::Infallible;
  let error = rebuild(&bridge, operations).unwrap_err();
  assert!(error.to_string().contains("no error descriptor mapper"));
}

#[test]
fn rejects_duplicate_or_incomplete_operation_tables() {
  let bridge = bridge(HostFlavor::Node);
  let mut operations = rust_plan(&bridge).operations().to_vec();
  operations[1].operation_id = OperationId::new(0);
  let error = rebuild(&bridge, operations).unwrap_err();
  assert!(error.to_string().contains("duplicate Rust operation ID 0"));

  let mut operations = rust_plan(&bridge).operations().to_vec();
  operations.pop();
  let error = rebuild(&bridge, operations).unwrap_err();
  assert!(error.to_string().contains("bridge requires 11"));

  let mut operations = rust_plan(&bridge).operations().to_vec();
  operations[0].arguments[1].name = ident("signed");
  let error = rebuild(&bridge, operations).unwrap_err();
  assert!(error.to_string().contains("repeats Rust argument name"));
}

#[test]
fn rejects_unstructured_callback_stream_and_missing_receiver_shortcuts() {
  let bridge = bridge(HostFlavor::Node);

  let mut operations = rust_plan(&bridge).operations().to_vec();
  operations[4].arguments[0].binding = lower(
    syn::parse_quote!(fixture::CallbackHandle),
    syn::parse_quote!(fixture::lower_callback),
  );
  let error = rebuild(&bridge, operations).unwrap_err();
  assert!(error.to_string().contains("explicit carrier adapter"));

  let mut operations = rust_plan(&bridge).operations().to_vec();
  operations[8].arguments[0].binding = lower(
    syn::parse_quote!(fixture::InputStreamHandle),
    syn::parse_quote!(fixture::lower_input_stream),
  );
  let error = rebuild(&bridge, operations).unwrap_err();
  assert!(error.to_string().contains("explicit carrier adapter"));

  let mut operations = rust_plan(&bridge).operations().to_vec();
  operations[2].receiver = None;
  let error = rebuild(&bridge, operations).unwrap_err();
  assert!(error.to_string().contains("no resource receiver"));

  let mut operations = rust_plan(&bridge).operations().to_vec();
  operations[3].target = RustOperationTarget::Native {
    call: syn::parse_quote!(fixture::fake_callback_call),
  };
  let error = rebuild(&bridge, operations).unwrap_err();
  assert!(error
    .to_string()
    .contains("incompatible with CallbackMethod"));
}
