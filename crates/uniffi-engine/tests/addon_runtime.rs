use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use napi_family_core::{
  AsyncKind, CallbackContract, CallbackReentrancy, CallbackRetention, CallbackThreading,
  CallbackUseSite, FamilyOperationInput, FamilyPlan, FamilyPlanInput, HostFlavor,
  OperationDispatch, OperationKind, ResourceBinding, ResourceKind, ResourceOwnership,
  StreamDirection, StreamSlotIdentity, StreamUseSite, StreamValueBinding, ValuePath,
};
use napi_uniffi_engine::{
  generate_napi_module, ArgumentBinding, ErrorBinding, ReturnBinding, RustArgumentPlan,
  RustBridgePlan, RustOperationPlan, RustOperationTarget, RustResourceHook, RustResourceHooks,
};
use proc_macro2::{Ident, Span};

struct TempFixture(PathBuf);

impl Drop for TempFixture {
  fn drop(&mut self) {
    let _ = fs::remove_dir_all(&self.0);
  }
}

fn family() -> FamilyPlan {
  let mut operations = vec![
    FamilyOperationInput {
      id: 0,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 1,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 2,
      kind: OperationKind::CallbackMethod,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::CallbackHost {
        callback_type_id: 0,
        method_id: 0,
      },
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 3,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: vec![
        CallbackUseSite {
          operation_id: 3,
          callback_type_id: 0,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Argument(0),
            napi_family_core::ValuePathSegment::Field("event".to_owned()),
            napi_family_core::ValuePathSegment::Variant("Ready".to_owned()),
            napi_family_core::ValuePathSegment::Field("callback".to_owned()),
          ]),
          contract: CallbackContract {
            retention: CallbackRetention::Retained,
            threading: CallbackThreading::CallingThread,
            reentrancy: CallbackReentrancy::Forbidden,
          },
        },
        CallbackUseSite {
          operation_id: 3,
          callback_type_id: 0,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Argument(0),
            napi_family_core::ValuePathSegment::Field("optionalNull".to_owned()),
            napi_family_core::ValuePathSegment::Optional,
            napi_family_core::ValuePathSegment::Field("callback".to_owned()),
          ]),
          contract: CallbackContract {
            retention: CallbackRetention::Retained,
            threading: CallbackThreading::CallingThread,
            reentrancy: CallbackReentrancy::Forbidden,
          },
        },
        CallbackUseSite {
          operation_id: 3,
          callback_type_id: 0,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Argument(0),
            napi_family_core::ValuePathSegment::Field("optionalUndefined".to_owned()),
            napi_family_core::ValuePathSegment::Optional,
            napi_family_core::ValuePathSegment::Field("callback".to_owned()),
          ]),
          contract: CallbackContract {
            retention: CallbackRetention::Retained,
            threading: CallbackThreading::CallingThread,
            reentrancy: CallbackReentrancy::Forbidden,
          },
        },
        CallbackUseSite {
          operation_id: 3,
          callback_type_id: 0,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Argument(0),
            napi_family_core::ValuePathSegment::Field("callbacksEmpty".to_owned()),
            napi_family_core::ValuePathSegment::SequenceElement,
          ]),
          contract: CallbackContract {
            retention: CallbackRetention::Retained,
            threading: CallbackThreading::CallingThread,
            reentrancy: CallbackReentrancy::Forbidden,
          },
        },
        CallbackUseSite {
          operation_id: 3,
          callback_type_id: 0,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Argument(0),
            napi_family_core::ValuePathSegment::Field("callbacks".to_owned()),
            napi_family_core::ValuePathSegment::SequenceElement,
          ]),
          contract: CallbackContract {
            retention: CallbackRetention::Retained,
            threading: CallbackThreading::CallingThread,
            reentrancy: CallbackReentrancy::Forbidden,
          },
        },
        CallbackUseSite {
          operation_id: 3,
          callback_type_id: 0,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Argument(0),
            napi_family_core::ValuePathSegment::Field("mapEmpty".to_owned()),
            napi_family_core::ValuePathSegment::MapKey,
          ]),
          contract: CallbackContract {
            retention: CallbackRetention::Retained,
            threading: CallbackThreading::CallingThread,
            reentrancy: CallbackReentrancy::Forbidden,
          },
        },
        CallbackUseSite {
          operation_id: 3,
          callback_type_id: 0,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Argument(0),
            napi_family_core::ValuePathSegment::Field("map".to_owned()),
            napi_family_core::ValuePathSegment::MapKey,
          ]),
          contract: CallbackContract {
            retention: CallbackRetention::Retained,
            threading: CallbackThreading::CallingThread,
            reentrancy: CallbackReentrancy::Forbidden,
          },
        },
        CallbackUseSite {
          operation_id: 3,
          callback_type_id: 0,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Argument(0),
            napi_family_core::ValuePathSegment::Field("map".to_owned()),
            napi_family_core::ValuePathSegment::MapValue,
          ]),
          contract: CallbackContract {
            retention: CallbackRetention::Retained,
            threading: CallbackThreading::CallingThread,
            reentrancy: CallbackReentrancy::Forbidden,
          },
        },
        CallbackUseSite {
          operation_id: 3,
          callback_type_id: 0,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Argument(0),
            napi_family_core::ValuePathSegment::Field("setEmpty".to_owned()),
            napi_family_core::ValuePathSegment::SetElement,
          ]),
          contract: CallbackContract {
            retention: CallbackRetention::Retained,
            threading: CallbackThreading::CallingThread,
            reentrancy: CallbackReentrancy::Forbidden,
          },
        },
        CallbackUseSite {
          operation_id: 3,
          callback_type_id: 0,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Argument(0),
            napi_family_core::ValuePathSegment::Field("set".to_owned()),
            napi_family_core::ValuePathSegment::SetElement,
          ]),
          contract: CallbackContract {
            retention: CallbackRetention::Retained,
            threading: CallbackThreading::CallingThread,
            reentrancy: CallbackReentrancy::Forbidden,
          },
        },
      ],
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 4,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: vec![StreamUseSite {
        operation_id: 4,
        use_site_id: 0,
        path: ValuePath::argument(0),
        direction: StreamDirection::Input,
        item: StreamValueBinding {
          carrier: napi_family_core::CarrierKind::Primitive,
          conversion: napi_family_core::ConversionRecipe::Identity,
        },
        error: StreamValueBinding {
          carrier: napi_family_core::CarrierKind::Primitive,
          conversion: napi_family_core::ConversionRecipe::Identity,
        },
        is_send: false,
        slots: vec![
          StreamSlotIdentity {
            use_site_id: 0,
            operation_id: 5,
            kind: OperationKind::InputStreamPull,
          },
          StreamSlotIdentity {
            use_site_id: 0,
            operation_id: 6,
            kind: OperationKind::InputStreamCancel,
          },
        ],
      }],
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 5,
      kind: OperationKind::InputStreamPull,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::InputStreamHostPull,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 6,
      kind: OperationKind::InputStreamCancel,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::InputStreamHostCancel,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
  ];
  operations.extend([
    FamilyOperationInput {
      id: 7,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: Some(ResourceBinding {
        kind: ResourceKind::Object,
        ownership: ResourceOwnership::Owned,
      }),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 8,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: true,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: Some(ResourceBinding {
        kind: ResourceKind::Object,
        ownership: ResourceOwnership::Owned,
      }),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 9,
      kind: OperationKind::OutputStreamStart,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: Some(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Owned,
      }),
      callbacks: Vec::new(),
      streams: vec![StreamUseSite {
        operation_id: 9,
        use_site_id: 1,
        path: ValuePath::return_value(),
        direction: StreamDirection::Output,
        item: StreamValueBinding {
          carrier: napi_family_core::CarrierKind::Primitive,
          conversion: napi_family_core::ConversionRecipe::Identity,
        },
        error: StreamValueBinding {
          carrier: napi_family_core::CarrierKind::Primitive,
          conversion: napi_family_core::ConversionRecipe::Identity,
        },
        is_send: false,
        slots: vec![
          StreamSlotIdentity {
            use_site_id: 1,
            operation_id: 9,
            kind: OperationKind::OutputStreamStart,
          },
          StreamSlotIdentity {
            use_site_id: 1,
            operation_id: 10,
            kind: OperationKind::OutputStreamNext,
          },
          StreamSlotIdentity {
            use_site_id: 1,
            operation_id: 11,
            kind: OperationKind::OutputStreamCancel,
          },
        ],
      }],
      stream_slot: Some(StreamSlotIdentity {
        use_site_id: 1,
        operation_id: 9,
        kind: OperationKind::OutputStreamStart,
      }),
    },
    FamilyOperationInput {
      id: 10,
      kind: OperationKind::OutputStreamNext,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Borrowed,
      }),
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: Some(StreamSlotIdentity {
        use_site_id: 1,
        operation_id: 10,
        kind: OperationKind::OutputStreamNext,
      }),
    },
    FamilyOperationInput {
      id: 11,
      kind: OperationKind::OutputStreamCancel,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Borrowed,
      }),
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: Some(StreamSlotIdentity {
        use_site_id: 1,
        operation_id: 11,
        kind: OperationKind::OutputStreamCancel,
      }),
    },
    FamilyOperationInput {
      id: 12,
      kind: OperationKind::OutputStreamStart,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: Some(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Owned,
      }),
      callbacks: Vec::new(),
      streams: vec![
        StreamUseSite {
          operation_id: 12,
          use_site_id: 2,
          path: ValuePath::argument(0),
          direction: StreamDirection::Input,
          item: StreamValueBinding {
            carrier: napi_family_core::CarrierKind::Primitive,
            conversion: napi_family_core::ConversionRecipe::Identity,
          },
          error: StreamValueBinding {
            carrier: napi_family_core::CarrierKind::Primitive,
            conversion: napi_family_core::ConversionRecipe::Identity,
          },
          is_send: false,
          slots: vec![
            StreamSlotIdentity {
              use_site_id: 2,
              operation_id: 15,
              kind: OperationKind::InputStreamPull,
            },
            StreamSlotIdentity {
              use_site_id: 2,
              operation_id: 16,
              kind: OperationKind::InputStreamCancel,
            },
          ],
        },
        StreamUseSite {
          operation_id: 12,
          use_site_id: 3,
          path: ValuePath::return_value(),
          direction: StreamDirection::Output,
          item: StreamValueBinding {
            carrier: napi_family_core::CarrierKind::Primitive,
            conversion: napi_family_core::ConversionRecipe::Identity,
          },
          error: StreamValueBinding {
            carrier: napi_family_core::CarrierKind::Primitive,
            conversion: napi_family_core::ConversionRecipe::Identity,
          },
          is_send: false,
          slots: vec![
            StreamSlotIdentity {
              use_site_id: 3,
              operation_id: 12,
              kind: OperationKind::OutputStreamStart,
            },
            StreamSlotIdentity {
              use_site_id: 3,
              operation_id: 13,
              kind: OperationKind::OutputStreamNext,
            },
            StreamSlotIdentity {
              use_site_id: 3,
              operation_id: 14,
              kind: OperationKind::OutputStreamCancel,
            },
          ],
        },
      ],
      stream_slot: Some(StreamSlotIdentity {
        use_site_id: 3,
        operation_id: 12,
        kind: OperationKind::OutputStreamStart,
      }),
    },
    FamilyOperationInput {
      id: 13,
      kind: OperationKind::OutputStreamNext,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Borrowed,
      }),
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: Some(StreamSlotIdentity {
        use_site_id: 3,
        operation_id: 13,
        kind: OperationKind::OutputStreamNext,
      }),
    },
    FamilyOperationInput {
      id: 14,
      kind: OperationKind::OutputStreamCancel,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Borrowed,
      }),
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: Some(StreamSlotIdentity {
        use_site_id: 3,
        operation_id: 14,
        kind: OperationKind::OutputStreamCancel,
      }),
    },
    FamilyOperationInput {
      id: 15,
      kind: OperationKind::InputStreamPull,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::InputStreamHostPull,
      receiver: Some(ResourceBinding {
        kind: ResourceKind::InputStream,
        ownership: ResourceOwnership::Borrowed,
      }),
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: Some(StreamSlotIdentity {
        use_site_id: 2,
        operation_id: 15,
        kind: OperationKind::InputStreamPull,
      }),
    },
    FamilyOperationInput {
      id: 16,
      kind: OperationKind::InputStreamCancel,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::InputStreamHostCancel,
      receiver: Some(ResourceBinding {
        kind: ResourceKind::InputStream,
        ownership: ResourceOwnership::Borrowed,
      }),
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: Some(StreamSlotIdentity {
        use_site_id: 2,
        operation_id: 16,
        kind: OperationKind::InputStreamCancel,
      }),
    },
    FamilyOperationInput {
      id: 17,
      kind: OperationKind::CallbackMethod,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::CallbackHost {
        callback_type_id: 0,
        method_id: 1,
      },
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 18,
      kind: OperationKind::CallbackMethod,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::CallbackHost {
        callback_type_id: 0,
        method_id: 2,
      },
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 19,
      kind: OperationKind::CallbackMethod,
      async_kind: AsyncKind::Sync,
      fallible: true,
      argument_count: 0,
      dispatch: OperationDispatch::CallbackHost {
        callback_type_id: 0,
        method_id: 3,
      },
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 20,
      kind: OperationKind::CallbackMethod,
      async_kind: AsyncKind::Async,
      fallible: true,
      argument_count: 0,
      dispatch: OperationDispatch::CallbackHost {
        callback_type_id: 0,
        method_id: 4,
      },
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 21,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: vec![CallbackUseSite {
        operation_id: 21,
        callback_type_id: 0,
        path: ValuePath::argument(0),
        contract: CallbackContract {
          retention: CallbackRetention::Retained,
          threading: CallbackThreading::MayCrossThread,
          reentrancy: CallbackReentrancy::Allowed,
        },
      }],
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 22,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 23,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 24,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 25,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 26,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 27,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: vec![CallbackUseSite {
        operation_id: 27,
        callback_type_id: 0,
        path: ValuePath::new(vec![
          napi_family_core::ValuePathSegment::Return,
          napi_family_core::ValuePathSegment::Field("callback".to_owned()),
        ]),
        contract: CallbackContract {
          retention: CallbackRetention::Retained,
          threading: CallbackThreading::MayCrossThread,
          reentrancy: CallbackReentrancy::Allowed,
        },
      }],
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 28,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 29,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 30,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: Some(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Owned,
      }),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 31,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 32,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result: None,
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
  ]);
  operations[5].receiver = Some(ResourceBinding {
    kind: ResourceKind::InputStream,
    ownership: ResourceOwnership::Borrowed,
  });
  operations[6].receiver = Some(ResourceBinding {
    kind: ResourceKind::InputStream,
    ownership: ResourceOwnership::Borrowed,
  });
  operations[5].stream_slot = Some(StreamSlotIdentity {
    use_site_id: 0,
    operation_id: 5,
    kind: OperationKind::InputStreamPull,
  });
  operations[6].stream_slot = Some(StreamSlotIdentity {
    use_site_id: 0,
    operation_id: 6,
    kind: OperationKind::InputStreamCancel,
  });
  FamilyPlan::build(FamilyPlanInput {
    flavor: HostFlavor::Node,
    operations,
  })
  .unwrap()
}

