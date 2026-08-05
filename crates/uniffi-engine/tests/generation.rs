use napi_family_core::{
  AsyncKind, CallbackContract, CallbackReentrancy, CallbackRetention, CallbackThreading,
  CallbackUseSite, CarrierKind, ClosePolicy, ConversionRecipe, DeadlineAction,
  FamilyOperationInput, FamilyOperationTarget, FamilyPlan, FamilyPlanInput, HostFlavor,
  OperationDispatch, OperationKind, ReceiverBinding, ResourceBinding, ResourceKind,
  ResourceOwnership, StreamDirection, StreamSlotIdentity, StreamUseSite, StreamValueBinding,
  ValuePath,
};
use napi_uniffi_engine::{
  generate_napi_module, ArgumentBinding, ErrorBinding, ReturnBinding, RustArgumentPlan,
  RustBridgePlan, RustOperationPlan, RustOperationTarget, RustReceiverPlan, RustResourceHook,
  RustResourceHooks, BACKEND_FACTORY_EXPORT,
};
use proc_macro2::{Ident, Span};

fn ident(name: &str) -> Ident {
  Ident::new(name, Span::call_site())
}

const TEST_CLOSE_POLICY: ClosePolicy = ClosePolicy {
  grace_ms: 5_000,
  on_deadline: DeadlineAction::Detach,
};

fn argument(name: &str, binding: ArgumentBinding) -> RustArgumentPlan {
  RustArgumentPlan {
    name: ident(name),
    binding,
  }
}

fn family_operation(
  id: u32,
  kind: OperationKind,
  async_kind: AsyncKind,
  argument_count: usize,
  fallible: bool,
  dispatch: OperationDispatch,
) -> FamilyOperationInput {
  FamilyOperationInput {
    id,
    kind,
    async_kind,
    fallible,
    argument_count,
    dispatch,
    receiver: None,
    result: None,
    callbacks: Vec::new(),
    streams: Vec::new(),
    stream_slot: None,
  }
}

