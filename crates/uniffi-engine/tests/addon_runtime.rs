use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use napi_family_core::{
  AsyncKind, CallbackContract, CallbackReentrancy, CallbackRetention, CallbackThreading,
  CallbackUseSite, ClosePolicy, DeadlineAction, FamilyOperationInput, FamilyPlan, FamilyPlanInput,
  HostFlavor, OperationDispatch, OperationKind, ReceiverBinding, ResourceBinding, ResourceKind,
  ResourceOwnership, StreamDirection, StreamSlotIdentity, StreamUseSite, StreamValueBinding,
  ValuePath,
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: vec![napi_family_core::ResultResourceUseSite {
        operation_id: 7,
        path: ValuePath::return_value(),
        binding: ResourceBinding {
          kind: ResourceKind::Object,
          ownership: ResourceOwnership::Owned,
        },
      }],
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
      result_resources: vec![napi_family_core::ResultResourceUseSite {
        operation_id: 8,
        path: ValuePath::return_value(),
        binding: ResourceBinding {
          kind: ResourceKind::Object,
          ownership: ResourceOwnership::Owned,
        },
      }],
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
      result_resources: vec![napi_family_core::ResultResourceUseSite {
        operation_id: 9,
        path: ValuePath::return_value(),
        binding: ResourceBinding {
          kind: ResourceKind::OutputStream,
          ownership: ResourceOwnership::Owned,
        },
      }],
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
      receiver: Some(ReceiverBinding::Resource(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Borrowed,
      })),
      result_resources: Vec::new(),
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
      receiver: Some(ReceiverBinding::Resource(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Borrowed,
      })),
      result_resources: Vec::new(),
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
      result_resources: vec![napi_family_core::ResultResourceUseSite {
        operation_id: 12,
        path: ValuePath::return_value(),
        binding: ResourceBinding {
          kind: ResourceKind::OutputStream,
          ownership: ResourceOwnership::Owned,
        },
      }],
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
      receiver: Some(ReceiverBinding::Resource(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Borrowed,
      })),
      result_resources: Vec::new(),
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
      receiver: Some(ReceiverBinding::Resource(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Borrowed,
      })),
      result_resources: Vec::new(),
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
      receiver: Some(ReceiverBinding::Resource(ResourceBinding {
        kind: ResourceKind::InputStream,
        ownership: ResourceOwnership::Borrowed,
      })),
      result_resources: Vec::new(),
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
      receiver: Some(ReceiverBinding::Resource(ResourceBinding {
        kind: ResourceKind::InputStream,
        ownership: ResourceOwnership::Borrowed,
      })),
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
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
      result_resources: vec![napi_family_core::ResultResourceUseSite {
        operation_id: 30,
        path: ValuePath::return_value(),
        binding: ResourceBinding {
          kind: ResourceKind::OutputStream,
          ownership: ResourceOwnership::Owned,
        },
      }],
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
      result_resources: Vec::new(),
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
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 33,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: vec![CallbackUseSite {
        operation_id: 33,
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
      id: 34,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: vec![napi_family_core::ResultResourceUseSite {
        operation_id: 34,
        path: ValuePath::return_value(),
        binding: ResourceBinding {
          kind: ResourceKind::Object,
          ownership: ResourceOwnership::Owned,
        },
      }],
      callbacks: vec![CallbackUseSite {
        operation_id: 34,
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
      id: 35,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: vec![napi_family_core::ResultResourceUseSite {
        operation_id: 35,
        path: ValuePath::return_value(),
        binding: ResourceBinding {
          kind: ResourceKind::OutputStream,
          ownership: ResourceOwnership::Owned,
        },
      }],
      callbacks: vec![CallbackUseSite {
        operation_id: 35,
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
      id: 36,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 37,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 38,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 39,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 40,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 41,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 42,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Async,
      fallible: true,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: vec![CallbackUseSite {
        operation_id: 42,
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
      id: 43,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: vec![CallbackUseSite {
        operation_id: 43,
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
      id: 44,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 45,
      kind: OperationKind::Method,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ReceiverBinding::Value),
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 46,
      kind: OperationKind::Method,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ReceiverBinding::Value),
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 47,
      kind: OperationKind::Method,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ReceiverBinding::Value),
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 48,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: vec![
        napi_family_core::ResultResourceUseSite {
          operation_id: 48,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("optionalObject".to_owned()),
            napi_family_core::ValuePathSegment::Optional,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::Object,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 48,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("objects".to_owned()),
            napi_family_core::ValuePathSegment::SequenceElement,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::Object,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 48,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("variant".to_owned()),
            napi_family_core::ValuePathSegment::Variant("Ready".to_owned()),
            napi_family_core::ValuePathSegment::Field("object".to_owned()),
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::Object,
            ownership: ResourceOwnership::Owned,
          },
        },
      ],
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 49,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: vec![
        napi_family_core::ResultResourceUseSite {
          operation_id: 49,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("optionalOutput".to_owned()),
            napi_family_core::ValuePathSegment::Optional,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::OutputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 49,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("outputs".to_owned()),
            napi_family_core::ValuePathSegment::SequenceElement,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::OutputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 49,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("variant".to_owned()),
            napi_family_core::ValuePathSegment::Variant("Ready".to_owned()),
            napi_family_core::ValuePathSegment::Field("output".to_owned()),
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::OutputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
      ],
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 50,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: vec![
        napi_family_core::ResultResourceUseSite {
          operation_id: 50,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("optionalObject".to_owned()),
            napi_family_core::ValuePathSegment::Optional,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::Object,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 50,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("objects".to_owned()),
            napi_family_core::ValuePathSegment::SequenceElement,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::Object,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 50,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("variant".to_owned()),
            napi_family_core::ValuePathSegment::Variant("Ready".to_owned()),
            napi_family_core::ValuePathSegment::Field("object".to_owned()),
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::Object,
            ownership: ResourceOwnership::Owned,
          },
        },
      ],
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 51,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: vec![
        napi_family_core::ResultResourceUseSite {
          operation_id: 51,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("optionalOutput".to_owned()),
            napi_family_core::ValuePathSegment::Optional,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::OutputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 51,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("outputs".to_owned()),
            napi_family_core::ValuePathSegment::SequenceElement,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::OutputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 51,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("variant".to_owned()),
            napi_family_core::ValuePathSegment::Variant("Ready".to_owned()),
            napi_family_core::ValuePathSegment::Field("output".to_owned()),
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::OutputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
      ],
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 52,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: vec![
        napi_family_core::ResultResourceUseSite {
          operation_id: 52,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("optionalInput".to_owned()),
            napi_family_core::ValuePathSegment::Optional,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::InputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 52,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("inputs".to_owned()),
            napi_family_core::ValuePathSegment::SequenceElement,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::InputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 52,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("variant".to_owned()),
            napi_family_core::ValuePathSegment::Variant("Ready".to_owned()),
            napi_family_core::ValuePathSegment::Field("input".to_owned()),
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::InputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
      ],
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 53,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: vec![
        napi_family_core::ResultResourceUseSite {
          operation_id: 53,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("optionalInput".to_owned()),
            napi_family_core::ValuePathSegment::Optional,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::InputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 53,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("inputs".to_owned()),
            napi_family_core::ValuePathSegment::SequenceElement,
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::InputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
        napi_family_core::ResultResourceUseSite {
          operation_id: 53,
          path: ValuePath::new(vec![
            napi_family_core::ValuePathSegment::Return,
            napi_family_core::ValuePathSegment::Field("variant".to_owned()),
            napi_family_core::ValuePathSegment::Variant("Ready".to_owned()),
            napi_family_core::ValuePathSegment::Field("input".to_owned()),
          ]),
          binding: ResourceBinding {
            kind: ResourceKind::InputStream,
            ownership: ResourceOwnership::Owned,
          },
        },
      ],
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 54,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Async,
      fallible: true,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: vec![CallbackUseSite {
        operation_id: 54,
        callback_type_id: 0,
        path: ValuePath::argument(0),
        contract: CallbackContract {
          retention: CallbackRetention::Scoped,
          threading: CallbackThreading::MayCrossThread,
          reentrancy: CallbackReentrancy::Allowed,
        },
      }],
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 55,
      kind: OperationKind::Function,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 1,
      dispatch: OperationDispatch::Native,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: vec![CallbackUseSite {
        operation_id: 55,
        callback_type_id: 0,
        path: ValuePath::argument(0),
        contract: CallbackContract {
          retention: CallbackRetention::Scoped,
          threading: CallbackThreading::CallingThread,
          reentrancy: CallbackReentrancy::Allowed,
        },
      }],
      streams: Vec::new(),
      stream_slot: None,
    },
  ]);
  operations.extend([
    FamilyOperationInput {
      id: 56,
      kind: OperationKind::OutputStreamNext,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ReceiverBinding::Resource(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Borrowed,
      })),
      result_resources: vec![napi_family_core::ResultResourceUseSite {
        operation_id: 56,
        path: ValuePath::new(vec![
          napi_family_core::ValuePathSegment::Return,
          napi_family_core::ValuePathSegment::StreamItem,
        ]),
        binding: ResourceBinding {
          kind: ResourceKind::Object,
          ownership: ResourceOwnership::Owned,
        },
      }],
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 57,
      kind: OperationKind::OutputStreamNext,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ReceiverBinding::Resource(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Borrowed,
      })),
      result_resources: vec![napi_family_core::ResultResourceUseSite {
        operation_id: 57,
        path: ValuePath::new(vec![
          napi_family_core::ValuePathSegment::Return,
          napi_family_core::ValuePathSegment::StreamError,
        ]),
        binding: ResourceBinding {
          kind: ResourceKind::Object,
          ownership: ResourceOwnership::Owned,
        },
      }],
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
    FamilyOperationInput {
      id: 58,
      kind: OperationKind::OutputStreamNext,
      async_kind: AsyncKind::Async,
      fallible: false,
      argument_count: 0,
      dispatch: OperationDispatch::Native,
      receiver: Some(ReceiverBinding::Resource(ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Borrowed,
      })),
      result_resources: vec![napi_family_core::ResultResourceUseSite {
        operation_id: 58,
        path: ValuePath::new(vec![
          napi_family_core::ValuePathSegment::Return,
          napi_family_core::ValuePathSegment::StreamItem,
        ]),
        binding: ResourceBinding {
          kind: ResourceKind::Object,
          ownership: ResourceOwnership::Owned,
        },
      }],
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    },
  ]);
  operations[5].receiver = Some(ReceiverBinding::Resource(ResourceBinding {
    kind: ResourceKind::InputStream,
    ownership: ResourceOwnership::Borrowed,
  }));
  operations[6].receiver = Some(ReceiverBinding::Resource(ResourceBinding {
    kind: ResourceKind::InputStream,
    ownership: ResourceOwnership::Borrowed,
  }));
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
    close_policy: ClosePolicy {
      // The canonical frontend supplies the production default.  This addon
      // intentionally installs a short explicit policy to exercise deadline
      // detach without making the real Node test wait seconds.
      grace_ms: 40,
      on_deadline: DeadlineAction::Detach,
    },
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
    RustOperationPlan {
      operation_id: id(33),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::invoke_host_late),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("probe", Span::call_site()),
        binding: ArgumentBinding::CallbackProxy {
          rust_type: syn::parse_quote!(fixture::HostCallProxy),
          build: syn::parse_quote!(fixture::build_host_call_proxy),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(34),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::make_object_reentrant),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("probe", Span::call_site()),
        binding: ArgumentBinding::CallbackProxy {
          rust_type: syn::parse_quote!(fixture::HostCallProxy),
          build: syn::parse_quote!(fixture::build_reentrant_proxy),
        },
      }],
      return_binding: ReturnBinding::ObjectLease {
        carrier_type: syn::parse_quote!(fixture::ObjectHandle),
        lift: syn::parse_quote!(fixture::lift_object),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(35),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::make_output_reentrant),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("probe", Span::call_site()),
        binding: ArgumentBinding::CallbackProxy {
          rust_type: syn::parse_quote!(fixture::HostCallProxy),
          build: syn::parse_quote!(fixture::build_reentrant_proxy),
        },
      }],
      return_binding: ReturnBinding::OutputStreamLease {
        carrier_type: syn::parse_quote!(fixture::OutputHandle),
        lift: syn::parse_quote!(fixture::lift_output),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(36),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::arm_host_proxy_gate),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(37),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::release_host_proxy_gate),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(38),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::call_held_host),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(39),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::object_release_count),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("handle", Span::call_site()),
        binding: ArgumentBinding::Direct {
          carrier_type: syn::parse_quote!(u32),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(40),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::output_cancel_count),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("handle", Span::call_site()),
        binding: ArgumentBinding::Direct {
          carrier_type: syn::parse_quote!(u32),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(41),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::output_release_count),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("handle", Span::call_site()),
        binding: ArgumentBinding::Direct {
          carrier_type: syn::parse_quote!(u32),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(42),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::invoke_host_late_fallible),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("probe", Span::call_site()),
        binding: ArgumentBinding::CallbackProxy {
          rust_type: syn::parse_quote!(fixture::HostCallProxy),
          build: syn::parse_quote!(fixture::build_host_call_proxy),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Descriptor {
        map: syn::parse_quote!(fixture::map_error),
      },
    },
    RustOperationPlan {
      operation_id: id(43),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::invoke_two_host_methods_late),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("probe", Span::call_site()),
        binding: ArgumentBinding::CallbackProxy {
          rust_type: syn::parse_quote!(fixture::TwoHostCallProxy),
          build: syn::parse_quote!(fixture::build_two_host_call_proxy),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(44),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::release_two_host_methods_gate),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(45),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::record_value_method),
      },
      receiver: Some(napi_uniffi_engine::RustReceiverPlan {
        name: Ident::new("record", Span::call_site()),
        binding: ArgumentBinding::LowerWith {
          carrier_type: syn::parse_quote!(napi::bindgen_prelude::Object<'static>),
          lower: syn::parse_quote!(fixture::lower_record_value),
        },
      }),
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(46),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::enum_value_method),
      },
      receiver: Some(napi_uniffi_engine::RustReceiverPlan {
        name: Ident::new("value", Span::call_site()),
        binding: ArgumentBinding::LowerWith {
          carrier_type: syn::parse_quote!(napi::bindgen_prelude::Object<'static>),
          lower: syn::parse_quote!(fixture::lower_enum_value),
        },
      }),
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(47),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::async_record_value_method),
      },
      receiver: Some(napi_uniffi_engine::RustReceiverPlan {
        name: Ident::new("record", Span::call_site()),
        binding: ArgumentBinding::LowerWith {
          carrier_type: syn::parse_quote!(napi::bindgen_prelude::Object<'static>),
          lower: syn::parse_quote!(fixture::lower_record_value),
        },
      }),
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(48),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::make_nested_object),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("null_optional", Span::call_site()),
        binding: ArgumentBinding::Direct {
          carrier_type: syn::parse_quote!(u32),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(fixture::NestedObjectResult),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(49),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::make_nested_output),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(fixture::NestedOutputResult),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(50),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::late_nested_object),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(fixture::NestedObjectResult),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(51),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::late_nested_output),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(fixture::NestedOutputResult),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(52),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::late_nested_input),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(fixture::NestedInputResult),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(53),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::make_nested_input),
      },
      receiver: None,
      arguments: Vec::new(),
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(fixture::NestedInputResult),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(54),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::invoke_scoped_callback),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("callback", Span::call_site()),
        binding: ArgumentBinding::CallbackProxy {
          rust_type: syn::parse_quote!(fixture::ScopedAsyncProxy),
          build: syn::parse_quote!(fixture::build_scoped_async_proxy),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Descriptor {
        map: syn::parse_quote!(fixture::map_error),
      },
    },
    RustOperationPlan {
      operation_id: id(55),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::hold_callback),
      },
      receiver: None,
      arguments: vec![RustArgumentPlan {
        name: Ident::new("callback", Span::call_site()),
        binding: ArgumentBinding::CallbackProxy {
          rust_type: syn::parse_quote!(fixture::CallbackProxy),
          build: syn::parse_quote!(fixture::build_returned_callback_proxy),
        },
      }],
      return_binding: ReturnBinding::Direct {
        carrier_type: syn::parse_quote!(u32),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(56),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::next_output_object_item),
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
        carrier_type: syn::parse_quote!(fixture::OutputObjectItemStep),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(57),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::next_output_object_error),
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
        carrier_type: syn::parse_quote!(fixture::OutputObjectErrorStep),
      },
      error_binding: ErrorBinding::Infallible,
    },
    RustOperationPlan {
      operation_id: id(58),
      target: RustOperationTarget::Native {
        call: syn::parse_quote!(fixture::late_output_object_item),
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
        carrier_type: syn::parse_quote!(fixture::OutputObjectItemStep),
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
  use napi::bindgen_prelude::{{Function, FnArgs, JsObjectValue, Object, Promise}};
  use napi::threadsafe_function::{{ThreadsafeFunction, ThreadsafeFunctionCallMode}};
  use napi_derive::napi;
  use std::sync::{{atomic::{{AtomicBool, AtomicU32, Ordering}}, Arc, Mutex}};

  #[napi(object)]
  pub struct ObjectHandle {{ pub handle: u32 }}
  #[napi(object)]
  pub struct OutputHandle {{ pub handle: u32 }}
  #[napi(object)]
  pub struct OutputStep {{ pub kind: String }}
  #[napi(object)]
  pub struct OutputObjectItemStep {{ pub kind: String, pub value: ObjectHandle }}
  #[napi(object)]
  pub struct OutputObjectErrorStep {{ pub kind: String, pub error: ObjectHandle }}
  #[napi(object)]
  pub struct CallbackResult {{ pub callback: u32 }}
  #[napi(object)]
  pub struct NestedObjectVariant {{ pub tag: String, pub object: ObjectHandle }}
  #[napi(object, use_nullable = true)]
  pub struct NestedObjectResult {{
    pub optional_object: Option<ObjectHandle>,
    pub objects: Vec<ObjectHandle>,
    pub variant: NestedObjectVariant,
  }}
  #[napi(object)]
  pub struct NestedOutputVariant {{ pub tag: String, pub output: OutputHandle }}
  #[napi(object)]
  pub struct NestedOutputResult {{
    pub optional_output: Option<OutputHandle>,
    pub outputs: Vec<OutputHandle>,
    pub variant: NestedOutputVariant,
  }}
  #[napi(object)]
  pub struct NestedInputVariant {{ pub tag: String, pub input: u32 }}
  #[napi(object)]
  pub struct NestedInputResult {{
    pub optional_input: Option<u32>,
    pub inputs: Vec<u32>,
    pub variant: NestedInputVariant,
  }}
  #[napi(object)]
  pub struct CallbackEnvelope {{ pub kind: String, pub value: u32 }}
  static OBJECT_RELEASES: Mutex<Vec<u32>> = Mutex::new(Vec::new());
  static OUTPUT_CANCELS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
  static OUTPUT_RELEASES: Mutex<Vec<u32>> = Mutex::new(Vec::new());
  static NESTED_CALLBACK_HOLDS: Mutex<Vec<napi_uniffi_engine::SessionCallbackLease>> = Mutex::new(Vec::new());
  static DIRECT_CALLBACK_HOLDS: Mutex<Vec<CallbackProxy>> = Mutex::new(Vec::new());
  static NEXT_OUTPUT: AtomicU32 = AtomicU32::new(101);
  static NEXT_STREAM_OBJECT: AtomicU32 = AtomicU32::new(1001);
  static CANCEL_GATE_ARMED: AtomicBool = AtomicBool::new(false);
  static CANCEL_GATE_RELEASED: AtomicBool = AtomicBool::new(true);
  static CALLBACK_DROP_ARMED: AtomicBool = AtomicBool::new(false);
  static CALLBACK_DROP_RELEASED: AtomicBool = AtomicBool::new(true);
  static CALLBACK_DROP_WORKER: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);
  static ASYNC_CALLBACK_RESULT_RELEASED: AtomicBool = AtomicBool::new(true);
  static LATE_OUTPUT_RESULT_RELEASED: AtomicBool = AtomicBool::new(true);
  static HOST_PROXY_GATE_RELEASED: AtomicBool = AtomicBool::new(true);
  static HELD_HOST_PROXY: Mutex<Option<HostCallProxy>> = Mutex::new(None);
  static TWO_HOST_METHODS_RELEASED: AtomicBool = AtomicBool::new(false);
  static HELD_TWO_HOST_METHOD: Mutex<Option<(ThreadsafeFunction<u32, (), u32, napi::Status, false>, u32)>> = Mutex::new(None);

  pub struct CallbackProxy {{ id: u32, lease: napi_uniffi_engine::SessionCallbackLease }}
  struct HostCallProxyInner {{ id: u32, call: ThreadsafeFunction<u32, (), u32, napi::Status, false>, _lease: napi_uniffi_engine::SessionCallbackLease }}
  #[derive(Clone)]
  pub struct HostCallProxy {{ inner: Arc<HostCallProxyInner> }}
  struct TwoHostCallProxyInner {{ id: u32, first: Mutex<Option<ThreadsafeFunction<u32, (), u32, napi::Status, false>>>, second: Mutex<Option<ThreadsafeFunction<u32, (), u32, napi::Status, false>>>, _lease: napi_uniffi_engine::SessionCallbackLease }}
  pub struct TwoHostCallProxy {{ inner: Arc<TwoHostCallProxyInner> }}
  type ScopedCallbackArgs = FnArgs<(u32, u32, u32, u32, Vec<u32>)>;
  type ScopedCallbackTsfn = ThreadsafeFunction<ScopedCallbackArgs, Promise<CallbackEnvelope>, ScopedCallbackArgs, napi::Status, false>;
  pub struct ScopedAsyncProxy {{ id: u32, invoker: napi_uniffi_engine::SessionCallbackInvoker, callback: Arc<ScopedCallbackTsfn> }}

  pub struct RecordValue {{ amount: u32 }}
  pub enum EnumValue {{ Ready(u32), Other }}

  pub fn answer() -> i64 {{ 42 }}
  pub async fn plus_one(value: i64) -> i64 {{ value + 1 }}
  fn nested_object(base: u32, optional: Option<ObjectHandle>) -> NestedObjectResult {{
    NestedObjectResult {{
      optional_object: optional,
      objects: vec![ObjectHandle {{ handle: base + 1 }}, ObjectHandle {{ handle: base + 2 }}],
      variant: NestedObjectVariant {{
        tag: "Ready".to_owned(),
        object: ObjectHandle {{ handle: base + 4 }},
      }},
    }}
  }}
  fn nested_output(base: u32) -> NestedOutputResult {{
    NestedOutputResult {{
      optional_output: Some(OutputHandle {{ handle: base }}),
      outputs: vec![OutputHandle {{ handle: base + 1 }}, OutputHandle {{ handle: base + 2 }}],
      variant: NestedOutputVariant {{
        tag: "Ready".to_owned(),
        output: OutputHandle {{ handle: base + 3 }},
      }},
    }}
  }}
  fn nested_input(base: u32) -> NestedInputResult {{
    NestedInputResult {{
      optional_input: Some(base),
      inputs: vec![base + 1, base + 2],
      variant: NestedInputVariant {{ tag: "Ready".to_owned(), input: base + 3 }},
    }}
  }}
  pub fn make_nested_object(null_optional: u32) -> NestedObjectResult {{
    let base = if null_optional == 0 {{ 400 }} else {{ 410 }};
    let optional = (null_optional == 0).then(|| ObjectHandle {{ handle: base + 3 }});
    nested_object(base, optional)
  }}
  pub fn make_nested_output() -> NestedOutputResult {{ nested_output(500) }}
  pub fn make_nested_input() -> NestedInputResult {{ nested_input(600) }}
  pub async fn late_nested_object() -> NestedObjectResult {{
    while !LATE_OUTPUT_RESULT_RELEASED.load(Ordering::Acquire) {{ std::thread::yield_now(); }}
    nested_object(420, Some(ObjectHandle {{ handle: 423 }}))
  }}
  pub async fn late_nested_output() -> NestedOutputResult {{
    while !LATE_OUTPUT_RESULT_RELEASED.load(Ordering::Acquire) {{ std::thread::yield_now(); }}
    nested_output(510)
  }}
  pub async fn late_nested_input() -> NestedInputResult {{
    while !LATE_OUTPUT_RESULT_RELEASED.load(Ordering::Acquire) {{ std::thread::yield_now(); }}
    nested_input(610)
  }}
  pub fn lower_record_value(value: Object<'static>) -> Result<RecordValue, napi_uniffi_engine::BridgeErrorDescriptor> {{
    let amount = value.get_named_property::<u32>("amount").map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    Ok(RecordValue {{ amount }})
  }}
  pub fn record_value_method(value: RecordValue) -> u32 {{ value.amount + 1 }}
  pub async fn async_record_value_method(value: RecordValue) -> u32 {{ value.amount + 3 }}
  pub fn lower_enum_value(value: Object<'static>) -> Result<EnumValue, napi_uniffi_engine::BridgeErrorDescriptor> {{
    let tag = value.get_named_property::<String>("tag").map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    if tag == "Ready" {{
      let amount = value.get_named_property::<u32>("amount").map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
      Ok(EnumValue::Ready(amount))
    }} else {{
      Ok(EnumValue::Other)
    }}
  }}
  pub fn enum_value_method(value: EnumValue) -> u32 {{ match value {{ EnumValue::Ready(amount) => amount + 2, EnumValue::Other => 0 }} }}
  pub fn build_callback_proxy(_host: &napi::bindgen_prelude::Object<'static>, callback_type_id: u32, callback_id: u32, contract: napi_uniffi_engine::SessionCallbackArgument, _invoker: napi_uniffi_engine::SessionCallbackInvoker, lease: napi_uniffi_engine::SessionCallbackLease) -> Result<CallbackProxy, napi_uniffi_engine::BridgeErrorDescriptor> {{
    assert_eq!(callback_type_id, 0); assert_eq!(contract.callback_type_id, 0); assert_eq!(contract.retention, napi_uniffi_engine::SessionCallbackRetention::Retained); assert_eq!(contract.threading, napi_uniffi_engine::SessionCallbackThreading::MayCrossThread); assert_eq!(contract.reentrancy, napi_uniffi_engine::SessionCallbackReentrancy::Allowed); Ok(CallbackProxy {{ id: callback_id, lease }})
  }}
  pub fn build_scoped_async_proxy(host: &Object<'static>, callback_type_id: u32, callback_id: u32, contract: napi_uniffi_engine::SessionCallbackArgument, invoker: napi_uniffi_engine::SessionCallbackInvoker) -> Result<ScopedAsyncProxy, napi_uniffi_engine::BridgeErrorDescriptor> {{
    assert_eq!(callback_type_id, 0); assert_eq!(contract.callback_type_id, 0); assert_eq!(contract.retention, napi_uniffi_engine::SessionCallbackRetention::Scoped); assert_eq!(contract.threading, napi_uniffi_engine::SessionCallbackThreading::MayCrossThread); assert_eq!(contract.reentrancy, napi_uniffi_engine::SessionCallbackReentrancy::Allowed);
    let callback = host.get_named_property::<Function<'static, ScopedCallbackArgs, Promise<CallbackEnvelope>>>("invokeCallbackAsync").map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?.build_threadsafe_function().build().map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    Ok(ScopedAsyncProxy {{ id: callback_id, invoker, callback: Arc::new(callback) }})
  }}
  pub fn build_returned_callback_proxy(_host: &Object<'static>, callback_type_id: u32, callback_id: u32, contract: napi_uniffi_engine::SessionCallbackArgument, invoker: napi_uniffi_engine::SessionCallbackInvoker) -> Result<CallbackProxy, napi_uniffi_engine::BridgeErrorDescriptor> {{
    assert_eq!(callback_type_id, 0); assert_eq!(contract.callback_type_id, 0); assert_eq!(contract.retention, napi_uniffi_engine::SessionCallbackRetention::Scoped); assert_eq!(contract.threading, napi_uniffi_engine::SessionCallbackThreading::CallingThread); assert_eq!(contract.reentrancy, napi_uniffi_engine::SessionCallbackReentrancy::Allowed);
    let lease = invoker.retain_returned_callback(callback_type_id, callback_id).map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::backend(error.to_string()))?;
    Ok(CallbackProxy {{ id: callback_id, lease }})
  }}
  pub async fn invoke_scoped_callback(proxy: ScopedAsyncProxy) -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{
    let invocation_id = proxy.invoker.next_invocation_id().map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::backend(error.to_string()))?;
    let result = proxy.callback.call_async(ScopedCallbackArgs::from((0, proxy.id, 0, invocation_id, vec![proxy.id]))).await.map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::backend(error.to_string()))?;
    Ok(result.await.map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::backend(error.to_string()))?.value)
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
  pub fn build_host_call_proxy(host: &Object<'static>, _callback_type_id: u32, callback_id: u32, _contract: napi_uniffi_engine::SessionCallbackArgument, _invoker: napi_uniffi_engine::SessionCallbackInvoker, lease: napi_uniffi_engine::SessionCallbackLease) -> Result<HostCallProxy, napi_uniffi_engine::BridgeErrorDescriptor> {{
    let method = host.get_named_property::<Function<'static, u32, ()>>("recordHostCall").map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    let call = method.build_threadsafe_function().callee_handled::<false>().build_callback(|context| Ok(context.value)).map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    Ok(HostCallProxy {{ inner: Arc::new(HostCallProxyInner {{ id: callback_id, call, _lease: lease }}) }})
  }}
  pub fn build_two_host_call_proxy(host: &Object<'static>, _callback_type_id: u32, callback_id: u32, _contract: napi_uniffi_engine::SessionCallbackArgument, _invoker: napi_uniffi_engine::SessionCallbackInvoker, lease: napi_uniffi_engine::SessionCallbackLease) -> Result<TwoHostCallProxy, napi_uniffi_engine::BridgeErrorDescriptor> {{
    // Accessing the same property twice creates two method wrappers on one
    // invocation Proxy.  The first TSFN is dropped while the second remains
    // live so an early method finalizer cannot invalidate its host target.
    let first_method = host.get_named_property::<Function<'static, u32, ()>>("recordHostCall").map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    let first = first_method.build_threadsafe_function().callee_handled::<false>().build_callback(|context| Ok(context.value)).map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    let second_method = host.get_named_property::<Function<'static, u32, ()>>("recordHostCall").map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    let second = second_method.build_threadsafe_function().callee_handled::<false>().build_callback(|context| Ok(context.value)).map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    Ok(TwoHostCallProxy {{ inner: Arc::new(TwoHostCallProxyInner {{ id: callback_id, first: Mutex::new(Some(first)), second: Mutex::new(Some(second)), _lease: lease }}) }})
  }}
  pub fn build_reentrant_proxy(host: &Object<'static>, callback_type_id: u32, callback_id: u32, contract: napi_uniffi_engine::SessionCallbackArgument, invoker: napi_uniffi_engine::SessionCallbackInvoker, lease: napi_uniffi_engine::SessionCallbackLease) -> Result<HostCallProxy, napi_uniffi_engine::BridgeErrorDescriptor> {{
    let proxy = build_host_call_proxy(host, callback_type_id, callback_id, contract, invoker, lease)?;
    let close = host.get_named_property::<Function<'static, (), ()>>("reenterClose").map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()))?;
    close.call(()).map_err(|error| napi_uniffi_engine::BridgeErrorDescriptor::backend(error.to_string()))?;
    Ok(proxy)
  }}
  pub async fn invoke_host_late(proxy: HostCallProxy) -> u32 {{
    while !HOST_PROXY_GATE_RELEASED.load(Ordering::Acquire) {{ std::thread::yield_now(); }}
    HELD_HOST_PROXY.lock().unwrap().replace(proxy.clone());
    let _ = proxy.inner.call.call_async(proxy.inner.id).await;
    proxy.inner.id
  }}
  pub async fn invoke_host_late_fallible(proxy: HostCallProxy) -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{
    let id = invoke_host_late(proxy).await;
    if id == 608 {{
      Err(napi_uniffi_engine::BridgeErrorDescriptor::backend("fallible Host fixture error"))
    }} else {{
      Ok(id)
    }}
  }}
  pub async fn invoke_two_host_methods_late(proxy: TwoHostCallProxy) -> u32 {{
    while !HOST_PROXY_GATE_RELEASED.load(Ordering::Acquire) {{ std::thread::yield_now(); }}
    let first = proxy.inner.first.lock().unwrap().take().expect("first Host method");
    let second = proxy.inner.second.lock().unwrap().take().expect("second Host method");
    let id = proxy.inner.id;
    HELD_TWO_HOST_METHOD.lock().unwrap().replace((second, id));
    drop(first);
    drop(proxy);
    while !TWO_HOST_METHODS_RELEASED.load(Ordering::Acquire) {{ std::thread::yield_now(); }}
    let held = HELD_TWO_HOST_METHOD.lock().unwrap().take();
    if let Some((second, id)) = held {{
      let _ = second.call_async(id).await;
    }}
    id
  }}
  pub fn arm_host_proxy_gate() -> u32 {{ HOST_PROXY_GATE_RELEASED.store(false, Ordering::Release); TWO_HOST_METHODS_RELEASED.store(false, Ordering::Release); 0 }}
  pub fn release_host_proxy_gate() -> u32 {{ HOST_PROXY_GATE_RELEASED.store(true, Ordering::Release); 0 }}
  pub fn release_two_host_methods_gate() -> u32 {{ TWO_HOST_METHODS_RELEASED.store(true, Ordering::Release); 0 }}
  pub fn call_held_host() -> u32 {{
    if let Some(proxy) = HELD_HOST_PROXY.lock().unwrap().take() {{
      let _ = proxy.inner.call.call(proxy.inner.id, ThreadsafeFunctionCallMode::NonBlocking);
    }}
    0
  }}
  pub fn make_object_reentrant(_proxy: HostCallProxy) -> u32 {{ 808 }}
  pub fn make_output_reentrant(_proxy: HostCallProxy) -> u32 {{ 910 }}
  pub fn object_release_count(handle: u32) -> u32 {{ OBJECT_RELEASES.lock().unwrap().iter().filter(|value| **value == handle).count() as u32 }}
  pub fn output_cancel_count(handle: u32) -> u32 {{ OUTPUT_CANCELS.lock().unwrap().iter().filter(|value| **value == handle).count() as u32 }}
  pub fn output_release_count(handle: u32) -> u32 {{ OUTPUT_RELEASES.lock().unwrap().iter().filter(|value| **value == handle).count() as u32 }}
  pub fn consume_input(stream_id: u32) -> u32 {{ stream_id }}
  pub fn make_object() -> u32 {{ 77 }}
  pub fn lift_object(handle: u32) -> Result<ObjectHandle, napi_uniffi_engine::BridgeErrorDescriptor> {{ Ok(ObjectHandle {{ handle }}) }}
  pub fn fail_object() -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{ Err(napi_uniffi_engine::BridgeErrorDescriptor::backend("object failure")) }}
  pub fn map_error(error: napi_uniffi_engine::BridgeErrorDescriptor) -> napi_uniffi_engine::BridgeErrorDescriptor {{ error }}
  pub async fn start_output() -> u32 {{ NEXT_OUTPUT.fetch_add(1, Ordering::Relaxed) }}
  pub fn lift_output(handle: u32) -> Result<OutputHandle, napi_uniffi_engine::BridgeErrorDescriptor> {{ Ok(OutputHandle {{ handle }}) }}
  pub fn lower_output(stream: u32) -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{ Ok(stream) }}
  pub async fn next_output(_stream: u32) -> OutputStep {{ OutputStep {{ kind: "done".to_owned() }} }}
  pub async fn next_output_object_item(_stream: u32) -> OutputObjectItemStep {{
    OutputObjectItemStep {{ kind: "item".to_owned(), value: ObjectHandle {{ handle: NEXT_STREAM_OBJECT.fetch_add(1, Ordering::Relaxed) }} }}
  }}
  pub async fn next_output_object_error(_stream: u32) -> OutputObjectErrorStep {{
    OutputObjectErrorStep {{ kind: "error".to_owned(), error: ObjectHandle {{ handle: NEXT_STREAM_OBJECT.fetch_add(1, Ordering::Relaxed) }} }}
  }}
  pub async fn late_output_object_item(_stream: u32) -> OutputObjectItemStep {{
    while !LATE_OUTPUT_RESULT_RELEASED.load(Ordering::Acquire) {{ std::thread::yield_now(); }}
    OutputObjectItemStep {{ kind: "item".to_owned(), value: ObjectHandle {{ handle: 3001 }} }}
  }}
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
    if handle == 909 || handle == 910 {{
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
const originalSetTimeout = global.setTimeout;
const originalClearTimeout = global.clearTimeout;
const teardownTimerHandles = new Set();
let teardownTimersCreated = 0;
let teardownTimersCleared = 0;
global.setTimeout = function(callback, delay, ...args) {
  const timer = originalSetTimeout.call(this, callback, delay, ...args);
  if (delay === 40) {
    teardownTimersCreated += 1;
    teardownTimerHandles.add(timer);
  }
  return timer;
};
global.clearTimeout = function(timer) {
  if (teardownTimerHandles.delete(timer)) teardownTimersCleared += 1;
  return originalClearTimeout.call(this, timer);
};
const timerSnapshot = () => ({ created: teardownTimersCreated, cleared: teardownTimersCleared });
const assertOneTeardownTimer = (before, label) => {
  const after = timerSnapshot();
  assert.equal(after.created - before.created, 1, `${label}: exactly one 40ms timer created`);
  assert.equal(after.cleared - before.cleared, 1, `${label}: exactly one 40ms timer cleared`);
};
(async () => {
  const addon = require(process.argv[1]);
  assert.deepEqual(Object.keys(addon), ['__uniffi_backend_factory']);
  const retained = [];
  const releasedCallbacks = [];
  const releasedStreams = [];
  const callbackCalls = [];
  const hostProxyCalls = [];
  let resolvePendingPull;
  let reentrantSession;
  let reentrantClosePromise;
  let failRetain = false;
  const host = {
    retainCallback(typeId, id) { if (failRetain) throw new Error('retain failed'); retained.push([typeId, id]); },
    releaseCallback(typeId, id) { releasedCallbacks.push([typeId, id]); const index = retained.findIndex(([t, i]) => t === typeId && i === id); if (index >= 0) retained.splice(index, 1); },
    invokeCallbackSync(typeId, id, methodId, args) { callbackCalls.push(['sync', typeId, id, methodId, args]); return { kind: 'value', value: 7 }; },
    invokeCallbackAsync(typeId, id, methodId, invocationId, args) { callbackCalls.push(['async', typeId, id, methodId, invocationId, args]); return Promise.resolve({ kind: 'value', value: 7 }); },
    pullInputStream(id) { if (id === 44) return new Promise(resolve => { resolvePendingPull = resolve; }); return Promise.resolve({ kind: 'done' }); },
    cancelInputStream(id) { return Promise.resolve(id); },
    releaseInputStream(id) { releasedStreams.push(id); },
    recordHostCall(id) { hostProxyCalls.push(id); return id; },
    reenterClose() { return reentrantSession ? (reentrantClosePromise = reentrantSession.close()) : undefined; },
  };
  const session = addon.__uniffi_backend_factory(host);
  assert.equal(session.hostFlavor, 'node');
  for (const operationId of [-1, 0.5, 2 ** 32]) {
    assert.throws(() => session.invokeSync(operationId, []));
  }
  assert.equal(session.invokeSync(0, []).kind, 'value');
  assert.equal(session.invokeSync(0, []).value, 42n);
  assert.equal((await session.invokeAsync(1, [41n])).value, 42n);
  const valueReceiverObjectReleasesBefore = session.invokeSync(39, [9999]).value;
  const valueReceiverObjectReleases = releasedCallbacks.slice();
  const valueReceiverStreamReleases = releasedStreams.slice();
  assert.equal(session.invokeSync(45, [{ amount: 9, handle: 9999 }]).value, 10);
  assert.equal(session.invokeSync(46, [{ tag: 'Ready', amount: 7, handle: 9999 }]).value, 9);
  assert.equal((await session.invokeAsync(47, [{ amount: 6, handle: 9999 }])).value, 9);
  assert.deepEqual(releasedCallbacks, valueReceiverObjectReleases);
  assert.deepEqual(releasedStreams, valueReceiverStreamReleases);
  const objectReleaseCount = async (handle) => {
    const query = addon.__uniffi_backend_factory(host);
    const count = query.invokeSync(39, [handle]).value;
    await query.close();
    return count;
  };
  const outputCancelCount = async (handle) => {
    const query = addon.__uniffi_backend_factory(host);
    const count = query.invokeSync(40, [handle]).value;
    await query.close();
    return count;
  };
  const outputReleaseCount = async (handle) => {
    const query = addon.__uniffi_backend_factory(host);
    const count = query.invokeSync(41, [handle]).value;
    await query.close();
    return count;
  };

  // A single return value can contain an optional object, a record sequence,
  // and a tagged enum payload.  The null optional must be skipped while every
  // non-null object is retained and released exactly once.
  const nestedObjectSession = addon.__uniffi_backend_factory(host);
  const nestedObject = nestedObjectSession.invokeSync(48, [0]).value;
  assert.equal(nestedObject.optionalObject.handle, 403);
  assert.deepEqual(nestedObject.objects.map(({ handle }) => handle), [401, 402]);
  assert.equal(nestedObject.variant.tag, 'Ready');
  assert.equal(nestedObject.variant.object.handle, 404);
  await nestedObjectSession.close();
  for (const handle of [401, 402, 403, 404]) {
    assert.equal(await objectReleaseCount(handle), 1, `nested object ${handle} released once`);
  }
  const nestedNullSession = addon.__uniffi_backend_factory(host);
  const nestedNull = nestedNullSession.invokeSync(48, [1]).value;
  assert.equal(nestedNull.optionalObject, null);
  assert.deepEqual(nestedNull.objects.map(({ handle }) => handle), [411, 412]);
  assert.equal(nestedNull.variant.object.handle, 414);
  await nestedNullSession.close();
  assert.equal(await objectReleaseCount(413), 0, 'null optional object is not released');
  for (const handle of [411, 412, 414]) {
    assert.equal(await objectReleaseCount(handle), 1, `nested null-object ${handle} released once`);
  }

  // Output-stream resources follow the same path fan-out and are cancelled
  // before release, including the optional and enum branches.
  const nestedOutputSession = addon.__uniffi_backend_factory(host);
  const nestedOutput = nestedOutputSession.invokeSync(49, []).value;
  assert.equal(nestedOutput.optionalOutput.handle, 500);
  assert.deepEqual(nestedOutput.outputs.map(({ handle }) => handle), [501, 502]);
  assert.equal(nestedOutput.variant.output.handle, 503);
  await nestedOutputSession.close();
  await new Promise((resolve) => setTimeout(resolve, 10));
  for (const handle of [500, 501, 502, 503]) {
    assert.equal(await outputCancelCount(handle), 1, `nested output ${handle} cancelled once`);
    assert.equal(await outputReleaseCount(handle), 1, `nested output ${handle} released once`);
  }

  // StreamStep is a first-class resource path.  The live item branch retains
  // its owned object until explicit release, while the error branch is
  // released by session close; both branches must preserve the exact tagged
  // own-key shape.
  const stepLiveSession = addon.__uniffi_backend_factory(host);
  const stepLiveOutput = (await stepLiveSession.invokeAsync(9, [])).value;
  const liveItemStep = (await stepLiveSession.invokeAsync(56, [stepLiveOutput])).value;
  assert.deepEqual(Object.keys(liveItemStep).sort(), ['kind', 'value']);
  assert.equal(liveItemStep.kind, 'item');
  assert.equal(liveItemStep.value.handle, 1001);
  stepLiveSession.releaseObject(liveItemStep.value);
  await stepLiveSession.close();
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.equal(await objectReleaseCount(1001), 1, 'live StreamItem object released exactly once');
  assert.equal(await outputCancelCount(stepLiveOutput.handle), 1, 'live step output cancelled once');
  assert.equal(await outputReleaseCount(stepLiveOutput.handle), 1, 'live step output released once');

  const stepErrorSession = addon.__uniffi_backend_factory(host);
  const stepErrorOutput = (await stepErrorSession.invokeAsync(9, [])).value;
  const errorStep = (await stepErrorSession.invokeAsync(57, [stepErrorOutput])).value;
  assert.deepEqual(Object.keys(errorStep).sort(), ['error', 'kind']);
  assert.equal(errorStep.kind, 'error');
  assert.equal(errorStep.error.handle, 1002);
  await stepErrorSession.close();
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.equal(await objectReleaseCount(1002), 1, 'StreamError object released exactly once');
  assert.equal(await outputCancelCount(stepErrorOutput.handle), 1, 'error step output cancelled once');
  assert.equal(await outputReleaseCount(stepErrorOutput.handle), 1, 'error step output released once');

  // A late output step settles only after deadline detach.  The detached
  // walker still follows StreamItem and disposes its object exactly once,
  // independently of the output receiver cleanup.
  const lateStepController = addon.__uniffi_backend_factory(host);
  lateStepController.invokeSync(31, []);
  const lateStepSession = addon.__uniffi_backend_factory(host);
  const lateStepOutput = (await lateStepSession.invokeAsync(9, [])).value;
  const lateStepResult = lateStepSession.invokeAsync(58, [lateStepOutput]);
  const lateStepClose = lateStepSession.close();
  await new Promise((resolve) => setTimeout(resolve, 80));
  await lateStepClose;
  lateStepController.invokeSync(32, []);
  const lateItemStep = (await lateStepResult).value;
  assert.deepEqual(Object.keys(lateItemStep).sort(), ['kind', 'value']);
  assert.equal(lateItemStep.value.handle, 3001);
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(await objectReleaseCount(3001), 1, 'late StreamItem object released exactly once');
  assert.equal(await outputCancelCount(lateStepOutput.handle), 1, 'late step output cancelled once');
  assert.equal(await outputReleaseCount(lateStepOutput.handle), 1, 'late step output released once');
  await lateStepController.close();

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
  const nestedInputSession = addon.__uniffi_backend_factory(host);
  const nestedInput = nestedInputSession.invokeSync(53, []).value;
  assert.equal(nestedInput.optionalInput, 600);
  assert.deepEqual(nestedInput.inputs, [601, 602]);
  assert.equal(nestedInput.variant.input, 603);
  await nestedInputSession.close();
  for (const streamId of [600, 601, 602, 603]) {
    assert.equal(
      releasedStreams.filter((id) => id === streamId).length,
      1,
      `nested input ${streamId} released once`,
    );
  }
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

  // HostAndArguments uses an independent revocable proxy for every async
  // invocation.  Two futures can be held and released together without a
  // session-level "current Host" slot; each captured proxy reaches the right
  // application call exactly once.
  const hostProxyController = addon.__uniffi_backend_factory(host);
  hostProxyController.invokeSync(36, []);
  const hostProxyA = addon.__uniffi_backend_factory(host);
  const hostProxyB = addon.__uniffi_backend_factory(host);
  const hostProxyAResult = hostProxyA.invokeAsync(33, [601]);
  const hostProxyBResult = hostProxyB.invokeAsync(33, [602]);
  hostProxyController.invokeSync(37, []);
  assert.equal((await hostProxyAResult).value, 601);
  assert.equal((await hostProxyBResult).value, 602);
  assert.deepEqual(hostProxyCalls.slice(-2).sort((left, right) => left - right), [601, 602]);
  const hostProxyATimers = timerSnapshot();
  await hostProxyA.close();
  assertOneTeardownTimer(hostProxyATimers, 'host proxy A natural close');
  const hostProxyBTimers = timerSnapshot();
  await hostProxyB.close();
  assertOneTeardownTimer(hostProxyBTimers, 'host proxy B natural close');
  // The last captured proxy is still held by the fixture, but natural detach
  // revokes its lease before this post-close call reaches the application.
  const callsBeforeNaturalLateHost = hostProxyCalls.slice();
  hostProxyController.invokeSync(38, []);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(hostProxyCalls, callsBeforeNaturalLateHost);
  await hostProxyController.close();

  // Two method wrappers on one captured Proxy retain independent target
  // references.  Drop the first TSFN, force a V8 collection, then let the
  // second wrapper call the application before the invocation closes.
  const methodGcController = addon.__uniffi_backend_factory(host);
  methodGcController.invokeSync(36, []);
  const methodGcSession = addon.__uniffi_backend_factory(host);
  const methodGcResult = methodGcSession.invokeAsync(43, [606]);
  methodGcController.invokeSync(37, []);
  await new Promise((resolve) => setImmediate(resolve));
  if (global.gc) { global.gc(); await new Promise((resolve) => setImmediate(resolve)); }
  methodGcController.invokeSync(44, []);
  assert.equal((await methodGcResult).value, 606);
  assert.equal(hostProxyCalls.at(-1), 606);
  await methodGcSession.close();
  await methodGcController.close();

  // Closing while the invocation is still held must settle naturally when
  // the first captured Host call arrives before the grace deadline.
  const naturalLateHostController = addon.__uniffi_backend_factory(host);
  naturalLateHostController.invokeSync(36, []);
  const naturalLateHostSession = addon.__uniffi_backend_factory(host);
  const naturalLateHostResult = naturalLateHostSession.invokeAsync(33, [604]);
  const naturalLateHostTimers = timerSnapshot();
  const naturalLateHostClose = naturalLateHostSession.close();
  await new Promise((resolve) => setTimeout(resolve, 10));
  naturalLateHostController.invokeSync(37, []);
  assert.equal((await naturalLateHostResult).value, 604);
  await naturalLateHostClose;
  assertOneTeardownTimer(naturalLateHostTimers, 'natural late Host close');
  await naturalLateHostController.close();

  // A proxy captured by an invocation that outlives the deadline may be
  // entered by its native future after detach; the revoked lease makes that a
  // no-op rather than an application/Host callback or an unhandled rejection.
  const deadlineHostController = addon.__uniffi_backend_factory(host);
  deadlineHostController.invokeSync(36, []);
  const deadlineHostSession = addon.__uniffi_backend_factory(host);
  const deadlineHostResult = deadlineHostSession.invokeAsync(33, [603]);
  const callsBeforeDeadlineLateHost = hostProxyCalls.slice();
  const deadlineHostClose = deadlineHostSession.close();
  await new Promise((resolve) => setTimeout(resolve, 80));
  await deadlineHostClose;
  deadlineHostController.invokeSync(37, []);
  assert.equal((await deadlineHostResult).value, 603);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(hostProxyCalls, callsBeforeDeadlineLateHost);
  deadlineHostController.invokeSync(38, []);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(hostProxyCalls, callsBeforeDeadlineLateHost);
  await deadlineHostController.close();

  // A synchronous HostAndArguments lowerer can re-enter close().  The native
  // result still crosses the original invocation boundary, while close's
  // detached cleanup releases each late resource exactly once and never
  // creates a new session lease.
  reentrantSession = addon.__uniffi_backend_factory(host);
  const reentrantObject = reentrantSession.invokeSync(34, [701]);
  assert.equal(reentrantObject.value.handle, 808);
  await reentrantClosePromise;
  assert.strictEqual(reentrantSession.close(), reentrantClosePromise);
  reentrantSession = addon.__uniffi_backend_factory(host);
  const reentrantOutput = reentrantSession.invokeSync(35, [702]);
  assert.equal(reentrantOutput.value.handle, 910);
  await reentrantClosePromise;
  await reentrantSession.close();
  await new Promise((resolve) => setTimeout(resolve, 20));

  const reentrantResourceQuery = addon.__uniffi_backend_factory(host);
  assert.equal(reentrantResourceQuery.invokeSync(39, [808]).value, 1);
  assert.equal(reentrantResourceQuery.invokeSync(40, [910]).value, 1);
  assert.equal(reentrantResourceQuery.invokeSync(41, [910]).value, 1);
  await reentrantResourceQuery.close();

  // Keep a fallible async HostAndArguments operation in the real addon so the
  // engine's custom entry is compiled through the Descriptor error path too.
  const fallibleHostController = addon.__uniffi_backend_factory(host);
  const fallibleHostSession = addon.__uniffi_backend_factory(host);
  assert.equal((await fallibleHostSession.invokeAsync(42, [605])).value, 605);
  await fallibleHostSession.close();
  fallibleHostController.invokeSync(38, []);
  await fallibleHostController.close();
  const fallibleErrorController = addon.__uniffi_backend_factory(host);
  fallibleErrorController.invokeSync(36, []);
  const fallibleErrorSession = addon.__uniffi_backend_factory(host);
  const fallibleErrorResult = fallibleErrorSession.invokeAsync(42, [608]);
  fallibleErrorController.invokeSync(37, []);
  const fallibleErrorEnvelope = await fallibleErrorResult;
  assert.equal(fallibleErrorEnvelope.kind, 'error');
  assert.equal(fallibleErrorEnvelope.error.message, 'fallible Host fixture error');
  await fallibleErrorSession.close();
  fallibleErrorController.invokeSync(38, []);
  await fallibleErrorController.close();

  const pending = session.invokeAsync(9, []);
  await session.close();
  await pending;
  const valueReceiverResourceQuery = addon.__uniffi_backend_factory(host);
  assert.equal(valueReceiverResourceQuery.invokeSync(39, [9999]).value, valueReceiverObjectReleasesBefore);
  await valueReceiverResourceQuery.close();
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

  // These three native futures stay pending across the configured 40ms
  // deadline.  Waking them afterwards must deliver the result Promise but
  // must not re-enter the revoked Host; every nested resource is cleaned by
  // the engine-owned late-result path exactly once.
  const lateHostCalls = hostProxyCalls.slice();
  const lateObjectController = addon.__uniffi_backend_factory(host);
  lateObjectController.invokeSync(31, []);
  const lateObjectSession = addon.__uniffi_backend_factory(host);
  const lateObjectResult = lateObjectSession.invokeAsync(50, []);
  let lateObjectClosed = false;
  const lateObjectClose = lateObjectSession.close().then(() => { lateObjectClosed = true; });
  await new Promise((resolve) => setTimeout(resolve, 80));
  assert.equal(lateObjectClosed, true);
  lateObjectController.invokeSync(32, []);
  const lateObjectValue = (await lateObjectResult).value;
  assert.equal(lateObjectValue.variant.tag, 'Ready');
  await new Promise((resolve) => setImmediate(resolve));
  for (const handle of [421, 422, 423, 424]) {
    assert.equal(await objectReleaseCount(handle), 1, `late nested object ${handle} released once`);
  }
  assert.deepEqual(hostProxyCalls, lateHostCalls, 'late nested object never re-enters Host');
  await lateObjectController.close();

  const nestedLateOutputController = addon.__uniffi_backend_factory(host);
  nestedLateOutputController.invokeSync(31, []);
  const nestedLateOutputSession = addon.__uniffi_backend_factory(host);
  const nestedLateOutputResult = nestedLateOutputSession.invokeAsync(51, []);
  let nestedLateOutputClosed = false;
  const nestedLateOutputClose = nestedLateOutputSession.close().then(() => { nestedLateOutputClosed = true; });
  await new Promise((resolve) => setTimeout(resolve, 80));
  assert.equal(nestedLateOutputClosed, true);
  nestedLateOutputController.invokeSync(32, []);
  const nestedLateOutputValue = (await nestedLateOutputResult).value;
  assert.equal(nestedLateOutputValue.variant.tag, 'Ready');
  await new Promise((resolve) => setTimeout(resolve, 5));
  for (const handle of [510, 511, 512, 513]) {
    assert.equal(await outputCancelCount(handle), 1, `late nested output ${handle} cancelled once`);
    assert.equal(await outputReleaseCount(handle), 1, `late nested output ${handle} released once`);
  }
  assert.deepEqual(hostProxyCalls, lateHostCalls, 'late nested output never re-enters Host');
  await nestedLateOutputController.close();

  const lateInputController = addon.__uniffi_backend_factory(host);
  lateInputController.invokeSync(31, []);
  const lateInputSession = addon.__uniffi_backend_factory(host);
  const lateInputResult = lateInputSession.invokeAsync(52, []);
  let lateInputClosed = false;
  const lateInputClose = lateInputSession.close().then(() => { lateInputClosed = true; });
  await new Promise((resolve) => setTimeout(resolve, 80));
  assert.equal(lateInputClosed, true);
  lateInputController.invokeSync(32, []);
  const lateInputValue = (await lateInputResult).value;
  assert.equal(lateInputValue.variant.tag, 'Ready');
  await new Promise((resolve) => setImmediate(resolve));
  for (const streamId of [610, 611, 612, 613]) {
    assert.equal(
      releasedStreams.filter((id) => id === streamId).length,
      1,
      `late nested input ${streamId} released once`,
    );
  }
  assert.deepEqual(hostProxyCalls, lateHostCalls, 'late nested input never re-enters Host');
  await lateInputController.close();

  // Every teardown uses one short Node timer.  A never-settling callback,
  // input pull and output cancel must detach at the same deadline; resolving
  // any of them afterwards cannot re-enter Host or duplicate release hooks.
  const deadlineCallbackSession = addon.__uniffi_backend_factory(host);
  const deadlineCallbackController = addon.__uniffi_backend_factory(host);
  deadlineCallbackController.invokeSync(28, []);
  const deadlineRetained = retained.slice();
  const deadlineReleased = releasedCallbacks.slice();
  const deadlineCallbackResult = deadlineCallbackSession.invokeAsync(27, []);
  let deadlineCallbackClosed = false;
  const deadlineCallbackClose = deadlineCallbackSession.close().then(() => { deadlineCallbackClosed = true; });
  await new Promise((resolve) => setTimeout(resolve, 80));
  assert.equal(deadlineCallbackClosed, true);
  deadlineCallbackController.invokeSync(29, []);
  await deadlineCallbackResult;
  assert.deepEqual(retained, deadlineRetained);
  assert.deepEqual(releasedCallbacks, deadlineReleased);
  await deadlineCallbackController.close();

  const deadlineInputSession = addon.__uniffi_backend_factory(host);
  const deadlineInput = deadlineInputSession.invokeAsync(5, [44]);
  let deadlineInputClosed = false;
  const deadlineInputClose = deadlineInputSession.close().then(() => { deadlineInputClosed = true; });
  const releasedBeforeDeadlineInput = releasedStreams.slice();
  await new Promise((resolve) => setTimeout(resolve, 80));
  assert.equal(deadlineInputClosed, true);
  resolvePendingPull({ kind: 'done' });
  await deadlineInput;
  assert.deepEqual(releasedStreams, releasedBeforeDeadlineInput);

  const deadlineOutputSession = addon.__uniffi_backend_factory(host);
  const deadlineOutputController = addon.__uniffi_backend_factory(host);
  deadlineOutputController.invokeSync(23, []);
  const deadlineOutput = (await deadlineOutputSession.invokeAsync(9, [])).value;
  const deadlineCancel = deadlineOutputSession.cancelOutputStream(deadlineOutput);
  let deadlineOutputClosed = false;
  const deadlineOutputClose = deadlineOutputSession.close().then(() => { deadlineOutputClosed = true; });
  await new Promise((resolve) => setTimeout(resolve, 80));
  assert.equal(deadlineOutputClosed, true);
  deadlineOutputController.invokeSync(24, []);
  await deadlineCancel;
  await deadlineOutputController.close();

  // Scoped callback proxies receive the same session-owned invoker as
  // retained proxies but never acquire a callback lease. Two proxy instances
  // in one session must therefore consume one shared invocation-ID domain.
  const scopedSession = addon.__uniffi_backend_factory(host);
  const scopedCallsBefore = callbackCalls.length;
  assert.equal((await scopedSession.invokeAsync(54, [40])).value, 7);
  assert.equal((await scopedSession.invokeAsync(54, [41])).value, 7);
  const scopedCalls = callbackCalls.slice(scopedCallsBefore);
  assert.deepEqual(scopedCalls.map((call) => call[2]), [40, 41]);
  assert.deepEqual(scopedCalls.map((call) => call[4]), [0, 1]);
  await scopedSession.close();

  // A callback proxy returned by a callback-method/typed lowerer may retain
  // its JS registration from the owning thread without receiving an
  // argument-transfer lease. The helper below exercises that exact API and
  // verifies close-vs-drop remains one logical release.
  const returnedCallbackSession = addon.__uniffi_backend_factory(host);
  assert.equal(returnedCallbackSession.invokeSync(55, [77]).value, 77);
  assert.deepEqual(retained.at(-1), [0, 77]);
  returnedCallbackSession.invokeSync(22, []);
  for (let turn = 0; turn < 10 && releasedCallbacks.filter(([typeId, id]) => typeId === 0 && id === 77).length === 0; turn++) {
    await new Promise((resolve) => setTimeout(resolve, 1));
  }
  assert.equal(releasedCallbacks.filter(([typeId, id]) => typeId === 0 && id === 77).length, 1);
  await returnedCallbackSession.close();

  const returnedCallbackRace = addon.__uniffi_backend_factory(host);
  assert.equal(returnedCallbackRace.invokeSync(55, [78]).value, 78);
  await returnedCallbackRace.close();
  // The proxy remains in the fixture until a separate owning session drops
  // it; session close has already claimed the lease, so this is still once.
  const returnedCallbackDropper = addon.__uniffi_backend_factory(host);
  returnedCallbackDropper.invokeSync(22, []);
  await returnedCallbackDropper.close();
  assert.equal(releasedCallbacks.filter(([typeId, id]) => typeId === 0 && id === 78).length, 1);

  const failedReturnedCallback = addon.__uniffi_backend_factory(host);
  failRetain = true;
  let failedReturnedEnvelope;
  try {
    failedReturnedEnvelope = failedReturnedCallback.invokeSync(55, [80]);
  } catch (_) {
    failedReturnedEnvelope = { kind: 'error' };
  }
  assert.equal(failedReturnedEnvelope.kind, 'error');
  failRetain = false;
  for (let turn = 0; turn < 10 && releasedCallbacks.filter(([typeId, id]) => typeId === 0 && id === 80).length === 0; turn++) {
    await new Promise((resolve) => setTimeout(resolve, 1));
  }
  assert.equal(releasedCallbacks.filter(([typeId, id]) => typeId === 0 && id === 80).length, 1);
  await failedReturnedCallback.close();

  const closedReturnedCallback = addon.__uniffi_backend_factory(host);
  await closedReturnedCallback.close();
  const retainedBeforeClosedReturn = retained.slice();
  let closedReturnedEnvelope;
  try {
    closedReturnedEnvelope = closedReturnedCallback.invokeSync(55, [79]);
  } catch (_) {
    closedReturnedEnvelope = { kind: 'error' };
  }
  assert.equal(closedReturnedEnvelope.kind, 'error');
  assert.deepEqual(retained, retainedBeforeClosedReturn);

  let droppedSession = addon.__uniffi_backend_factory(host);
  const droppedOutput = droppedSession.invokeAsync(9, []);
  const droppedInput = droppedSession.invokeAsync(5, [33]);
  droppedSession = null;
  if (global.gc) global.gc();
  await droppedOutput;
  await droppedInput;
  if (global.gc) global.gc();
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(
    releasedStreams.filter((id) => [11, 22, 44, 33].includes(id)),
    [11, 22, 44, 44, 33],
  );
  for (const streamId of [600, 601, 602, 603, 610, 611, 612, 613]) {
    assert.equal(
      releasedStreams.filter((id) => id === streamId).length,
      1,
      `nested stream ${streamId} released once`,
    );
  }
})().catch(error => { console.error(error); process.exitCode = 1; })
  .finally(() => {
    global.setTimeout = originalSetTimeout;
    global.clearTimeout = originalClearTimeout;
  });
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