fn plan(family: &FamilyPlan) -> RustBridgePlan {
  let id = |value| value;
  let ops = vec![
    RustOperationPlan {
      operation_id: id(0),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::answer),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::I64BigInt,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(1),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::plus_one),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("value", Span::call_site()),
        binding: ArgumentBinding::I64BigInt,
      }],
      return_binding: ReturnBinding::I64BigInt,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(2),
      target: RustOperationTarget::CallbackHost,
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Unit,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(3),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::observe),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("observer", Span::call_site()),
        binding: ArgumentBinding::LowerWithHost {
          carrier_type: syn::parse_quote!(napi::bindgen_prelude::Object<'static>),
          lower: syn::parse_quote!(fixture::lower_nested_callback),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(4),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::consume_input),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("source", Span::call_site()),
        binding: ArgumentBinding::InputStreamProxy {
          rust_type: syn::parse_quote!(u32),
          build: syn::parse_quote!(fixture::build_input_stream_proxy),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(5),
      target: RustOperationTarget::InputStreamHostPull,
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Unit,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(6),
      target: RustOperationTarget::InputStreamHostCancel,
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Unit,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(7),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::make_object),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::ObjectLease {
        carrier_type: syn::parse_quote!(fixture::ObjectHandle),
        lift: syn::parse_quote!(fixture::lift_object),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(8),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::fail_object),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::ObjectLease {
        carrier_type: syn::parse_quote!(fixture::ObjectHandle),
        lift: syn::parse_quote!(fixture::lift_object),
      },
      error_binding: ErrorBinding::Descriptor {
        map: syn::parse_quote!(fixture::map_error),
      },
    },
    RustOperationPlan {
      operation_id: id(9),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::start_output),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::OutputStreamLease {
        carrier_type: syn::parse_quote!(fixture::OutputHandle),
        lift: syn::parse_quote!(fixture::lift_output),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(10),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::next_output),
      },
      receiver: Some(napi_uniffi_engine::RustReceiverPlan {
        name: Ident::new("stream", Span::call_site()),
        binding: ArgumentBinding::OutputStreamLease {
          carrier_type: syn::parse_quote!(u32),
          lower: syn::parse_quote!(fixture::lower_output),
          ownership: ResourceOwnership::Borrowed,
        },
      }),
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(fixture::OutputStep),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(11),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::cancel_output),
      },
      receiver: Some(napi_uniffi_engine::RustReceiverPlan {
        name: Ident::new("stream", Span::call_site()),
        binding: ArgumentBinding::OutputStreamLease {
          carrier_type: syn::parse_quote!(u32),
          lower: syn::parse_quote!(fixture::lower_output),
          ownership: ResourceOwnership::Borrowed,
        },
      }),
      arguments: Vec::new(),
      return_binding: ReturnBinding::Unit,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(12),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::start_bidi),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("source", Span::call_site()),
        binding: ArgumentBinding::InputStreamProxy {
          rust_type: syn::parse_quote!(u32),
          build: syn::parse_quote!(fixture::build_input_stream_proxy),
        },
      }],
      return_binding: ReturnBinding::OutputStreamLease {
        carrier_type: syn::parse_quote!(fixture::OutputHandle),
        lift: syn::parse_quote!(fixture::lift_output),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(13),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::next_output),
      },
      receiver: Some(napi_uniffi_engine::RustReceiverPlan {
        name: Ident::new("stream", Span::call_site()),
        binding: ArgumentBinding::OutputStreamLease {
          carrier_type: syn::parse_quote!(u32),
          lower: syn::parse_quote!(fixture::lower_output),
          ownership: ResourceOwnership::Borrowed,
        },
      }),
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(fixture::OutputStep),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(14),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::cancel_output),
      },
      receiver: Some(napi_uniffi_engine::RustReceiverPlan {
        name: Ident::new("stream", Span::call_site()),
        binding: ArgumentBinding::OutputStreamLease {
          carrier_type: syn::parse_quote!(u32),
          lower: syn::parse_quote!(fixture::lower_output),
          ownership: ResourceOwnership::Borrowed,
        },
      }),
      arguments: Vec::new(),
      return_binding: ReturnBinding::Unit,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(15),
      target: RustOperationTarget::InputStreamHostPull,
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Unit,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(16),
      target: RustOperationTarget::InputStreamHostCancel,
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Unit,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(17),
      target: RustOperationTarget::CallbackHost,
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Unit,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(18),
      target: RustOperationTarget::CallbackHost,
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Unit,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(19),
      target: RustOperationTarget::CallbackHost,
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Unit,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(20),
      target: RustOperationTarget::CallbackHost,
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Unit,
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(21),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::hold_callback),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("callback", Span::call_site()),
        binding: ArgumentBinding::CallbackProxy {
          rust_type: syn::parse_quote!(fixture::CallbackProxy),
          build: syn::parse_quote!(fixture::build_callback_proxy),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(22),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::drop_callback_holds_on_worker),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(23),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::arm_cancel_gate),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(24),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::release_cancel_gate),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(25),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::arm_callback_drop),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(26),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::release_callback_drop),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(27),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::async_callback_result),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(fixture::CallbackResult),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(28),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::arm_async_callback_result),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(29),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::release_async_callback_result),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(30),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::start_late_output),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::OutputStreamLease {
        carrier_type: syn::parse_quote!(fixture::OutputHandle),
        lift: syn::parse_quote!(fixture::lift_output),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(31),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::arm_late_output_result),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(32),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::release_late_output_result),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
  ];
  RustBridgePlan::build_with_resource_hooks(
    family,
    ops,
    RustResourceHooks {
      release_object: Some(RustResourceHook {
        call: syn::parse_quote!(fixture::release_object),
        carrier_type: syn::parse_quote!(u32),
      }),
      cancel_output_stream: Some(RustResourceHook {
        call: syn::parse_quote!(fixture::cancel_output_resource),
        carrier_type: syn::parse_quote!(u32),
      }),
      release_output_stream: Some(RustResourceHook {
        call: syn::parse_quote!(fixture::release_output),
        carrier_type: syn::parse_quote!(u32),
      }),
    },
  )
  .unwrap()
}