fn family(flavor: HostFlavor) -> FamilyPlan {
  let mut operations = vec![
    family_operation(
      0,
      OperationKind::Function,
      AsyncKind::Sync,
      4,
      true,
      OperationDispatch::Native,
    ),
    family_operation(
      1,
      OperationKind::Function,
      AsyncKind::Async,
      0,
      false,
      OperationDispatch::Native,
    ),
    family_operation(
      2,
      OperationKind::Method,
      AsyncKind::Sync,
      1,
      false,
      OperationDispatch::Native,
    ),
    family_operation(
      3,
      OperationKind::CallbackMethod,
      AsyncKind::Async,
      0,
      true,
      OperationDispatch::CallbackHost {
        callback_type_id: 3,
        method_id: 0,
      },
    ),
    family_operation(
      4,
      OperationKind::Function,
      AsyncKind::Sync,
      1,
      false,
      OperationDispatch::Native,
    ),
    family_operation(
      5,
      OperationKind::OutputStreamStart,
      AsyncKind::Sync,
      0,
      false,
      OperationDispatch::Native,
    ),
    family_operation(
      6,
      OperationKind::OutputStreamNext,
      AsyncKind::Async,
      0,
      false,
      OperationDispatch::Native,
    ),
    family_operation(
      7,
      OperationKind::OutputStreamCancel,
      AsyncKind::Async,
      0,
      false,
      OperationDispatch::Native,
    ),
    family_operation(
      8,
      OperationKind::Function,
      AsyncKind::Async,
      1,
      false,
      OperationDispatch::Native,
    ),
    family_operation(
      9,
      OperationKind::InputStreamPull,
      AsyncKind::Async,
      0,
      false,
      OperationDispatch::InputStreamHostPull,
    ),
    family_operation(
      10,
      OperationKind::InputStreamCancel,
      AsyncKind::Async,
      0,
      false,
      OperationDispatch::InputStreamHostCancel,
    ),
  ];
  operations[1].result = Some(ResourceBinding {
    kind: ResourceKind::Object,
    ownership: ResourceOwnership::Owned,
  });
  operations[2].receiver = Some(ReceiverBinding::Resource(ResourceBinding {
    kind: ResourceKind::Object,
    ownership: ResourceOwnership::Borrowed,
  }));
  operations[4].callbacks.push(CallbackUseSite {
    operation_id: 4,
    callback_type_id: 3,
    path: ValuePath::argument(0),
    contract: CallbackContract {
      retention: CallbackRetention::Retained,
      threading: CallbackThreading::MayCrossThread,
      reentrancy: CallbackReentrancy::Forbidden,
    },
  });
  operations[5].result = Some(ResourceBinding {
    kind: ResourceKind::OutputStream,
    ownership: ResourceOwnership::Owned,
  });
  operations[5].streams.push(StreamUseSite {
    operation_id: 5,
    use_site_id: 0,
    path: ValuePath::return_value(),
    direction: StreamDirection::Output,
    item: StreamValueBinding {
      carrier: CarrierKind::Primitive,
      conversion: ConversionRecipe::Identity,
    },
    error: StreamValueBinding {
      carrier: CarrierKind::Primitive,
      conversion: ConversionRecipe::Identity,
    },
    is_send: false,
    slots: vec![
      StreamSlotIdentity {
        use_site_id: 0,
        operation_id: 5,
        kind: OperationKind::OutputStreamStart,
      },
      StreamSlotIdentity {
        use_site_id: 0,
        operation_id: 6,
        kind: OperationKind::OutputStreamNext,
      },
      StreamSlotIdentity {
        use_site_id: 0,
        operation_id: 7,
        kind: OperationKind::OutputStreamCancel,
      },
    ],
  });
  operations[5].stream_slot = Some(StreamSlotIdentity {
    use_site_id: 0,
    operation_id: 5,
    kind: OperationKind::OutputStreamStart,
  });
  for (id, kind) in [
    (6, OperationKind::OutputStreamNext),
    (7, OperationKind::OutputStreamCancel),
  ] {
    operations[id as usize].receiver = Some(ReceiverBinding::Resource(ResourceBinding {
      kind: ResourceKind::OutputStream,
      ownership: ResourceOwnership::Borrowed,
    }));
    operations[id as usize].kind = kind;
    operations[id as usize].stream_slot = Some(StreamSlotIdentity {
      use_site_id: 0,
      operation_id: id,
      kind,
    });
  }
  operations[8].streams.push(StreamUseSite {
    operation_id: 8,
    use_site_id: 1,
    path: ValuePath::argument(0),
    direction: StreamDirection::Input,
    item: StreamValueBinding {
      carrier: CarrierKind::Primitive,
      conversion: ConversionRecipe::Identity,
    },
    error: StreamValueBinding {
      carrier: CarrierKind::Primitive,
      conversion: ConversionRecipe::Identity,
    },
    is_send: false,
    slots: vec![
      StreamSlotIdentity {
        use_site_id: 1,
        operation_id: 9,
        kind: OperationKind::InputStreamPull,
      },
      StreamSlotIdentity {
        use_site_id: 1,
        operation_id: 10,
        kind: OperationKind::InputStreamCancel,
      },
    ],
  });
  operations[9].receiver = Some(ReceiverBinding::Resource(ResourceBinding {
    kind: ResourceKind::InputStream,
    ownership: ResourceOwnership::Borrowed,
  }));
  operations[10].receiver = Some(ReceiverBinding::Resource(ResourceBinding {
    kind: ResourceKind::InputStream,
    ownership: ResourceOwnership::Borrowed,
  }));
  operations[9].stream_slot = Some(StreamSlotIdentity {
    use_site_id: 1,
    operation_id: 9,
    kind: OperationKind::InputStreamPull,
  });
  operations[10].stream_slot = Some(StreamSlotIdentity {
    use_site_id: 1,
    operation_id: 10,
    kind: OperationKind::InputStreamCancel,
  });
  FamilyPlan::build(FamilyPlanInput {
    flavor,
    close_policy: TEST_CLOSE_POLICY,
    operations,
  })
  .unwrap()
}

fn native(
  id: u32,
  call: syn::Path,
  arguments: Vec<RustArgumentPlan>,
  return_binding: ReturnBinding,
  error_binding: ErrorBinding,
) -> RustOperationPlan {
  RustOperationPlan {
    operation_id: id,
    target: RustOperationTarget::Native { call },
    receiver: None,
    arguments,
    return_binding,
    error_binding,
  }
}

fn host(id: u32, target: RustOperationTarget) -> RustOperationPlan {
  RustOperationPlan {
    operation_id: id,
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

fn rust_plan(family: &FamilyPlan) -> RustBridgePlan {
  let operations = vec![
    native(
      0,
      syn::parse_quote!(fixture::numbers),
      vec![
        argument("signed", ArgumentBinding::I64BigInt),
        argument("unsigned", ArgumentBinding::U64BigInt),
        argument(
          "payload",
          ArgumentBinding::LowerWith {
            carrier_type: syn::parse_quote!(fixture::PayloadCarrier),
            lower: syn::parse_quote!(fixture::lower_payload),
          },
        ),
        argument(
          "bytes",
          ArgumentBinding::LowerWith {
            carrier_type: syn::parse_quote!(napi::bindgen_prelude::Uint8Array),
            lower: syn::parse_quote!(fixture::lower_bytes),
          },
        ),
      ],
      ReturnBinding::I64BigInt,
      ErrorBinding::Descriptor {
        map: syn::parse_quote!(fixture::map_declared_error),
      },
    ),
    native(
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
      native(
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
        ownership: ResourceOwnership::Borrowed,
      },
    ),
    host(3, RustOperationTarget::CallbackHost),
    native(
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
    native(
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
      native(
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
        ownership: ResourceOwnership::Borrowed,
      },
    ),
    with_receiver(
      native(
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
        ownership: ResourceOwnership::Borrowed,
      },
    ),
    native(
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
    host(9, RustOperationTarget::InputStreamHostPull),
    host(10, RustOperationTarget::InputStreamHostCancel),
  ];
  RustBridgePlan::build_with_resource_hooks(
    family,
    operations,
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
    },
  )
  .unwrap()
}

#[test]
fn generates_dense_private_operations_and_one_factory() {
  let family = family(HostFlavor::Node);
  let generated = generate_napi_module(&family, &rust_plan(&family)).unwrap();
  let source = generated.source().to_string();
  assert_eq!(generated.raw_operation_names().len(), 8);
  assert_eq!(
    generated.public_exports().collect::<Vec<_>>(),
    vec![BACKEND_FACTORY_EXPORT]
  );
  assert_eq!(source.matches("register_module_export (").count(), 2);
  assert!(!source.contains("register_module_export ( None , \"__uniffi_raw_operation_"));
  for expected in [
    "__uniffi_backend_factory",
    "create_backend_session",
    "SessionOperationDispatch :: CallbackHostAsync",
    "SessionOperationDispatch :: InputStreamHostPull",
    "get_i64",
    "get_u64",
    "require_lossless_i64",
    "require_lossless_u64",
    "build_callback_proxy",
    "lift_output_stream",
    "build_input_stream_proxy",
    "lower_object",
  ] {
    assert!(
      source.contains(expected),
      "generated source misses {expected}"
    );
  }
  assert!(syn::parse2::<syn::File>(generated.source().clone()).is_ok());
}

#[test]
fn exposes_callback_stream_and_resource_runtime_hooks() {
  let family = family(HostFlavor::Node);
  let generated = generate_napi_module(&family, &rust_plan(&family)).unwrap();
  let entrypoints = generated.family().runtime_entrypoints().collect::<Vec<_>>();
  for expected in [
    napi_family_core::RuntimeEntrypoint::ReleaseObject,
    napi_family_core::RuntimeEntrypoint::RetainCallback,
    napi_family_core::RuntimeEntrypoint::ReleaseCallback,
    napi_family_core::RuntimeEntrypoint::InvokeCallbackAsync,
    napi_family_core::RuntimeEntrypoint::PullInputStream,
    napi_family_core::RuntimeEntrypoint::CancelInputStream,
    napi_family_core::RuntimeEntrypoint::ReleaseInputStream,
    napi_family_core::RuntimeEntrypoint::NextOutputStream,
    napi_family_core::RuntimeEntrypoint::CancelOutputStream,
    napi_family_core::RuntimeEntrypoint::ReleaseOutputStream,
  ] {
    assert!(entrypoints.contains(&expected), "missing {expected:?}");
  }
  assert!(matches!(
    generated.family().operations()[2].receiver,
    Some(ReceiverBinding::Resource(ResourceBinding {
      ownership: ResourceOwnership::Borrowed,
      ..
    }))
  ));
  assert!(matches!(
    generated.family().operations()[3].target,
    FamilyOperationTarget::CallbackHost {
      callback_type_id: 3,
      method_id: 0
    }
  ));
  assert!(generated
    .source()
    .to_string()
    .contains("SessionCallbackReentrancy :: Forbidden"));
}

#[test]
fn value_receivers_use_regular_lowering_and_never_resource_binding() {
  let family = FamilyPlan::build(FamilyPlanInput {
    flavor: HostFlavor::Node,
    close_policy: TEST_CLOSE_POLICY,
    operations: vec![FamilyOperationInput {
      id: 0,
      kind: OperationKind::Method,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ReceiverBinding::Value),
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    }],
  })
  .unwrap();
  let valid = RustBridgePlan::build(
    &family,
    vec![RustOperationPlan {
      operation_id: 0,
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::record_method),
      },
      receiver: Some(RustReceiverPlan {
        name: ident("record"),
        binding: ArgumentBinding::LowerWith {
          carrier_type: syn::parse_quote!(napi::bindgen_prelude::Object<'static>),
          lower: syn::parse_quote!(fixture::lower_record),
        },
      }),
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    }],
  )
  .unwrap();
  let generated = generate_napi_module(&family, &valid).unwrap();
  let source = generated.source().to_string();
  assert!(source.contains("fixture :: lower_record"));
  assert!(source.contains("SessionReceiver :: Value"));
  assert!(!source.contains("missing resource receiver"));

  let invalid = RustBridgePlan::build(
    &family,
    vec![RustOperationPlan {
      operation_id: 0,
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::record_method),
      },
      receiver: Some(RustReceiverPlan {
        name: ident("record"),
        binding: ArgumentBinding::ObjectLease {
          carrier_type: syn::parse_quote!(u32),
          lower: syn::parse_quote!(fixture::lower_object),
          ownership: ResourceOwnership::Borrowed,
        },
      }),
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    }],
  )
  .unwrap_err();
  assert!(invalid.to_string().contains("regular value lowering"));
}