fn fixture_root() -> TempFixture {
  let nonce = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  let path = std::env::temp_dir().join(format!(
    "napi-uniffi-engine-addon-{}-{nonce}",
    std::process::id()
  ));
  fs::create_dir_all(path.join("src")).unwrap();
  TempFixture(path)
}

fn dylib_path(target: &Path) -> PathBuf {
  let extension = if cfg!(target_os = "macos") {
    "dylib"
  } else if cfg!(target_os = "windows") {
    "dll"
  } else {
    "so"
  };
  let prefix = if cfg!(target_os = "windows") {
    ""
  } else {
    "lib"
  };
  target
    .join("debug")
    .join(format!("{prefix}uniffi_engine_addon_fixture.{extension}"))
}

#[test]
fn generated_factory_runs_sync_bigint_async_callback_stream_and_close_in_node() {
  let family = family();
  let generated = generate_napi_module(&family, &plan(&family)).unwrap();
  let fixture = fixture_root();
  let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
    .ancestors()
    .nth(2)
    .unwrap();
  let cargo_toml = format!(
    r#"[package]
name = "uniffi-engine-addon-fixture"
version = "0.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
futures = "0.3"
napi = {{ path = {:?}, default-features = false, features = ["napi6", "async-runtime"] }}
napi-derive = {{ path = {:?}, default-features = false, features = ["type-def"] }}
napi-uniffi-engine = {{ path = {:?} }}

[build-dependencies]
napi-build = {{ path = {:?} }}
"#,
    repository.join("crates/napi"),
    repository.join("crates/macro"),
    repository.join("crates/uniffi-engine"),
    repository.join("crates/build")
  );
  fs::write(fixture.0.join("Cargo.toml"), cargo_toml).unwrap();
  fs::write(
    fixture.0.join("build.rs"),
    "fn main() { napi_build::setup(); }\n",
  )
  .unwrap();
  let source = format!(
    r#"
use std::future::Future;
use std::pin::Pin;
use std::sync::{{atomic::{{AtomicBool, Ordering}}, Mutex}};
use napi::bindgen_prelude::{{register_async_runtime, AsyncRuntime, AsyncRuntimeRejection, AsyncRuntimeTask}};

struct FixtureRuntime {{ active: AtomicBool, workers: Mutex<Vec<std::thread::JoinHandle<()>>> }}
unsafe impl AsyncRuntime for FixtureRuntime {{
  fn spawn(&self, task: AsyncRuntimeTask) -> std::result::Result<(), AsyncRuntimeRejection<AsyncRuntimeTask>> {{
    if !self.active.load(Ordering::Acquire) {{ return Err(AsyncRuntimeRejection::new(task, napi::Error::new(napi::Status::GenericFailure, "stopped"))); }}
    self.workers.lock().unwrap().push(std::thread::spawn(move || futures::executor::block_on(task)));
    Ok(())
  }}
  fn block_on(&self, future: Pin<&mut dyn Future<Output = ()>>) -> napi::Result<()> {{ futures::executor::block_on(future); Ok(()) }}
  fn start(&self) -> napi::Result<()> {{ self.active.store(true, Ordering::Release); Ok(()) }}
  fn shutdown(&self) -> napi::Result<()> {{ self.active.store(false, Ordering::Release); for worker in self.workers.lock().unwrap().drain(..) {{ let _ = worker.join(); }} Ok(()) }}
}}
#[napi_derive::module_init]
fn install_runtime() {{ register_async_runtime(FixtureRuntime {{ active: AtomicBool::new(false), workers: Mutex::new(Vec::new()) }}); }}

mod fixture {{
  use napi::bindgen_prelude::JsObjectValue;
  use napi_derive::napi;
  use std::sync::{{atomic::{{AtomicBool, AtomicU32, Ordering}}, Mutex}};

  #[napi(object)]
  pub struct ObjectHandle {{ pub handle: u32 }}
  #[napi(object)]
  pub struct OutputHandle {{ pub handle: u32 }}
  #[napi(object)]
  pub struct OutputStep {{ pub kind: String }}
  #[napi(object)]
  pub struct CallbackResult {{ pub callback: u32 }}
  static OBJECT_RELEASES: Mutex<Vec<u32>> = Mutex::new(Vec::new());
  static OUTPUT_CANCELS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
  static OUTPUT_RELEASES: Mutex<Vec<u32>> = Mutex::new(Vec::new());
  static NESTED_CALLBACK_HOLDS: Mutex<Vec<napi_uniffi_engine::SessionCallbackLease>> = Mutex::new(Vec::new());
  static DIRECT_CALLBACK_HOLDS: Mutex<Vec<CallbackProxy>> = Mutex::new(Vec::new());
  static NEXT_OUTPUT: AtomicU32 = AtomicU32::new(101);
  static CANCEL_GATE_ARMED: AtomicBool = AtomicBool::new(false);
  static CANCEL_GATE_RELEASED: AtomicBool = AtomicBool::new(true);
  static CALLBACK_DROP_ARMED: AtomicBool = AtomicBool::new(false);
  static CALLBACK_DROP_RELEASED: AtomicBool = AtomicBool::new(true);
  static CALLBACK_DROP_WORKER: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);
  static ASYNC_CALLBACK_RESULT_RELEASED: AtomicBool = AtomicBool::new(true);
  static LATE_OUTPUT_RESULT_RELEASED: AtomicBool = AtomicBool::new(true);

  pub struct CallbackProxy {{ id: u32, lease: napi_uniffi_engine::SessionCallbackLease }}

  pub fn answer() -> i64 {{ 42 }}
  pub async fn plus_one(value: i64) -> i64 {{ value + 1 }}
  pub fn build_callback_proxy(_host: &napi::bindgen_prelude::Object<'static>, callback_type_id: u32, callback_id: u32, contract: napi_uniffi_engine::SessionCallbackArgument, lease: napi_uniffi_engine::SessionCallbackLease) -> Result<CallbackProxy, napi_uniffi_engine::BridgeErrorDescriptor> {{
    assert_eq!(callback_type_id, 0); assert_eq!(contract.callback_type_id, 0); assert_eq!(contract.retention, napi_uniffi_engine::SessionCallbackRetention::Retained); assert_eq!(contract.threading, napi_uniffi_engine::SessionCallbackThreading::MayCrossThread); assert_eq!(contract.reentrancy, napi_uniffi_engine::SessionCallbackReentrancy::Allowed); Ok(CallbackProxy {{ id: callback_id, lease }})
  }}
  pub fn hold_callback(proxy: CallbackProxy) -> u32 {{
    let id = proxy.id;
    DIRECT_CALLBACK_HOLDS.lock().unwrap().push(proxy);
    id
  }}
  pub fn drop_callback_holds_on_worker() -> u32 {{
    let holds = std::mem::take(&mut *DIRECT_CALLBACK_HOLDS.lock().unwrap());
    let worker = std::thread::spawn(move || {{
      while CALLBACK_DROP_ARMED.load(Ordering::Acquire)
        && !CALLBACK_DROP_RELEASED.load(Ordering::Acquire)
      {{ std::thread::yield_now(); }}
      drop(holds);
    }});
    if CALLBACK_DROP_ARMED.load(Ordering::Acquire) {{
      *CALLBACK_DROP_WORKER.lock().unwrap() = Some(worker);
    }} else {{
      worker.join().unwrap();
    }}
    0
  }}
  pub fn lower_nested_callback(_host: &napi::bindgen_prelude::Object<'static>, value: napi::bindgen_prelude::Object<'static>, transfers: &napi_uniffi_engine::SessionCallbackTransfers) -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{
    // Transfer ownership by canonical use-site index.  Empty optional and
    // container sites must not accidentally borrow a neighboring callback;
    // wrong value indexes are rejected by the exact lease API as well.
    for index in [1usize, 2, 3, 5, 8] {{ assert!(transfers.lease(index, 0).is_err()); }}
    assert!(transfers.lease(4, 1).is_err());
    assert!(transfers.lease(6, 1).is_err());
    let mut holds = NESTED_CALLBACK_HOLDS.lock().unwrap();
    for index in [0usize, 4, 6, 7, 9] {{
      if let Ok(lease) = transfers.lease(index, 0) {{ holds.push(lease); }}
    }}
    let event = value.get_named_property::<napi::bindgen_prelude::Object<'static>>("event").map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    let tag = event.get_named_property::<String>("tag").map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    if tag != "Ready" {{ return Ok(0); }}
    event.get_named_property::<u32>("callback").map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))
  }}
  pub fn observe(callback_id: u32) -> u32 {{ callback_id }}
  pub fn build_input_stream_proxy(_host: &napi::bindgen_prelude::Object<'static>, stream_id: u32) -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{ Ok(stream_id) }}
  pub fn consume_input(stream_id: u32) -> u32 {{ stream_id }}
  pub fn make_object() -> u32 {{ 77 }}
  pub fn lift_object(handle: u32) -> Result<ObjectHandle, napi_uniffi_engine::BridgeErrorDescriptor> {{ Ok(ObjectHandle {{ handle }}) }}
  pub fn fail_object() -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{ Err(napi_uniffi_engine::BridgeErrorDescriptor::backend("object failure")) }}
  pub fn map_error(error: napi_uniffi_engine::BridgeErrorDescriptor) -> napi_uniffi_engine::BridgeErrorDescriptor {{ error }}
  pub async fn start_output() -> u32 {{ NEXT_OUTPUT.fetch_add(1, Ordering::Relaxed) }}
  pub fn lift_output(handle: u32) -> Result<OutputHandle, napi_uniffi_engine::BridgeErrorDescriptor> {{ Ok(OutputHandle {{ handle }}) }}
  pub fn lower_output(stream: u32) -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{ Ok(stream) }}
  pub async fn next_output(_stream: u32) -> OutputStep {{ OutputStep {{ kind: "done".to_owned() }} }}
  pub async fn cancel_output(_stream: u32) {{}}
  pub fn start_bidi(_source: u32) -> u32 {{ 202 }}
  pub fn release_object(handle: u32) -> Result<(), napi_uniffi_engine::BridgeErrorDescriptor> {{
    let mut seen = OBJECT_RELEASES.lock().unwrap();
    assert!(!seen.contains(&handle), "duplicate object release");
    seen.push(handle);
    Ok(())
  }}
  pub async fn cancel_output_resource(handle: u32) -> Result<(), napi_uniffi_engine::BridgeErrorDescriptor> {{
    let gated = CANCEL_GATE_ARMED.load(Ordering::Acquire);
    if gated {{
      while !CANCEL_GATE_RELEASED.load(Ordering::Acquire) {{ std::thread::yield_now(); }}
      CANCEL_GATE_ARMED.store(false, Ordering::Release);
    }}
    let mut seen = OUTPUT_CANCELS.lock().unwrap();
    assert!(!seen.contains(&handle), "duplicate output cancel");
    seen.push(handle);
    if gated {{ return Err(napi_uniffi_engine::BridgeErrorDescriptor::backend("cancel failed")); }}
    Ok(())
  }}
  pub fn release_output(handle: u32) -> Result<(), napi_uniffi_engine::BridgeErrorDescriptor> {{
    assert!(!CANCEL_GATE_ARMED.load(Ordering::Acquire), "output released before cancel settled");
    if handle == 909 {{
      assert!(OUTPUT_CANCELS.lock().unwrap().contains(&handle), "late output released before cancel hook");
    }}
    let mut seen = OUTPUT_RELEASES.lock().unwrap();
    assert!(!seen.contains(&handle), "duplicate output release");
    seen.push(handle);
    Ok(())
  }}
  pub fn arm_cancel_gate() -> u32 {{
    CANCEL_GATE_RELEASED.store(false, Ordering::Release);
    CANCEL_GATE_ARMED.store(true, Ordering::Release);
    0
  }}
  pub fn release_cancel_gate() -> u32 {{
    CANCEL_GATE_RELEASED.store(true, Ordering::Release);
    0
  }}
  pub fn arm_callback_drop() -> u32 {{
    CALLBACK_DROP_RELEASED.store(false, Ordering::Release);
    CALLBACK_DROP_ARMED.store(true, Ordering::Release);
    0
  }}
  pub fn release_callback_drop() -> u32 {{
    CALLBACK_DROP_RELEASED.store(true, Ordering::Release);
    CALLBACK_DROP_ARMED.store(false, Ordering::Release);
    if let Some(worker) = CALLBACK_DROP_WORKER.lock().unwrap().take() {{ worker.join().unwrap(); }}
    0
  }}
  pub async fn async_callback_result() -> CallbackResult {{
    while !ASYNC_CALLBACK_RESULT_RELEASED.load(Ordering::Acquire) {{ std::thread::yield_now(); }}
    CallbackResult {{ callback: 1234 }}
  }}
  pub fn arm_async_callback_result() -> u32 {{
    ASYNC_CALLBACK_RESULT_RELEASED.store(false, Ordering::Release);
    0
  }}
  pub fn release_async_callback_result() -> u32 {{
    ASYNC_CALLBACK_RESULT_RELEASED.store(true, Ordering::Release);
    0
  }}
  pub async fn start_late_output() -> u32 {{
    while !LATE_OUTPUT_RESULT_RELEASED.load(Ordering::Acquire) {{ std::thread::yield_now(); }}
    909
  }}
  pub fn arm_late_output_result() -> u32 {{
    LATE_OUTPUT_RESULT_RELEASED.store(false, Ordering::Release);
    0
  }}
  pub fn release_late_output_result() -> u32 {{
    LATE_OUTPUT_RESULT_RELEASED.store(true, Ordering::Release);
    0
  }}
}}