#[test]
fn async_value_receiver_lowering_stays_outside_worker_future() {
  let family = FamilyPlan::build(FamilyPlanInput {
    flavor: HostFlavor::Node,
    close_policy: TEST_CLOSE_POLICY,
    operations: vec![FamilyOperationInput {
      id: 0,
      kind: OperationKind::Method,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ReceiverBinding::Value),
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    }],
  })
  .unwrap();
  let bridge = RustBridgePlan::build(
    &family,
    vec![RustOperationPlan {
      operation_id: 0,
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::async_record_method),
      },
      receiver: Some(RustReceiverPlan {
        name: ident("record"),
        binding: ArgumentBinding::LowerWith {
          carrier_type: syn::parse_quote!(napi::bindgen_prelude::Object<'static>),
          lower: syn::parse_quote!(fixture::lower_record),
        },
      }),
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    }],
  )
  .unwrap();
  let source = generate_napi_module(&family, &bridge)
    .unwrap()
    .source()
    .to_string();
  assert!(source.contains("__uniffi_env"));
  let lower_offset = source.find("fixture :: lower_record").unwrap();
  let future_offset = source.find("let __uniffi_future = async move").unwrap();
  assert!(lower_offset < future_offset);
  let future_end = source[future_offset..]
    .find("let __uniffi_promise")
    .map(|offset| future_offset + offset)
    .unwrap();
  let future = &source[future_offset..future_end];
  assert!(!future.contains("fixture :: lower_record"));
  assert!(!future.contains("napi :: bindgen_prelude :: Object"));
  assert!(!source.contains("async fn __uniffi_raw_operation_0"));
}

#[test]
fn node_and_ohos_share_operations_but_select_hooks() {
  let node = family(HostFlavor::Node);
  let ohos = family(HostFlavor::Ohos);
  assert_eq!(node.operations(), ohos.operations());
  assert_ne!(node.hooks(), ohos.hooks());
  let node_source = generate_napi_module(&node, &rust_plan(&node))
    .unwrap()
    .source()
    .to_string();
  let ohos_source = generate_napi_module(&ohos, &rust_plan(&ohos))
    .unwrap()
    .source()
    .to_string();
  assert!(node_source.contains("\"node\""));
  assert!(ohos_source.contains("\"ohos\""));
}

#[test]
fn rejects_duplicate_or_incomplete_rust_operation_tables() {
  let family = family(HostFlavor::Node);
  let mut operations = rust_plan(&family).operations().to_vec();
  operations[1].operation_id = 0;
  let error = RustBridgePlan::build(&family, operations).unwrap_err();
  assert!(error.to_string().contains("duplicate Rust operation ID 0"));
  let mut operations = rust_plan(&family).operations().to_vec();
  operations.pop();
  let error = RustBridgePlan::build(&family, operations).unwrap_err();
  assert!(error.to_string().contains("bridge requires 11"));
}

#[test]
fn callback_and_stream_bindings_are_structured() {
  let family = family(HostFlavor::Node);
  let mut operations = rust_plan(&family).operations().to_vec();
  operations[4].arguments[0].binding = ArgumentBinding::Direct {
    carrier_type: syn::parse_quote!(u32),
  };
  let error = RustBridgePlan::build(&family, operations).unwrap_err();
  assert!(error.to_string().contains("structured callback"));
}

#[test]
fn structured_bindings_match_every_canonical_use_site() {
  let nested_operation = FamilyOperationInput {
    id: 0,
    kind: OperationKind::Function,
    async_kind: AsyncKind::Sync,
    fallible: false,
    argument_count: 1,
    dispatch: OperationDispatch::Native,
    receiver: None,
    result: None,
    callbacks: vec![CallbackUseSite {
      operation_id: 0,
      callback_type_id: 3,
      path: ValuePath::new(vec![
        napi_family_core::ValuePathSegment::Argument(0),
        napi_family_core::ValuePathSegment::Field("callback".to_owned()),
      ]),
      contract: CallbackContract {
        retention: CallbackRetention::Retained,
        threading: CallbackThreading::CallingThread,
        reentrancy: CallbackReentrancy::Allowed,
      },
    }],
    streams: Vec::new(),
    stream_slot: None,
  };
  let nested_family = FamilyPlan::build(FamilyPlanInput {
    flavor: HostFlavor::Node,
    close_policy: TEST_CLOSE_POLICY,
    operations: vec![nested_operation],
  })
  .unwrap();
  let direct_binding = native(
    0,
    syn::parse_quote!(fixture::lower),
    vec![argument(
      "value",
      ArgumentBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
    )],
    ReturnBinding::Unit,
    ErrorBinding::Infallible,
  );
  let error = RustBridgePlan::build(&nested_family, vec![direct_binding]).unwrap_err();
  assert!(error.to_string().contains("structured callback"));
  let nested_binding = native(
    0,
    syn::parse_quote!(fixture::lower),
    vec![argument(
      "value",
      ArgumentBinding::LowerWithHost {
        carrier_type: syn::parse_quote!(u32),
        lower: syn::parse_quote!(fixture::lower),
      },
    )],
    ReturnBinding::Unit,
    ErrorBinding::Infallible,
  );
  assert!(RustBridgePlan::build(&nested_family, vec![nested_binding]).is_ok());

  let no_use_family = FamilyPlan::build(FamilyPlanInput {
    flavor: HostFlavor::Node,
    close_policy: TEST_CLOSE_POLICY,
    operations: vec![family_operation(
      0,
      OperationKind::Function,
      AsyncKind::Sync,
      1,
      false,
      OperationDispatch::Native,
    )],
  })
  .unwrap();
  let missing_use_site = native(
    0,
    syn::parse_quote!(fixture::lower),
    vec![argument(
      "value",
      ArgumentBinding::LowerWithHost {
        carrier_type: syn::parse_quote!(u32),
        lower: syn::parse_quote!(fixture::lower),
      },
    )],
    ReturnBinding::Unit,
    ErrorBinding::Infallible,
  );
  let error = RustBridgePlan::build(&no_use_family, vec![missing_use_site]).unwrap_err();
  assert!(error
    .to_string()
    .contains("structured callback or input stream"));

  let mut return_callback_operation = family_operation(
    0,
    OperationKind::Function,
    AsyncKind::Sync,
    0,
    false,
    OperationDispatch::Native,
  );
  return_callback_operation.callbacks.push(CallbackUseSite {
    operation_id: 0,
    callback_type_id: 3,
    path: ValuePath::return_value(),
    contract: CallbackContract {
      retention: CallbackRetention::Retained,
      threading: CallbackThreading::CallingThread,
      reentrancy: CallbackReentrancy::Allowed,
    },
  });
  let return_callback_family = FamilyPlan::build(FamilyPlanInput {
    flavor: HostFlavor::Node,
    close_policy: TEST_CLOSE_POLICY,
    operations: vec![return_callback_operation],
  })
  .unwrap();
  let direct_return = native(
    0,
    syn::parse_quote!(fixture::return_callback),
    Vec::new(),
    ReturnBinding::Direct {
      carrier_type: syn::parse_quote!(u32),
    },
    ErrorBinding::Infallible,
  );
  let error = RustBridgePlan::build(&return_callback_family, vec![direct_return]).unwrap_err();
  assert!(error.to_string().contains("direct callback lease"));
  let callback_lease = native(
    0,
    syn::parse_quote!(fixture::return_callback),
    Vec::new(),
    ReturnBinding::CallbackLease {
      carrier_type: syn::parse_quote!(u32),
      lift: syn::parse_quote!(fixture::lift_callback),
    },
    ErrorBinding::Infallible,
  );
  assert!(RustBridgePlan::build(&return_callback_family, vec![callback_lease]).is_ok());

  let no_return_callback_family = FamilyPlan::build(FamilyPlanInput {
    flavor: HostFlavor::Node,
    close_policy: TEST_CLOSE_POLICY,
    operations: vec![family_operation(
      0,
      OperationKind::Function,
      AsyncKind::Sync,
      0,
      false,
      OperationDispatch::Native,
    )],
  })
  .unwrap();
  let stray_callback_lease = native(
    0,
    syn::parse_quote!(fixture::return_callback),
    Vec::new(),
    ReturnBinding::CallbackLease {
      carrier_type: syn::parse_quote!(u32),
      lift: syn::parse_quote!(fixture::lift_callback),
    },
    ErrorBinding::Infallible,
  );
  let error =
    RustBridgePlan::build(&no_return_callback_family, vec![stray_callback_lease]).unwrap_err();
  assert!(error.to_string().contains("direct callback lease"));

  let host_family = FamilyPlan::build(FamilyPlanInput {
    flavor: HostFlavor::Node,
    close_policy: TEST_CLOSE_POLICY,
    operations: vec![FamilyOperationInput {
      id: 0,
      kind: OperationKind::CallbackMethod,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::CallbackHost {
        callback_type_id: 3,
        method_id: 0,
      },
      receiver: None,
      result: None,
      callbacks: vec![CallbackUseSite {
        operation_id: 0,
        callback_type_id: 3,
        path: ValuePath::return_value(),
        contract: CallbackContract {
          retention: CallbackRetention::Retained,
          threading: CallbackThreading::CallingThread,
          reentrancy: CallbackReentrancy::Allowed,
        },
      }],
      streams: Vec::new(),
      stream_slot: None,
    }],
  })
  .unwrap();
  let error = RustBridgePlan::build(
    &host_family,
    vec![host(0, RustOperationTarget::CallbackHost)],
  )
  .unwrap_err();
  assert!(error
    .to_string()
    .contains("host operation 0 must not contain callback or stream use-sites"));
}

#[test]
fn resource_results_are_mechanically_bound_to_return_carriers() {
  let family = family(HostFlavor::Node);
  let mut operations = rust_plan(&family).operations().to_vec();
  operations[1].return_binding = ReturnBinding::Direct {
    carrier_type: syn::parse_quote!(u32),
  };
  let error = RustBridgePlan::build(&family, operations).unwrap_err();
  assert!(error.to_string().contains("return binding incompatible"));

  let no_resource_family = FamilyPlan::build(FamilyPlanInput {
    flavor: HostFlavor::Node,
    close_policy: TEST_CLOSE_POLICY,
    operations: vec![family_operation(
      0,
      OperationKind::Function,
      AsyncKind::Sync,
      0,
      false,
      OperationDispatch::Native,
    )],
  })
  .unwrap();
  let error = RustBridgePlan::build(
    &no_resource_family,
    vec![RustOperationPlan {
      operation_id: 0,
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::wrong_resource),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::ObjectLease {
        carrier_type: syn::parse_quote!(u32),
        lift: syn::parse_quote!(fixture::lift_object),
      },
      error_binding: ErrorBinding::Infallible,
    }],
  )
  .unwrap_err();
  assert!(error.to_string().contains("return binding incompatible"));
}