{}
"#,
    generated.source()
  );
  fs::write(fixture.0.join("src/lib.rs"), source).unwrap();
  let target = repository.join("target/uniffi-engine-generated-addon");
  let build = Command::new(env!("CARGO"))
    .arg("build")
    .current_dir(&fixture.0)
    .env("CARGO_TARGET_DIR", &target)
    .output()
    .unwrap();
  assert!(
    build.status.success(),
    "generated addon failed to compile:\n{}\n{}",
    String::from_utf8_lossy(&build.stdout),
    String::from_utf8_lossy(&build.stderr)
  );
  let addon = fixture.0.join("fixture.node");
  fs::copy(dylib_path(&target), &addon).unwrap();
  let script = r#"
const assert = require('node:assert/strict');
(async () => {
  const addon = require(process.argv[1]);
  assert.deepEqual(Object.keys(addon), ['__uniffi_backend_factory']);
  const retained = [];
  const releasedCallbacks = [];
  const releasedStreams = [];
  const callbackCalls = [];
  let resolvePendingPull;
  let failRetain = false;
  const host = {
    retainCallback(typeId, id) { if (failRetain) throw new Error('retain failed'); retained.push([typeId, id]); },
    releaseCallback(typeId, id) { releasedCallbacks.push([typeId, id]); const index = retained.findIndex(([t, i]) => t === typeId && i === id); if (index >= 0) retained.splice(index, 1); },
    invokeCallbackSync(typeId, id, methodId, args) { callbackCalls.push(['sync', typeId, id, methodId, args]); return { kind: 'value', value: 7 }; },
    invokeCallbackAsync(typeId, id, methodId, invocationId, args) { callbackCalls.push(['async', typeId, id, methodId, invocationId, args]); return Promise.resolve({ kind: 'value', value: 7 }); },
    pullInputStream(id) { if (id === 44) return new Promise(resolve => { resolvePendingPull = resolve; }); return Promise.resolve({ kind: 'done' }); },
    cancelInputStream(id) { return Promise.resolve(id); },
    releaseInputStream(id) { releasedStreams.push(id); },
  };
  const session = addon.__uniffi_backend_factory(host);
  assert.equal(session.hostFlavor, 'node');
  for (const operationId of [-1, 0.5, 2 ** 32]) {
    assert.throws(() => session.invokeSync(operationId, []));
  }
  assert.equal(session.invokeSync(0, []).kind, 'value');
  assert.equal(session.invokeSync(0, []).value, 42n);
  assert.equal((await session.invokeAsync(1, [41n])).value, 42n);
  session.invokeSync(17, [3]);
  await session.invokeAsync(18, [3]);
  session.invokeSync(19, [3]);
  await session.invokeAsync(20, [3]);
  assert.deepEqual(callbackCalls.map((call) => call[0] + ':' + call[3]), ['sync:1', 'async:2', 'sync:3', 'async:4']);
  assert.equal(session.invokeSync(3, [{
    event: { tag: 'Ready', callback: 9 },
    optionalNull: null,
    optionalUndefined: null,
    callbacksEmpty: [],
    callbacks: [10],
    mapEmpty: new Map(),
    map: new Map([[20, 40]]),
    setEmpty: new Set(),
    set: new Set([30]),
  }]).value, 9);
  assert.deepEqual(retained, [[0, 9], [0, 10], [0, 20], [0, 40], [0, 30]]);
  assert.equal(session.invokeSync(3, [{
    event: { tag: 'Other' },
    optionalNull: null,
    optionalUndefined: null,
    callbacksEmpty: [],
    callbacks: [],
    mapEmpty: new Map(),
    map: new Map(),
    setEmpty: new Set(),
    set: new Set(),
  }]).value, 0);
  assert.deepEqual(retained, [[0, 9], [0, 10], [0, 20], [0, 40], [0, 30]]);
  assert.equal(session.invokeSync(21, [55]).value, 55);
  assert.deepEqual(retained, [[0, 9], [0, 10], [0, 20], [0, 40], [0, 30], [0, 55]]);
  session.invokeSync(22, []);
  for (let turn = 0; turn < 10 && releasedCallbacks.length === 0; turn++) {
    await new Promise((resolve) => setTimeout(resolve, 1));
  }
  assert.deepEqual(releasedCallbacks, [[0, 55]]);
  assert.deepEqual(retained, [[0, 9], [0, 10], [0, 20], [0, 40], [0, 30]]);
  failRetain = true;
  assert.throws(() => session.invokeSync(3, [{
    event: { tag: 'Ready', callback: 99 },
    optionalNull: null,
    optionalUndefined: undefined,
    callbacksEmpty: [],
    callbacks: [],
    mapEmpty: new Map(),
    map: new Map(),
    setEmpty: new Set(),
    set: new Set(),
  }]));
  failRetain = false;
  assert.deepEqual(releasedCallbacks, [[0, 55], [0, 99]]);
  const retainedBeforeInvalidIds = retained.slice();
  for (const callbackId of [-1, 1.5, 2 ** 32]) {
    assert.throws(() => session.invokeSync(3, [{
      event: { tag: 'Ready', callback: callbackId },
      optionalNull: null,
      optionalUndefined: null,
      callbacksEmpty: [],
      callbacks: [],
      mapEmpty: new Map(),
      map: new Map(),
      setEmpty: new Set(),
      set: new Set(),
    }]));
  }
  assert.deepEqual(retained, retainedBeforeInvalidIds);
  assert.deepEqual(releasedCallbacks, [[0, 55], [0, 99]]);
  assert.throws(() => session.invokeSync(3, [{
    event: { tag: 'Ready', callback: 101 },
    optionalNull: null,
    optionalUndefined: undefined,
    callbacksEmpty: [],
    callbacks: [],
    mapEmpty: new Map(),
    map: new Map(),
    setEmpty: new Set(),
    set: new Set(),
  }]));
  assert.deepEqual(retained, retainedBeforeInvalidIds);
  assert.deepEqual(releasedCallbacks, [[0, 55], [0, 99], [0, 101]]);
  const releasedStreamsBeforeInvalidIds = releasedStreams.slice();
  for (const streamId of [-1, 1.5, 2 ** 32]) {
    assert.throws(() => session.invokeAsync(5, [streamId]));
  }
  assert.deepEqual(releasedStreams, releasedStreamsBeforeInvalidIds);
  // Receiver handle extraction happens after resource retention.  A malformed
  // receiver must roll that provisional lease back instead of leaving a
  // phantom output resource for close().
  assert.throws(() => session.invokeAsync(10, [{}]));
  assert.equal(session.invokeSync(4, [11]).value, 11);
  await session.invokeAsync(5, [11]);
  assert.deepEqual(releasedStreams, [11]);
  await session.invokeAsync(6, [11]);
  assert.deepEqual(releasedStreams, [11]);
  const object = session.invokeSync(7, []).value;
  assert.equal(object.handle, 77);
  session.releaseObject(object);
  session.releaseObject(object);
  const failedObject = session.invokeSync(8, []);
  assert.equal(failedObject.kind, 'error');
  const output = (await session.invokeAsync(9, [])).value;
  assert.equal(typeof output.handle, 'number');
  if (global.gc) { global.gc(); await new Promise((resolve) => setImmediate(resolve)); }
  const step = (await session.invokeAsync(10, [output])).value;
  assert.deepEqual(step, { kind: 'done' });
  session.releaseOutputStream(output);
  session.releaseOutputStream(output);
  const bidi = session.invokeSync(12, [22]).value;
  assert.equal(bidi.handle, 202);
  await session.invokeAsync(15, [22]);
  assert.deepEqual(releasedStreams, [11, 22]);
  await session.invokeAsync(16, [22]);
  assert.deepEqual(releasedStreams, [11, 22]);
  await session.invokeAsync(13, [bidi]);
  session.releaseOutputStream(bidi);
  const cancelled = (await session.invokeAsync(9, [])).value;
  await session.cancelOutputStream(cancelled);
  session.releaseOutputStream(cancelled);
  await session.cancelOutputStream(cancelled);
  const gateSession = addon.__uniffi_backend_factory(host);
  const cancelSession = addon.__uniffi_backend_factory(host);
  gateSession.invokeSync(23, []);
  const gatedOutput = (await cancelSession.invokeAsync(9, [])).value;
  const gatedCancel = cancelSession.cancelOutputStream(gatedOutput);
  const gatedCancelAgain = cancelSession.cancelOutputStream(gatedOutput);
  assert.strictEqual(gatedCancelAgain, gatedCancel);
  let gatedCancelAgainSettled = false;
  gatedCancelAgain.then(() => { gatedCancelAgainSettled = true; });
  cancelSession.releaseOutputStream(gatedOutput);
  let gatedCloseSettled = false;
  const gatedClose = cancelSession.close().then(() => { gatedCloseSettled = true; });
  await Promise.resolve();
  assert.equal(gatedCloseSettled, false);
  assert.equal(gatedCancelAgainSettled, false);
  gateSession.invokeSync(24, []);
  assert.equal((await gatedCancel).kind, 'error');
  await gatedCancelAgain;
  assert.equal(gatedCancelAgainSettled, true);
  await gatedClose;
  const lateOutputSession = addon.__uniffi_backend_factory(host);
  const lateOutputController = addon.__uniffi_backend_factory(host);
  lateOutputController.invokeSync(23, []);
  lateOutputController.invokeSync(31, []);
  const lateOutputResult = lateOutputSession.invokeAsync(30, []);
  let lateOutputCloseSettled = false;
  const lateOutputClose = lateOutputSession.close().then(() => { lateOutputCloseSettled = true; });
  await Promise.resolve();
  assert.equal(lateOutputCloseSettled, false);
  lateOutputController.invokeSync(32, []);
  await lateOutputResult;
  await Promise.resolve();
  assert.equal(lateOutputCloseSettled, false);
  lateOutputController.invokeSync(24, []);
  await lateOutputClose;
  assert.equal(lateOutputCloseSettled, true);
  await lateOutputController.close();
  const raceSession = addon.__uniffi_backend_factory(host);
  raceSession.invokeSync(25, []);
  assert.equal(raceSession.invokeSync(21, [66]).value, 66);
  raceSession.invokeSync(22, []);
  await raceSession.close();
  assert.equal(releasedCallbacks.filter(([typeId, id]) => typeId === 0 && id === 66).length, 1);
  gateSession.invokeSync(26, []);
  assert.equal(releasedCallbacks.filter(([typeId, id]) => typeId === 0 && id === 66).length, 1);
  await gateSession.close();
  const lateCallbackSession = addon.__uniffi_backend_factory(host);
  const lateCallbackController = addon.__uniffi_backend_factory(host);
  lateCallbackController.invokeSync(28, []);
  const retainedBeforeLateCallback = retained.slice();
  const releasedBeforeLateCallback = releasedCallbacks.slice();
  const lateCallbackResult = lateCallbackSession.invokeAsync(27, []);
  let lateCallbackCloseSettled = false;
  const lateCallbackClose = lateCallbackSession.close().then(() => { lateCallbackCloseSettled = true; });
  await Promise.resolve();
  assert.equal(lateCallbackCloseSettled, false);
  assert.deepEqual(retained, retainedBeforeLateCallback);
  assert.deepEqual(releasedCallbacks, releasedBeforeLateCallback);
  lateCallbackController.invokeSync(29, []);
  await lateCallbackResult;
  await lateCallbackClose;
  assert.equal(lateCallbackCloseSettled, true);
  assert.deepEqual(retained, retainedBeforeLateCallback);
  assert.deepEqual(releasedCallbacks, releasedBeforeLateCallback);
  await lateCallbackController.close();
  const pending = session.invokeAsync(9, []);
  await session.close();
  await pending;
  assert.equal(retained.length, 0);
  const drainingSession = addon.__uniffi_backend_factory(host);
  const drainingPull = drainingSession.invokeAsync(5, [44]);
  let drainingCloseSettled = false;
  const drainingClose = drainingSession.close().then(() => { drainingCloseSettled = true; });
  await Promise.resolve();
  assert.equal(drainingCloseSettled, false);
  resolvePendingPull({ kind: 'done' });
  await drainingPull;
  await drainingClose;
  assert.equal(drainingCloseSettled, true);
  let droppedSession = addon.__uniffi_backend_factory(host);
  const droppedOutput = droppedSession.invokeAsync(9, []);
  const droppedInput = droppedSession.invokeAsync(5, [33]);
  droppedSession = null;
  if (global.gc) global.gc();
  await droppedOutput;
  await droppedInput;
  if (global.gc) global.gc();
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(releasedStreams, [11, 22, 44, 33]);
})().catch(error => { console.error(error); process.exitCode = 1; });
"#;
  let run = Command::new("node")
    .arg("--expose-gc")
    .arg("--unhandled-rejections=strict")
    .arg("-e")
    .arg(script)
    .arg(&addon)
    .output()
    .unwrap();
  assert!(
    run.status.success(),
    "generated addon runtime failed (status={:?}):\n{}\n{}",
    run.status,
    String::from_utf8_lossy(&run.stdout),
    String::from_utf8_lossy(&run.stderr)
  );
}
