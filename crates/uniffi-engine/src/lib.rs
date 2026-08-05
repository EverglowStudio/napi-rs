//! Programmatic UniFFI frontend for napi-rs.
//!
//! The frontend accepts an engine-owned family plan and a structured Rust call
//! plan. It performs no component discovery and has no process or filesystem
//! inputs. Raw operation callbacks stay private; the generated module
//! registers one backend factory containing a dense function table.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ffi::CString;
use std::fmt;
use std::ptr;

use napi::bindgen_prelude::{BigInt, ToNapiValue};
use napi::{sys, Result as NapiResult};
use napi_derive_backend::{NapiFn, NapiFnArg, NapiFnArgKind, NapiFnBuilder, TryToTokens};
use napi_family_core::StreamDirection;
use napi_family_core::{
  AsyncKind, BigIntWords, CallbackReentrancy, CallbackRetention, CallbackThreading,
  CallbackUseSite, FamilyOperation, FamilyOperationTarget, FamilyPlan, FamilyPlanError, HostFlavor,
  OperationKind, ReceiverBinding, ResourceKind, ResourceOwnership, ValuePath, ValuePathSegment,
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Path, Type};

pub use napi_family_core;
pub use napi_family_core::{ClosePolicy, DeadlineAction};
mod session;
pub use session::{
  create_backend_session, take_session_callback_transfers, SessionCallbackArgument,
  SessionCallbackLease, SessionCallbackReentrancy, SessionCallbackRetention,
  SessionCallbackThreading, SessionCallbackTransfers, SessionNativeCall,
  SessionOperationDescriptor, SessionOperationDispatch, SessionReceiver, SessionResourceCallbacks,
  SessionResourceReceiver, SessionStreamArgument, SessionStreamDirection, SessionValuePathSegment,
};

pub const BACKEND_FACTORY_EXPORT: &str = "__uniffi_backend_factory";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorDomain {
  Declared,
  Validation,
  Backend,
  Panic,
}

impl ErrorDomain {
  const fn as_str(self) -> &'static str {
    match self {
      Self::Declared => "declared",
      Self::Validation => "validation",
      Self::Backend => "backend",
      Self::Panic => "panic",
    }
  }
}

/// Owned canonical data for a private error descriptor.
#[derive(Clone, Debug, PartialEq)]
pub enum ErrorData {
  Null,
  Boolean(bool),
  Number(f64),
  BigInt(BigIntWords),
  String(String),
  Bytes(Vec<u8>),
  Sequence(Vec<ErrorData>),
  Record(BTreeMap<String, ErrorData>),
}

impl ToNapiValue for ErrorData {
  unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> NapiResult<sys::napi_value> {
    match value {
      Self::Null => {
        let mut raw = ptr::null_mut();
        napi::check_status!(unsafe { sys::napi_get_null(env, &mut raw) })?;
        Ok(raw)
      }
      Self::Boolean(value) => unsafe { bool::to_napi_value(env, value) },
      Self::Number(value) => unsafe { f64::to_napi_value(env, value) },
      Self::BigInt(value) => unsafe {
        BigInt::to_napi_value(
          env,
          BigInt {
            sign_bit: value.negative,
            words: value.words,
          },
        )
      },
      Self::String(value) => unsafe { String::to_napi_value(env, value) },
      Self::Bytes(value) => create_uint8_array(env, &value),
      Self::Sequence(value) => unsafe { Vec::<ErrorData>::to_napi_value(env, value) },
      Self::Record(value) => {
        let object = create_object(env)?;
        for (name, value) in value {
          set_named(env, object, &name, value)?;
        }
        Ok(object)
      }
    }
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BridgeErrorDescriptor {
  pub domain: ErrorDomain,
  pub error_name: String,
  pub variant: Option<String>,
  pub data: ErrorData,
  pub message: String,
  pub native_stack: Option<String>,
}

impl BridgeErrorDescriptor {
  pub fn validation(message: impl Into<String>) -> Self {
    let message = message.into();
    Self {
      domain: ErrorDomain::Validation,
      error_name: "ValidationError".to_owned(),
      variant: None,
      data: ErrorData::String(message.clone()),
      message,
      native_stack: None,
    }
  }

  pub fn backend(message: impl Into<String>) -> Self {
    let message = message.into();
    Self {
      domain: ErrorDomain::Backend,
      error_name: "BackendError".to_owned(),
      variant: None,
      data: ErrorData::String(message.clone()),
      message,
      native_stack: None,
    }
  }
}

impl ToNapiValue for BridgeErrorDescriptor {
  unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> NapiResult<sys::napi_value> {
    let object = create_object(env)?;
    set_named(env, object, "domain", value.domain.as_str())?;
    set_named(env, object, "errorName", value.error_name)?;
    set_named(env, object, "variant", value.variant)?;
    set_named(env, object, "data", value.data)?;
    set_named(env, object, "message", value.message)?;
    if let Some(native_stack) = value.native_stack {
      set_named(env, object, "nativeStack", native_stack)?;
    }
    Ok(object)
  }
}

/// Every private operation resolves to this stable value/error envelope.
#[derive(Clone, Debug, PartialEq)]
pub enum NapiCallResult<T> {
  Value(T),
  Error(BridgeErrorDescriptor),
}

impl<T: ToNapiValue> ToNapiValue for NapiCallResult<T> {
  unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> NapiResult<sys::napi_value> {
    let object = create_object(env)?;
    match value {
      Self::Value(value) => {
        set_named(env, object, "kind", "value")?;
        set_named(env, object, "value", value)?;
      }
      Self::Error(error) => {
        set_named(env, object, "kind", "error")?;
        set_named(env, object, "error", error)?;
      }
    }
    Ok(object)
  }
}

unsafe fn create_object(env: sys::napi_env) -> NapiResult<sys::napi_value> {
  let mut object = ptr::null_mut();
  napi::check_status!(unsafe { sys::napi_create_object(env, &mut object) })?;
  Ok(object)
}

unsafe fn create_uint8_array(env: sys::napi_env, value: &[u8]) -> NapiResult<sys::napi_value> {
  let mut data = ptr::null_mut();
  let mut array_buffer = ptr::null_mut();
  napi::check_status!(unsafe {
    sys::napi_create_arraybuffer(env, value.len(), &mut data, &mut array_buffer)
  })?;
  if !value.is_empty() {
    unsafe {
      ptr::copy_nonoverlapping(value.as_ptr(), data.cast::<u8>(), value.len());
    }
  }
  let mut typed_array = ptr::null_mut();
  napi::check_status!(unsafe {
    sys::napi_create_typedarray(
      env,
      sys::TypedarrayType::uint8_array,
      value.len(),
      array_buffer,
      0,
      &mut typed_array,
    )
  })?;
  Ok(typed_array)
}

unsafe fn set_named<T: ToNapiValue>(
  env: sys::napi_env,
  object: sys::napi_value,
  name: &str,
  value: T,
) -> NapiResult<()> {
  let name = CString::new(name)?;
  let value = unsafe { T::to_napi_value(env, value)? };
  napi::check_status!(unsafe { sys::napi_set_named_property(env, object, name.as_ptr(), value) })
}

/// Structured lowering for one Rust argument.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArgumentBinding {
  Direct {
    carrier_type: Type,
  },
  I64BigInt,
  U64BigInt,
  LowerWith {
    carrier_type: Type,
    lower: Path,
  },
  /// A structured lowerer that needs the session Host to construct nested
  /// callback/stream proxies. The path itself is carried by the family plan;
  /// this variant only supplies the Rust-side carrier and lowering hook.
  LowerWithHost {
    carrier_type: Type,
    lower: Path,
  },
  ObjectLease {
    carrier_type: Type,
    lower: Path,
    ownership: ResourceOwnership,
  },
  OutputStreamLease {
    carrier_type: Type,
    lower: Path,
    ownership: ResourceOwnership,
  },
  CallbackProxy {
    rust_type: Type,
    build: Path,
  },
  InputStreamProxy {
    rust_type: Type,
    build: Path,
  },
}

impl ArgumentBinding {
  fn carrier_type(&self) -> Type {
    match self {
      Self::Direct { carrier_type }
      | Self::LowerWith { carrier_type, .. }
      | Self::LowerWithHost { carrier_type, .. }
      | Self::ObjectLease { carrier_type, .. }
      | Self::OutputStreamLease { carrier_type, .. } => carrier_type.clone(),
      Self::I64BigInt | Self::U64BigInt => syn::parse_quote!(napi::bindgen_prelude::BigInt),
      Self::CallbackProxy { .. } | Self::InputStreamProxy { .. } => syn::parse_quote!(u32),
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustArgumentPlan {
  pub name: Ident,
  pub binding: ArgumentBinding,
}

/// Structured lifting for one Rust return value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReturnBinding {
  Unit,
  Direct { carrier_type: Type },
  I64BigInt,
  U64BigInt,
  LiftWith { carrier_type: Type, lift: Path },
  ObjectLease { carrier_type: Type, lift: Path },
  CallbackLease { carrier_type: Type, lift: Path },
  OutputStreamLease { carrier_type: Type, lift: Path },
}

impl ReturnBinding {
  fn carrier_type(&self) -> Type {
    match self {
      Self::Unit => syn::parse_quote!(()),
      Self::Direct { carrier_type }
      | Self::LiftWith { carrier_type, .. }
      | Self::ObjectLease { carrier_type, .. }
      | Self::CallbackLease { carrier_type, .. }
      | Self::OutputStreamLease { carrier_type, .. } => carrier_type.clone(),
      Self::I64BigInt | Self::U64BigInt => syn::parse_quote!(napi::bindgen_prelude::BigInt),
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorBinding {
  Infallible,
  Descriptor { map: Path },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RustOperationTarget {
  Native { call: Path },
  CallbackHost,
  InputStreamHostPull,
  InputStreamHostCancel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustReceiverPlan {
  pub name: Ident,
  pub binding: ArgumentBinding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustOperationPlan {
  pub operation_id: u32,
  pub target: RustOperationTarget,
  pub receiver: Option<RustReceiverPlan>,
  pub arguments: Vec<RustArgumentPlan>,
  pub return_binding: ReturnBinding,
  pub error_binding: ErrorBinding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustResourceHook {
  pub call: Path,
  pub carrier_type: Type,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RustResourceHooks {
  pub release_object: Option<RustResourceHook>,
  pub cancel_output_stream: Option<RustResourceHook>,
  pub release_output_stream: Option<RustResourceHook>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustBridgePlan {
  operations: Vec<RustOperationPlan>,
  resource_hooks: RustResourceHooks,
}

impl RustBridgePlan {
  pub fn build(
    family: &FamilyPlan,
    operations: Vec<RustOperationPlan>,
  ) -> Result<Self, EngineError> {
    Self::build_with_resource_hooks(family, operations, RustResourceHooks::default())
  }

  pub fn build_with_resource_hooks(
    family: &FamilyPlan,
    operations: Vec<RustOperationPlan>,
    resource_hooks: RustResourceHooks,
  ) -> Result<Self, EngineError> {
    let mut by_id = BTreeMap::new();
    for operation in operations {
      let id = operation.operation_id;
      if by_id.insert(id, operation).is_some() {
        return Err(EngineError::DuplicateRustOperation { id });
      }
    }
    if by_id.len() != family.operations().len() {
      return Err(EngineError::RustOperationCount {
        expected: family.operations().len(),
        actual: by_id.len(),
      });
    }

    let mut validated = Vec::with_capacity(by_id.len());
    for family_operation in family.operations() {
      let expected = family_operation.id;
      let Some(operation) = by_id.remove(&expected) else {
        return Err(EngineError::MissingRustOperation { id: expected });
      };
      validate_operation_target(family_operation, &operation)?;
      let is_native = matches!(operation.target, RustOperationTarget::Native { .. });
      if is_native {
        if operation.arguments.len() != family_operation.argument_count {
          return Err(EngineError::ArgumentCount {
            operation_id: operation.operation_id,
            expected: family_operation.argument_count,
            actual: operation.arguments.len(),
          });
        }
        validate_receiver(family_operation, &operation)?;
        validate_result_resource(family_operation, &operation)?;
      } else if operation.receiver.is_some() || !operation.arguments.is_empty() {
        return Err(EngineError::HostOperationHasRustBindings {
          operation_id: operation.operation_id,
        });
      }
      let mut argument_names = BTreeSet::new();
      if let Some(receiver) = &operation.receiver {
        argument_names.insert(receiver.name.to_string());
      }
      for argument in &operation.arguments {
        let name = argument.name.to_string();
        if !argument_names.insert(name.clone()) {
          return Err(EngineError::DuplicateRustArgument {
            operation_id: operation.operation_id,
            name,
          });
        }
      }
      if is_native {
        validate_structured_bindings(family_operation, &operation)?;
        match (family_operation.fallible, &operation.error_binding) {
          (false, ErrorBinding::Infallible) | (true, ErrorBinding::Descriptor { .. }) => {}
          (true, ErrorBinding::Infallible) => {
            return Err(EngineError::MissingErrorDescriptor {
              operation_id: operation.operation_id,
            });
          }
          (false, ErrorBinding::Descriptor { .. }) => {
            return Err(EngineError::UnexpectedErrorDescriptor {
              operation_id: operation.operation_id,
            });
          }
        }
      } else if !matches!(operation.return_binding, ReturnBinding::Unit)
        || !matches!(operation.error_binding, ErrorBinding::Infallible)
      {
        return Err(EngineError::HostOperationHasRustBindings {
          operation_id: operation.operation_id,
        });
      } else if !family_operation.callbacks.is_empty() || !family_operation.streams.is_empty() {
        return Err(EngineError::HostOperationHasStructuredUseSites {
          operation_id: operation.operation_id,
        });
      }
      validated.push(operation);
    }

    validate_resource_hooks(family, &resource_hooks)?;
    Ok(Self {
      operations: validated,
      resource_hooks,
    })
  }

  pub fn operations(&self) -> &[RustOperationPlan] {
    &self.operations
  }

  pub fn resource_hooks(&self) -> &RustResourceHooks {
    &self.resource_hooks
  }
}

fn validate_resource_hooks(
  family: &FamilyPlan,
  hooks: &RustResourceHooks,
) -> Result<(), EngineError> {
  let needs_object = family.operations().iter().any(|operation| {
    matches!(
      operation.receiver,
      Some(ReceiverBinding::Resource(resource)) if resource.kind == ResourceKind::Object
    ) || operation.result.map(|resource| resource.kind) == Some(ResourceKind::Object)
  });
  let needs_output = family.operations().iter().any(|operation| {
    matches!(
      operation.receiver,
      Some(ReceiverBinding::Resource(resource)) if resource.kind == ResourceKind::OutputStream
    ) || operation.result.map(|resource| resource.kind) == Some(ResourceKind::OutputStream)
      || operation
        .streams
        .iter()
        .any(|stream| stream.direction == StreamDirection::Output)
  });
  if needs_object && hooks.release_object.is_none() {
    return Err(EngineError::MissingResourceHook {
      role: "object release",
    });
  }
  if needs_output && hooks.cancel_output_stream.is_none() {
    return Err(EngineError::MissingResourceHook {
      role: "output-stream cancel",
    });
  }
  if needs_output && hooks.release_output_stream.is_none() {
    return Err(EngineError::MissingResourceHook {
      role: "output-stream release",
    });
  }
  Ok(())
}

fn validate_structured_bindings(
  family_operation: &FamilyOperation,
  operation: &RustOperationPlan,
) -> Result<(), EngineError> {
  for (index, argument) in operation.arguments.iter().enumerate() {
    let callback_paths = family_operation
      .callbacks
      .iter()
      .filter(|use_site| {
        matches!(
          use_site.path.segments().first(),
          Some(ValuePathSegment::Argument(argument_index)) if *argument_index as usize == index
        )
      })
      .collect::<Vec<_>>();
    let direct_callback = callback_paths
      .iter()
      .any(|use_site| matches!(use_site.path.segments(), [ValuePathSegment::Argument(_)]));
    let nested_callback = callback_paths
      .iter()
      .any(|use_site| use_site.path.segments().len() > 1);
    let callback_proxy = matches!(argument.binding, ArgumentBinding::CallbackProxy { .. });
    let lower_with_host = matches!(argument.binding, ArgumentBinding::LowerWithHost { .. });
    if direct_callback != callback_proxy {
      return Err(EngineError::InvalidStructuredBinding {
        operation_id: operation.operation_id,
        argument: index,
        role: "callback",
      });
    }
    let stream_paths = family_operation
      .streams
      .iter()
      .filter(|use_site| {
        use_site.direction == StreamDirection::Input
          && matches!(
            use_site.path.segments().first(),
            Some(ValuePathSegment::Argument(argument_index)) if *argument_index as usize == index
          )
      })
      .collect::<Vec<_>>();
    let direct_stream = stream_paths
      .iter()
      .any(|use_site| matches!(use_site.path.segments(), [ValuePathSegment::Argument(_)]));
    let nested_stream = stream_paths
      .iter()
      .any(|use_site| use_site.path.segments().len() > 1);
    let input_stream_proxy = matches!(argument.binding, ArgumentBinding::InputStreamProxy { .. });
    if direct_stream != input_stream_proxy {
      return Err(EngineError::InvalidStructuredBinding {
        operation_id: operation.operation_id,
        argument: index,
        role: "input stream",
      });
    }
    let nested_structured = nested_callback || nested_stream;
    if nested_structured != lower_with_host {
      return Err(EngineError::InvalidStructuredBinding {
        operation_id: operation.operation_id,
        argument: index,
        role: if nested_callback {
          "callback"
        } else if nested_stream {
          "input stream"
        } else {
          "callback or input stream"
        },
      });
    }
  }

  let return_callbacks = family_operation
    .callbacks
    .iter()
    .filter(|use_site| {
      matches!(
        use_site.path.segments().first(),
        Some(ValuePathSegment::Return)
      )
    })
    .collect::<Vec<_>>();
  let direct_return_callback = return_callbacks
    .iter()
    .any(|use_site| matches!(use_site.path.segments(), [ValuePathSegment::Return]));
  let callback_lease = matches!(
    operation.return_binding,
    ReturnBinding::CallbackLease { .. }
  );
  if direct_return_callback != callback_lease {
    return Err(EngineError::InvalidReturnBinding {
      operation_id: operation.operation_id,
      expected: "a direct callback lease matching the canonical return use-site",
    });
  }
  Ok(())
}

fn validate_operation_target(
  family_operation: &FamilyOperation,
  operation: &RustOperationPlan,
) -> Result<(), EngineError> {
  let valid = match (family_operation.target, &operation.target) {
    (FamilyOperationTarget::Native, RustOperationTarget::Native { .. }) => true,
    (FamilyOperationTarget::CallbackHost { .. }, RustOperationTarget::CallbackHost) => true,
    (FamilyOperationTarget::InputStreamHostPull, RustOperationTarget::InputStreamHostPull) => true,
    (FamilyOperationTarget::InputStreamHostCancel, RustOperationTarget::InputStreamHostCancel) => {
      true
    }
    _ => false,
  };
  if valid {
    Ok(())
  } else {
    Err(EngineError::InvalidOperationTarget {
      operation_id: operation.operation_id,
      kind: family_operation.kind,
    })
  }
}

fn validate_receiver(
  family_operation: &FamilyOperation,
  operation: &RustOperationPlan,
) -> Result<(), EngineError> {
  match (&family_operation.receiver, &operation.receiver) {
    (None, None) => Ok(()),
    (Some(ReceiverBinding::Value), Some(actual)) => {
      let valid = matches!(
        &actual.binding,
        ArgumentBinding::Direct { .. }
          | ArgumentBinding::I64BigInt
          | ArgumentBinding::U64BigInt
          | ArgumentBinding::LowerWith { .. }
          | ArgumentBinding::LowerWithHost { .. }
      );
      if valid {
        Ok(())
      } else {
        Err(EngineError::InvalidValueReceiver {
          operation_id: operation.operation_id,
        })
      }
    }
    (Some(ReceiverBinding::Resource(expected)), Some(actual)) => {
      let valid = match (expected.kind, &actual.binding) {
        (ResourceKind::Object, ArgumentBinding::ObjectLease { ownership, .. })
        | (ResourceKind::OutputStream, ArgumentBinding::OutputStreamLease { ownership, .. }) => {
          *ownership == expected.ownership
        }
        (ResourceKind::InputStream, ArgumentBinding::InputStreamProxy { .. }) => true,
        _ => false,
      };
      if valid {
        Ok(())
      } else {
        Err(EngineError::InvalidResourceReceiver {
          operation_id: operation.operation_id,
        })
      }
    }
    (Some(_), None) => Err(EngineError::MissingReceiver {
      operation_id: operation.operation_id,
    }),
    (None, Some(_)) => Err(EngineError::UnexpectedReceiver {
      operation_id: operation.operation_id,
    }),
  }
}

fn validate_result_resource(
  family_operation: &FamilyOperation,
  operation: &RustOperationPlan,
) -> Result<(), EngineError> {
  let valid = match family_operation.result.map(|resource| resource.kind) {
    None => !matches!(
      operation.return_binding,
      ReturnBinding::ObjectLease { .. } | ReturnBinding::OutputStreamLease { .. }
    ),
    Some(ResourceKind::Object) => {
      matches!(operation.return_binding, ReturnBinding::ObjectLease { .. })
    }
    Some(ResourceKind::InputStream) => false,
    Some(ResourceKind::OutputStream) => matches!(
      operation.return_binding,
      ReturnBinding::OutputStreamLease { .. }
    ),
  };
  if valid {
    Ok(())
  } else {
    Err(EngineError::InvalidResourceResult {
      operation_id: operation.operation_id,
    })
  }
}

#[derive(Debug)]
pub struct GeneratedNapiModule {
  family: FamilyPlan,
  source: TokenStream,
  raw_operation_names: Vec<String>,
}

impl GeneratedNapiModule {
  pub fn family(&self) -> &FamilyPlan {
    &self.family
  }

  pub fn source(&self) -> &TokenStream {
    &self.source
  }

  pub fn raw_operation_names(&self) -> &[String] {
    &self.raw_operation_names
  }

  pub fn public_exports(&self) -> impl Iterator<Item = &'static str> {
    std::iter::once(BACKEND_FACTORY_EXPORT)
  }
}

pub fn generate_napi_module(
  family: &FamilyPlan,
  rust: &RustBridgePlan,
) -> Result<GeneratedNapiModule, EngineError> {
  if family.operations().len() != rust.operations().len() {
    return Err(EngineError::RustOperationCount {
      expected: family.operations().len(),
      actual: rust.operations().len(),
    });
  }

  let mut source = TokenStream::new();
  let mut callbacks = Vec::with_capacity(rust.operations().len());
  let mut raw_operation_names = Vec::with_capacity(rust.operations().len());

  for (family_operation, operation) in family.operations().iter().zip(rust.operations()) {
    if family_operation.id != operation.operation_id {
      return Err(EngineError::OperationOrder {
        expected: family_operation.id,
        actual: operation.operation_id,
      });
    }
    if family_operation.target == FamilyOperationTarget::Native {
      let generated = generate_operation(operation, family_operation)?;
      source.extend(generated.body);
      generated
        .function
        .try_to_tokens(&mut source)
        .map_err(|error| EngineError::BackendCodegen(format!("{error:?}")))?;
      callbacks.push(Some(generated.callback_factory));
      raw_operation_names.push(generated.function_name.to_string());
    } else {
      callbacks.push(None);
    }
  }

  let resource_callbacks = generate_resource_callbacks(rust.resource_hooks(), &mut source)?;
  let factory = generate_factory(
    family.flavor(),
    family.close_policy(),
    family,
    rust.operations(),
    &callbacks,
    &resource_callbacks,
  )?;
  source.extend(factory.body);
  factory
    .function
    .try_to_tokens(&mut source)
    .map_err(|error| EngineError::BackendCodegen(format!("{error:?}")))?;

  Ok(GeneratedNapiModule {
    family: family.clone(),
    source,
    raw_operation_names,
  })
}

struct GeneratedOperation {
  function_name: Ident,
  callback_factory: Ident,
  body: TokenStream,
  function: NapiFn,
}

fn operation_requires_host(operation: &RustOperationPlan, family: &FamilyOperation) -> bool {
  let has_argument_callbacks = family.callbacks.iter().any(|use_site| {
    matches!(
      use_site.path.segments().first(),
      Some(ValuePathSegment::Argument(_))
    )
  });
  has_argument_callbacks
    || family
      .streams
      .iter()
      .any(|use_site| use_site.direction == StreamDirection::Input)
    || operation.arguments.iter().any(|argument| {
      matches!(
        argument.binding,
        ArgumentBinding::CallbackProxy { .. }
          | ArgumentBinding::InputStreamProxy { .. }
          | ArgumentBinding::LowerWithHost { .. }
      )
    })
    || operation
      .receiver
      .as_ref()
      .is_some_and(|receiver| matches!(&receiver.binding, ArgumentBinding::LowerWithHost { .. }))
}

fn value_receiver_requires_sync_lower(
  operation: &RustOperationPlan,
  family: &FamilyOperation,
) -> bool {
  if !matches!(family.receiver, Some(ReceiverBinding::Value)) {
    return false;
  }
  matches!(
    operation
      .receiver
      .as_ref()
      .map(|receiver| &receiver.binding),
    Some(
      ArgumentBinding::I64BigInt
        | ArgumentBinding::U64BigInt
        | ArgumentBinding::LowerWith { .. }
        | ArgumentBinding::LowerWithHost { .. }
    )
  )
}

fn generate_operation(
  operation: &RustOperationPlan,
  family: &FamilyOperation,
) -> Result<GeneratedOperation, EngineError> {
  let async_kind = family.async_kind;
  let id = operation.operation_id;
  let function_name = format_ident!("__uniffi_raw_operation_{id}");
  let callback_factory = format_ident!("_napi_rs_internal_register___uniffi_raw_operation_{id}");
  let return_carrier = operation.return_binding.carrier_type();
  // Callback use-sites rooted at the native return are tracked by the
  // session after settlement; they do not require a Host argument or a
  // callback-transfer table in the generated async native function.  Only
  // argument-rooted callbacks are lowered inside the native call itself.
  let has_argument_callbacks = family.callbacks.iter().any(|use_site| {
    matches!(
      use_site.path.segments().first(),
      Some(ValuePathSegment::Argument(_))
    )
  });
  let callback_transfer = has_argument_callbacks;
  let requires_host = operation_requires_host(operation, family);
  // A Host proxy is a per-invocation JS object and is therefore not `Send`.
  // For async HostAndArguments operations, lower the Host synchronously in the
  // N-API entry point and only move the resulting native proxy/carriers into
  // the worker future.  Keeping this special case in the engine frontend
  // avoids teaching the generic backend about this engine-owned protocol.
  // Any value receiver conversion that touches an N-API carrier must happen
  // before an async future is spawned.  This keeps raw N-API values out of
  // the worker future even when no Host proxy is otherwise required.
  let manual_async_entry = async_kind == AsyncKind::Async
    && (requires_host || value_receiver_requires_sync_lower(operation, family));
  let mut function_builder = NapiFnBuilder::new(function_name.clone(), function_name.to_string());
  if requires_host {
    function_builder = function_builder.argument(NapiFnArg {
      kind: NapiFnArgKind::PatType(Box::new(
        syn::parse_quote!(__uniffi_host: napi::bindgen_prelude::Object<'static>),
      )),
      ts_arg_type: None,
    });
  }
  if manual_async_entry {
    // `Env` is injected by the backend and is not a JavaScript argument.  It
    // lets the synchronous entry create a Promise before its Host object is
    // dropped, while no raw N-API value is captured by the worker future.
    function_builder = function_builder.argument(NapiFnArg {
      kind: NapiFnArgKind::PatType(Box::new(syn::parse_quote!(__uniffi_env: &napi::Env))),
      ts_arg_type: None,
    });
  }
  if callback_transfer {
    for name in ["__uniffi_session_generation", "__uniffi_callback_transfer"] {
      function_builder = function_builder.argument(NapiFnArg {
        kind: NapiFnArgKind::PatType(Box::new(syn::parse_quote!(#name: u32))),
        ts_arg_type: None,
      });
    }
  }
  function_builder = if manual_async_entry {
    function_builder
      .result_return_type(syn::parse_quote!(napi::bindgen_prelude::sys::napi_value))
      .asynchronous(false)
  } else {
    function_builder
      .return_type(syn::parse_quote!(napi_uniffi_engine::NapiCallResult<#return_carrier>))
      .asynchronous(async_kind == AsyncKind::Async)
  };
  function_builder = function_builder
    .strict(true)
    .skip_typescript(true)
    .private(true)
    .register_name(format_ident!("__napi_register_uniffi_raw_operation_{id}"));

  let mut argument_names = Vec::with_capacity(operation.arguments.len());
  let mut lowerings = Vec::new();
  let lower_error = |error: TokenStream| {
    if manual_async_entry {
      quote! {
        {
          let __uniffi_error_promise = napi::bindgen_prelude::PromiseRaw::<
            napi_uniffi_engine::NapiCallResult<#return_carrier>
          >::resolve(
            __uniffi_env,
            napi_uniffi_engine::NapiCallResult::Error(#error),
          )?;
          return Ok(napi::bindgen_prelude::JsValue::raw(&__uniffi_error_promise));
        }
      }
    } else {
      quote! { { return napi_uniffi_engine::NapiCallResult::Error(#error); } }
    }
  };
  if let Some(receiver) = &operation.receiver {
    let name = &receiver.name;
    let carrier_type = receiver.binding.carrier_type();
    let pat_type: syn::PatType = syn::parse_quote!(#name: #carrier_type);
    function_builder = function_builder.argument(NapiFnArg {
      kind: NapiFnArgKind::PatType(Box::new(pat_type)),
      ts_arg_type: None,
    });
    argument_names.push(name.clone());
    match &receiver.binding {
      ArgumentBinding::Direct { .. } => {}
      ArgumentBinding::I64BigInt => {
        let error = lower_error(quote!(
          napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string())
        ));
        lowerings.push(quote! {
          let (__uniffi_value, __uniffi_lossless) = #name.get_i64();
          let #name = match napi_uniffi_engine::napi_family_core::require_lossless_i64(
            __uniffi_value,
            __uniffi_lossless,
          ) {
            Ok(value) => value,
            Err(error) => #error,
          };
        });
      }
      ArgumentBinding::U64BigInt => {
        let error = lower_error(quote!(
          napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string())
        ));
        lowerings.push(quote! {
          let (__uniffi_negative, __uniffi_value, __uniffi_lossless) = #name.get_u64();
          let #name = match napi_uniffi_engine::napi_family_core::require_lossless_u64(
            __uniffi_negative,
            __uniffi_value,
            __uniffi_lossless,
          ) {
            Ok(value) => value,
            Err(error) => #error,
          };
        });
      }
      ArgumentBinding::LowerWith { lower, .. } => {
        let error = lower_error(quote!(error));
        lowerings.push(quote! {
          let #name = match #lower(#name) {
            Ok(value) => value,
            Err(error) => #error,
          };
        });
      }
      ArgumentBinding::LowerWithHost { lower, .. } => {
        let error = lower_error(quote!(error));
        lowerings.push(quote! {
          let #name = match #lower(&__uniffi_host, #name, &__uniffi_callback_transfers) {
            Ok(value) => value,
            Err(error) => #error,
          };
        });
      }
      ArgumentBinding::ObjectLease { lower, .. }
      | ArgumentBinding::OutputStreamLease { lower, .. } => {
        let error = lower_error(quote!(error));
        lowerings.push(quote! {
          let #name = match #lower(#name) {
            Ok(value) => value,
            Err(error) => #error,
          };
        });
      }
      ArgumentBinding::CallbackProxy { .. } | ArgumentBinding::InputStreamProxy { .. } => {
        return Err(EngineError::InvalidResourceReceiver {
          operation_id: operation.operation_id,
        });
      }
    }
  }
  for (argument_index, argument) in operation.arguments.iter().enumerate() {
    let name = &argument.name;
    let carrier_type = argument.binding.carrier_type();
    let pat_type: syn::PatType = syn::parse_quote!(#name: #carrier_type);
    function_builder = function_builder.argument(NapiFnArg {
      kind: NapiFnArgKind::PatType(Box::new(pat_type)),
      ts_arg_type: None,
    });
    argument_names.push(name.clone());
    match &argument.binding {
      ArgumentBinding::Direct { .. } => {}
      ArgumentBinding::I64BigInt => {
        let error = lower_error(quote!(
          napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string())
        ));
        lowerings.push(quote! {
        let (__uniffi_value, __uniffi_lossless) = #name.get_i64();
        let #name = match napi_uniffi_engine::napi_family_core::require_lossless_i64(
          __uniffi_value,
          __uniffi_lossless,
        ) {
          Ok(value) => value,
          Err(error) => #error,
        };
        });
      }
      ArgumentBinding::U64BigInt => {
        let error = lower_error(quote!(
          napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string())
        ));
        lowerings.push(quote! {
        let (__uniffi_negative, __uniffi_value, __uniffi_lossless) = #name.get_u64();
        let #name = match napi_uniffi_engine::napi_family_core::require_lossless_u64(
          __uniffi_negative,
          __uniffi_value,
          __uniffi_lossless,
        ) {
          Ok(value) => value,
          Err(error) => #error,
        };
        });
      }
      ArgumentBinding::LowerWith { lower, .. }
      | ArgumentBinding::ObjectLease { lower, .. }
      | ArgumentBinding::OutputStreamLease { lower, .. } => {
        let error = lower_error(quote!(error));
        lowerings.push(quote! {
        let #name = match #lower(#name) {
          Ok(value) => value,
          Err(error) => #error,
        };
        });
      }
      ArgumentBinding::LowerWithHost { lower, .. } => {
        let error = lower_error(quote!(error));
        lowerings.push(quote! {
        let #name = match #lower(&__uniffi_host, #name, &__uniffi_callback_transfers) {
          Ok(value) => value,
          Err(error) => #error,
        };
        });
      }
      ArgumentBinding::CallbackProxy { build, .. } => {
        let callback = family
          .callbacks
          .iter()
          .find(|use_site| {
            matches!(
              use_site.path.segments(),
              [ValuePathSegment::Argument(index)] if *index as usize == argument_index
            )
          })
          .ok_or(EngineError::MissingStructuredUseSite {
            operation_id: operation.operation_id,
            argument: argument_index,
            role: "callback",
          })?;
        let callback_type_id = callback.callback_type_id;
        let contract = callback_contract_tokens(callback);
        let callback_index = family
          .callbacks
          .iter()
          .position(|candidate| std::ptr::eq(candidate, callback))
          .expect("callback use-site belongs to family operation");
        let lease_error = lower_error(quote!(
          napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string())
        ));
        let build_error = lower_error(quote!(error));
        lowerings.push(quote! {
          let __uniffi_callback_lease = match __uniffi_callback_transfers.lease(
            #callback_index,
            0,
          ) {
            Ok(value) => value,
            Err(error) => #lease_error,
          };
          let #name = match #build(
            &__uniffi_host,
            #callback_type_id,
            #name,
            #contract,
            __uniffi_callback_lease,
          ) {
            Ok(value) => value,
            Err(error) => #build_error,
          };
        });
      }
      ArgumentBinding::InputStreamProxy { build, .. } => {
        let found = family.streams.iter().any(|use_site| {
          use_site.direction == StreamDirection::Input
            && matches!(
              use_site.path.segments(),
              [ValuePathSegment::Argument(index)] if *index as usize == argument_index
            )
        });
        if !found {
          return Err(EngineError::MissingStructuredUseSite {
            operation_id: operation.operation_id,
            argument: argument_index,
            role: "input stream",
          });
        }
        let error = lower_error(quote!(error));
        lowerings.push(quote! {
          let #name = match #build(&__uniffi_host, #name) {
            Ok(value) => value,
            Err(error) => #error,
          };
        });
      }
    }
  }

  let RustOperationTarget::Native { call } = &operation.target else {
    return Err(EngineError::UnexpectedHostOperationCodegen {
      operation_id: operation.operation_id,
    });
  };
  let invoke = if async_kind == AsyncKind::Async {
    quote!(#call(#(#argument_names),*).await)
  } else {
    quote!(#call(#(#argument_names),*))
  };
  let value = match &operation.error_binding {
    ErrorBinding::Infallible => quote!(let __uniffi_value = #invoke;),
    ErrorBinding::Descriptor { map } => quote! {
      let __uniffi_value = match #invoke {
        Ok(value) => value,
        Err(error) => return napi_uniffi_engine::NapiCallResult::Error(#map(error)),
      };
    },
  };
  let value_manual = match &operation.error_binding {
    ErrorBinding::Infallible => quote!(let __uniffi_value = #invoke;),
    ErrorBinding::Descriptor { map } => quote! {
      let __uniffi_value = match #invoke {
        Ok(value) => value,
        Err(error) => return Ok(napi_uniffi_engine::NapiCallResult::Error(#map(error))),
      };
    },
  };
  let lift = match &operation.return_binding {
    ReturnBinding::Unit | ReturnBinding::Direct { .. } => quote!(__uniffi_value),
    ReturnBinding::I64BigInt | ReturnBinding::U64BigInt => quote!({
      let parts = napi_uniffi_engine::napi_family_core::BigIntWords::from(__uniffi_value);
      napi::bindgen_prelude::BigInt {
        sign_bit: parts.negative,
        words: parts.words,
      }
    }),
    ReturnBinding::LiftWith { lift, .. }
    | ReturnBinding::ObjectLease { lift, .. }
    | ReturnBinding::CallbackLease { lift, .. }
    | ReturnBinding::OutputStreamLease { lift, .. } => quote!({
      match #lift(__uniffi_value) {
        Ok(value) => value,
        Err(error) => return napi_uniffi_engine::NapiCallResult::Error(error),
      }
    }),
  };
  let lift_manual = match &operation.return_binding {
    ReturnBinding::Unit | ReturnBinding::Direct { .. } => quote!(__uniffi_value),
    ReturnBinding::I64BigInt | ReturnBinding::U64BigInt => quote!({
      let parts = napi_uniffi_engine::napi_family_core::BigIntWords::from(__uniffi_value);
      napi::bindgen_prelude::BigInt {
        sign_bit: parts.negative,
        words: parts.words,
      }
    }),
    ReturnBinding::LiftWith { lift, .. }
    | ReturnBinding::ObjectLease { lift, .. }
    | ReturnBinding::CallbackLease { lift, .. }
    | ReturnBinding::OutputStreamLease { lift, .. } => quote!({
      match #lift(__uniffi_value) {
        Ok(value) => value,
        Err(error) => return Ok(napi_uniffi_engine::NapiCallResult::Error(error)),
      }
    }),
  };
  let async_token = if async_kind == AsyncKind::Async {
    quote!(async)
  } else {
    quote!()
  };
  let argument_declarations = operation.arguments.iter().map(|argument| {
    let name = &argument.name;
    let carrier_type = argument.binding.carrier_type();
    quote!(#name: #carrier_type)
  });
  let receiver_declaration = operation.receiver.iter().map(|receiver| {
    let receiver_name = &receiver.name;
    let carrier_type = receiver.binding.carrier_type();
    quote!(#receiver_name: #carrier_type,)
  });
  let host_declaration =
    requires_host.then(|| quote!(__uniffi_host: napi::bindgen_prelude::Object<'static>,));
  let transfer_declaration = callback_transfer.then(|| {
    quote! {
      __uniffi_session_generation: u32,
      __uniffi_callback_transfer: u32,
    }
  });
  // Synchronous operations keep the Host borrow marker to satisfy Rust's
  // unused-parameter checks.  Async Host operations use the dedicated entry
  // above, so no raw Host value is present in their worker future.
  let keep_host_alive = if requires_host && async_kind == AsyncKind::Sync {
    quote!(let _ = &__uniffi_host;)
  } else {
    quote!()
  };
  let callback_transfers = if callback_transfer {
    let error = lower_error(quote!(
      napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string())
    ));
    quote! {
      let __uniffi_callback_transfers = match napi_uniffi_engine::take_session_callback_transfers(
        __uniffi_session_generation,
        __uniffi_callback_transfer,
      ) {
        Ok(value) => value,
        Err(error) => #error,
      };
    }
  } else {
    quote! {
      let __uniffi_callback_transfers =
        napi_uniffi_engine::SessionCallbackTransfers::empty();
    }
  };
  let return_carrier = operation.return_binding.carrier_type();
  let env_declaration = manual_async_entry.then(|| quote!(__uniffi_env: &napi::Env,));
  let body = if manual_async_entry {
    quote! {
      #[doc(hidden)]
      fn #function_name(
        #host_declaration
        #env_declaration
        #transfer_declaration
        #(#receiver_declaration)*
        #(#argument_declarations),*
      ) -> napi::Result<napi::bindgen_prelude::sys::napi_value> {
        #callback_transfers
        #(#lowerings)*
        let __uniffi_future = async move {
          #value_manual
          Ok::<napi_uniffi_engine::NapiCallResult<#return_carrier>, napi::Error>(
            napi_uniffi_engine::NapiCallResult::Value(#lift_manual)
          )
        };
        let __uniffi_promise = __uniffi_env.spawn_future(__uniffi_future)?;
        Ok(napi::bindgen_prelude::JsValue::raw(&__uniffi_promise))
      }
    }
  } else {
    quote! {
      #[doc(hidden)]
      #async_token fn #function_name(
        #host_declaration
        #transfer_declaration
        #(#receiver_declaration)*
        #(#argument_declarations),*
      ) -> napi_uniffi_engine::NapiCallResult<#return_carrier> {
        #keep_host_alive
        #callback_transfers
        #(#lowerings)*
        #value
        napi_uniffi_engine::NapiCallResult::Value(#lift)
      }
    }
  };

  Ok(GeneratedOperation {
    function_name,
    callback_factory,
    body,
    function: function_builder.build(),
  })
}

fn callback_contract_tokens(use_site: &CallbackUseSite) -> TokenStream {
  let callback_type_id = use_site.callback_type_id;
  let retention = match use_site.contract.retention {
    CallbackRetention::Scoped => quote!(napi_uniffi_engine::SessionCallbackRetention::Scoped),
    CallbackRetention::Retained => quote!(napi_uniffi_engine::SessionCallbackRetention::Retained),
  };
  let threading = match use_site.contract.threading {
    CallbackThreading::CallingThread => {
      quote!(napi_uniffi_engine::SessionCallbackThreading::CallingThread)
    }
    CallbackThreading::MayCrossThread => {
      quote!(napi_uniffi_engine::SessionCallbackThreading::MayCrossThread)
    }
  };
  let reentrancy = match use_site.contract.reentrancy {
    CallbackReentrancy::Allowed => {
      quote!(napi_uniffi_engine::SessionCallbackReentrancy::Allowed)
    }
    CallbackReentrancy::Forbidden => {
      quote!(napi_uniffi_engine::SessionCallbackReentrancy::Forbidden)
    }
  };
  let path_tokens = session_path_tokens(&use_site.path);
  quote! {
    napi_uniffi_engine::SessionCallbackArgument {
      path: vec![#(#path_tokens),*],
      callback_type_id: #callback_type_id,
      retention: #retention,
      threading: #threading,
      reentrancy: #reentrancy,
    }
  }
}

struct GeneratedFactory {
  body: TokenStream,
  function: NapiFn,
}

#[derive(Default)]
struct GeneratedResourceCallbacks {
  release_object: Option<Ident>,
  cancel_output_stream: Option<Ident>,
  release_output_stream: Option<Ident>,
}

fn generate_resource_callbacks(
  hooks: &RustResourceHooks,
  source: &mut TokenStream,
) -> Result<GeneratedResourceCallbacks, EngineError> {
  Ok(GeneratedResourceCallbacks {
    release_object: hooks
      .release_object
      .as_ref()
      .map(|hook| generate_resource_callback("release_object", hook, false, source))
      .transpose()?,
    cancel_output_stream: hooks
      .cancel_output_stream
      .as_ref()
      .map(|hook| generate_resource_callback("cancel_output_stream", hook, true, source))
      .transpose()?,
    release_output_stream: hooks
      .release_output_stream
      .as_ref()
      .map(|hook| generate_resource_callback("release_output_stream", hook, false, source))
      .transpose()?,
  })
}

fn generate_resource_callback(
  name: &str,
  hook: &RustResourceHook,
  asynchronous: bool,
  source: &mut TokenStream,
) -> Result<Ident, EngineError> {
  let function_name = format_ident!("__uniffi_{name}");
  let callback_factory = format_ident!("_napi_rs_internal_register___uniffi_{name}");
  let register_name = format_ident!("__napi_register_uniffi_{name}");
  let carrier_type = &hook.carrier_type;
  let call = &hook.call;
  let invoke = if asynchronous {
    quote!(#call(handle).await)
  } else {
    quote!(#call(handle))
  };
  let async_token = asynchronous.then(|| quote!(async));
  source.extend(quote! {
    #[doc(hidden)]
    #async_token fn #function_name(
      handle: #carrier_type,
    ) -> napi_uniffi_engine::NapiCallResult<()> {
      match #invoke {
        Ok(()) => napi_uniffi_engine::NapiCallResult::Value(()),
        Err(error) => napi_uniffi_engine::NapiCallResult::Error(error),
      }
    }
  });
  let function = NapiFnBuilder::new(function_name.clone(), function_name.to_string())
    .argument(NapiFnArg {
      kind: NapiFnArgKind::PatType(Box::new(syn::parse_quote!(handle: #carrier_type))),
      ts_arg_type: None,
    })
    .return_type(syn::parse_quote!(napi_uniffi_engine::NapiCallResult<()>))
    .asynchronous(asynchronous)
    .strict(true)
    .skip_typescript(true)
    .private(true)
    .register_name(register_name)
    .build();
  function
    .try_to_tokens(source)
    .map_err(|error| EngineError::BackendCodegen(format!("{error:?}")))?;
  Ok(callback_factory)
}

fn generate_factory(
  flavor: HostFlavor,
  close_policy: ClosePolicy,
  family: &FamilyPlan,
  rust_operations: &[RustOperationPlan],
  callbacks: &[Option<Ident>],
  resource_callbacks: &GeneratedResourceCallbacks,
) -> Result<GeneratedFactory, EngineError> {
  let function_name = Ident::new(BACKEND_FACTORY_EXPORT, Span::call_site());
  let register_name = Ident::new("__napi_register_uniffi_backend_factory", Span::call_site());
  let function = NapiFnBuilder::new(function_name.clone(), BACKEND_FACTORY_EXPORT)
    .argument(NapiFnArg {
      kind: NapiFnArgKind::PatType(Box::new(syn::parse_quote!(env: &napi::Env))),
      ts_arg_type: None,
    })
    .argument(NapiFnArg {
      kind: NapiFnArgKind::PatType(Box::new(
        syn::parse_quote!(host: napi::bindgen_prelude::Object<'static>),
      )),
      ts_arg_type: None,
    })
    .result_return_type(syn::parse_quote!(napi::bindgen_prelude::Object<'static>))
    .strict(true)
    .skip_typescript(true)
    .register_name(register_name)
    .build();
  let flavor = match flavor {
    HostFlavor::Node => "node",
    HostFlavor::Ohos => "ohos",
  };
  let grace_ms = close_policy.grace_ms;
  let on_deadline = match close_policy.on_deadline {
    DeadlineAction::Detach => quote!(napi_uniffi_engine::DeadlineAction::Detach),
  };
  let descriptors = family
    .operations()
    .iter()
    .zip(rust_operations)
    .zip(callbacks)
    .map(|((operation, rust_operation), callback)| {
      session_descriptor(operation, rust_operation, callback.as_ref())
    })
    .collect::<Result<Vec<_>, _>>()?;
  let release_object = optional_callback_factory(resource_callbacks.release_object.as_ref());
  let cancel_output_stream =
    optional_callback_factory(resource_callbacks.cancel_output_stream.as_ref());
  let release_output_stream =
    optional_callback_factory(resource_callbacks.release_output_stream.as_ref());
  let body = quote! {
    #[doc(hidden)]
    fn #function_name(
      env: &napi::Env,
      host: napi::bindgen_prelude::Object<'static>,
    ) -> napi::Result<napi::bindgen_prelude::Object<'static>> {
      napi_uniffi_engine::create_backend_session(
        env,
        host,
        #flavor,
        napi_uniffi_engine::ClosePolicy {
          grace_ms: #grace_ms,
          on_deadline: #on_deadline,
        },
        vec![#(#descriptors),*],
        napi_uniffi_engine::SessionResourceCallbacks {
          release_object: #release_object,
          cancel_output_stream: #cancel_output_stream,
          release_output_stream: #release_output_stream,
        },
      )
    }
  };
  Ok(GeneratedFactory { body, function })
}

fn optional_callback_factory(callback: Option<&Ident>) -> TokenStream {
  callback.map_or_else(
    || quote!(None),
    |callback| quote!(Some(unsafe { #callback(env.raw())? })),
  )
}

fn session_descriptor(
  operation: &FamilyOperation,
  rust_operation: &RustOperationPlan,
  callback: Option<&Ident>,
) -> Result<TokenStream, EngineError> {
  let operation_id = operation.id;
  let dispatch = match operation.target {
    FamilyOperationTarget::Native => match operation.async_kind {
      AsyncKind::Sync => quote!(napi_uniffi_engine::SessionOperationDispatch::NativeSync),
      AsyncKind::Async => quote!(napi_uniffi_engine::SessionOperationDispatch::NativeAsync),
    },
    FamilyOperationTarget::CallbackHost {
      callback_type_id,
      method_id,
    } => match operation.async_kind {
      AsyncKind::Sync => quote! {
        napi_uniffi_engine::SessionOperationDispatch::CallbackHostSync {
          callback_type_id: #callback_type_id,
          method_id: #method_id,
        }
      },
      AsyncKind::Async => quote! {
        napi_uniffi_engine::SessionOperationDispatch::CallbackHostAsync {
          callback_type_id: #callback_type_id,
          method_id: #method_id,
        }
      },
    },
    FamilyOperationTarget::InputStreamHostPull => {
      if operation.async_kind != AsyncKind::Async {
        return Err(EngineError::InvalidHostOperationSignature {
          operation_id,
          reason: "input-stream pull must be async",
        });
      }
      quote!(napi_uniffi_engine::SessionOperationDispatch::InputStreamHostPull)
    }
    FamilyOperationTarget::InputStreamHostCancel => {
      if operation.async_kind != AsyncKind::Async {
        return Err(EngineError::InvalidHostOperationSignature {
          operation_id,
          reason: "input-stream cancel must be async",
        });
      }
      quote!(napi_uniffi_engine::SessionOperationDispatch::InputStreamHostCancel)
    }
  };
  let callback = match (operation.target, callback) {
    (FamilyOperationTarget::Native, Some(callback)) => {
      quote!(Some(unsafe { #callback(env.raw())? }))
    }
    (FamilyOperationTarget::Native, None) => {
      return Err(EngineError::MissingNativeCallback { operation_id });
    }
    (_, None) => quote!(None),
    (_, Some(_)) => {
      return Err(EngineError::UnexpectedNativeCallback { operation_id });
    }
  };
  let receiver = match operation.receiver {
    None => quote!(None),
    Some(ReceiverBinding::Value) => quote!(Some(napi_uniffi_engine::SessionReceiver::Value)),
    Some(ReceiverBinding::Resource(resource)) => {
      let resource = match resource.kind {
        ResourceKind::Object => quote!(napi_uniffi_engine::SessionResourceReceiver::Object),
        ResourceKind::InputStream => {
          quote!(napi_uniffi_engine::SessionResourceReceiver::InputStream)
        }
        ResourceKind::OutputStream => {
          quote!(napi_uniffi_engine::SessionResourceReceiver::OutputStream)
        }
      };
      quote!(Some(napi_uniffi_engine::SessionReceiver::Resource(#resource)))
    }
  };
  let result = match operation.result.map(|resource| resource.kind) {
    None => quote!(None),
    Some(ResourceKind::Object) => {
      quote!(Some(napi_uniffi_engine::SessionResourceReceiver::Object))
    }
    Some(ResourceKind::InputStream) => {
      quote!(Some(
        napi_uniffi_engine::SessionResourceReceiver::InputStream
      ))
    }
    Some(ResourceKind::OutputStream) => {
      quote!(Some(
        napi_uniffi_engine::SessionResourceReceiver::OutputStream
      ))
    }
  };
  let native_call = if !operation_requires_host(rust_operation, operation) {
    quote!(napi_uniffi_engine::SessionNativeCall::ArgumentsOnly)
  } else {
    quote!(napi_uniffi_engine::SessionNativeCall::HostAndArguments)
  };
  let callback_arguments = operation
    .callbacks
    .iter()
    .map(|use_site| {
      if !matches!(
        use_site.path.segments().first(),
        Some(ValuePathSegment::Argument(_) | ValuePathSegment::Return)
      ) {
        return Err(EngineError::UnsupportedUseSite {
          operation_id,
          role: "callback",
          path: use_site.path.to_string(),
        });
      }
      let path_tokens = session_path_tokens(&use_site.path);
      let callback_type_id = use_site.callback_type_id;
      let retention = match use_site.contract.retention {
        CallbackRetention::Scoped => quote!(napi_uniffi_engine::SessionCallbackRetention::Scoped),
        CallbackRetention::Retained => {
          quote!(napi_uniffi_engine::SessionCallbackRetention::Retained)
        }
      };
      let threading = match use_site.contract.threading {
        CallbackThreading::CallingThread => {
          quote!(napi_uniffi_engine::SessionCallbackThreading::CallingThread)
        }
        CallbackThreading::MayCrossThread => {
          quote!(napi_uniffi_engine::SessionCallbackThreading::MayCrossThread)
        }
      };
      let reentrancy = match use_site.contract.reentrancy {
        CallbackReentrancy::Allowed => {
          quote!(napi_uniffi_engine::SessionCallbackReentrancy::Allowed)
        }
        CallbackReentrancy::Forbidden => {
          quote!(napi_uniffi_engine::SessionCallbackReentrancy::Forbidden)
        }
      };
      Ok(quote! {
        napi_uniffi_engine::SessionCallbackArgument {
          path: vec![#(#path_tokens),*],
          callback_type_id: #callback_type_id,
          retention: #retention,
          threading: #threading,
          reentrancy: #reentrancy,
        }
      })
    })
    .collect::<Result<Vec<_>, _>>()?;
  let stream_arguments = operation
    .streams
    .iter()
    .filter_map(|use_site| match use_site.direction {
      StreamDirection::Input => Some(use_site),
      StreamDirection::Output => None,
    })
    .map(|use_site| {
      let Some(ValuePathSegment::Argument(_)) = use_site.path.segments().first() else {
        return Err(EngineError::UnsupportedUseSite {
          operation_id,
          role: "input stream",
          path: use_site.path.to_string(),
        });
      };
      let path_tokens = session_path_tokens(&use_site.path);
      let use_site_id = use_site.use_site_id;
      Ok(quote! {
        napi_uniffi_engine::SessionStreamArgument {
          path: vec![#(#path_tokens),*],
          use_site_id: #use_site_id,
          direction: napi_uniffi_engine::SessionStreamDirection::Input,
        }
      })
    })
    .collect::<Result<Vec<_>, _>>()?;
  let callback_transfer = operation.target == FamilyOperationTarget::Native
    && operation.callbacks.iter().any(|use_site| {
      matches!(
        use_site.path.segments().first(),
        Some(ValuePathSegment::Argument(_))
      )
    });
  for use_site in operation
    .streams
    .iter()
    .filter(|use_site| use_site.direction == StreamDirection::Output)
  {
    if !matches!(
      use_site.path.segments().first(),
      Some(ValuePathSegment::Return)
    ) {
      return Err(EngineError::UnsupportedUseSite {
        operation_id,
        role: "output stream",
        path: use_site.path.to_string(),
      });
    }
  }

  Ok(quote! {
    napi_uniffi_engine::SessionOperationDescriptor {
      dispatch: #dispatch,
      callback: #callback,
      native_call: #native_call,
      receiver: #receiver,
      result: #result,
      callback_transfer: #callback_transfer,
      callback_arguments: vec![#(#callback_arguments),*],
      stream_arguments: vec![#(#stream_arguments),*],
    }
  })
}

fn session_path_tokens(path: &ValuePath) -> Vec<TokenStream> {
  path
    .segments()
    .iter()
    .map(|segment| match segment {
      ValuePathSegment::Argument(index) => {
        quote!(napi_uniffi_engine::SessionValuePathSegment::Argument(#index))
      }
      ValuePathSegment::Return => quote!(napi_uniffi_engine::SessionValuePathSegment::Return),
      ValuePathSegment::Field(name) => {
        quote!(napi_uniffi_engine::SessionValuePathSegment::Field(#name.to_owned()))
      }
      ValuePathSegment::Variant(name) => {
        quote!(napi_uniffi_engine::SessionValuePathSegment::Variant(#name.to_owned()))
      }
      ValuePathSegment::Optional => {
        quote!(napi_uniffi_engine::SessionValuePathSegment::Optional)
      }
      ValuePathSegment::SequenceElement => {
        quote!(napi_uniffi_engine::SessionValuePathSegment::SequenceElement)
      }
      ValuePathSegment::MapKey => quote!(napi_uniffi_engine::SessionValuePathSegment::MapKey),
      ValuePathSegment::MapValue => quote!(napi_uniffi_engine::SessionValuePathSegment::MapValue),
      ValuePathSegment::SetElement => {
        quote!(napi_uniffi_engine::SessionValuePathSegment::SetElement)
      }
    })
    .collect()
}

#[derive(Debug)]
pub enum EngineError {
  Family(FamilyPlanError),
  DuplicateRustOperation {
    id: u32,
  },
  RustOperationCount {
    expected: usize,
    actual: usize,
  },
  MissingRustOperation {
    id: u32,
  },
  TooManyOperations,
  ArgumentCount {
    operation_id: u32,
    expected: usize,
    actual: usize,
  },
  DuplicateRustArgument {
    operation_id: u32,
    name: String,
  },
  InvalidArgumentBinding {
    operation_id: u32,
    argument: usize,
    expected: &'static str,
  },
  InvalidStructuredBinding {
    operation_id: u32,
    argument: usize,
    role: &'static str,
  },
  InvalidReturnBinding {
    operation_id: u32,
    expected: &'static str,
  },
  MissingErrorDescriptor {
    operation_id: u32,
  },
  UnexpectedErrorDescriptor {
    operation_id: u32,
  },
  InvalidOperationTarget {
    operation_id: u32,
    kind: OperationKind,
  },
  HostOperationHasRustBindings {
    operation_id: u32,
  },
  HostOperationHasStructuredUseSites {
    operation_id: u32,
  },
  MissingReceiver {
    operation_id: u32,
  },
  UnexpectedReceiver {
    operation_id: u32,
  },
  InvalidValueReceiver {
    operation_id: u32,
  },
  InvalidResourceReceiver {
    operation_id: u32,
  },
  InvalidResourceResult {
    operation_id: u32,
  },
  MissingStructuredUseSite {
    operation_id: u32,
    argument: usize,
    role: &'static str,
  },
  UnexpectedHostOperationCodegen {
    operation_id: u32,
  },
  MissingResourceHook {
    role: &'static str,
  },
  OperationOrder {
    expected: u32,
    actual: u32,
  },
  MissingNativeCallback {
    operation_id: u32,
  },
  UnexpectedNativeCallback {
    operation_id: u32,
  },
  InvalidHostOperationSignature {
    operation_id: u32,
    reason: &'static str,
  },
  UnsupportedUseSite {
    operation_id: u32,
    role: &'static str,
    path: String,
  },
  BackendCodegen(String),
}

impl fmt::Display for EngineError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Family(error) => error.fmt(formatter),
      Self::DuplicateRustOperation { id } => write!(formatter, "duplicate Rust operation ID {id}"),
      Self::RustOperationCount { expected, actual } => write!(
        formatter,
        "Rust operation plan has {actual} entries; bridge requires {expected}"
      ),
      Self::MissingRustOperation { id } => write!(formatter, "missing Rust operation ID {id}"),
      Self::TooManyOperations => formatter.write_str("Rust operation table exceeds u32"),
      Self::ArgumentCount {
        operation_id,
        expected,
        actual,
      } => write!(
        formatter,
        "operation {operation_id} has {actual} Rust arguments; bridge requires {expected}"
      ),
      Self::DuplicateRustArgument { operation_id, name } => write!(
        formatter,
        "operation {operation_id} repeats Rust argument name {name:?}"
      ),
      Self::InvalidArgumentBinding {
        operation_id,
        argument,
        expected,
      } => write!(
        formatter,
        "operation {operation_id} argument {argument} requires {expected}"
      ),
      Self::InvalidStructuredBinding {
        operation_id,
        argument,
        role,
      } => write!(
        formatter,
        "operation {operation_id} argument {argument} requires structured {role} binding"
      ),
      Self::InvalidReturnBinding {
        operation_id,
        expected,
      } => write!(
        formatter,
        "operation {operation_id} return requires {expected}"
      ),
      Self::MissingErrorDescriptor { operation_id } => write!(
        formatter,
        "fallible operation {operation_id} has no error descriptor mapper"
      ),
      Self::UnexpectedErrorDescriptor { operation_id } => write!(
        formatter,
        "infallible operation {operation_id} unexpectedly has an error descriptor mapper"
      ),
      Self::InvalidOperationTarget { operation_id, kind } => write!(
        formatter,
        "operation {operation_id} has a Rust target incompatible with {kind:?}"
      ),
      Self::HostOperationHasRustBindings { operation_id } => write!(
        formatter,
        "host operation {operation_id} must not contain Rust call arguments or a receiver"
      ),
      Self::HostOperationHasStructuredUseSites { operation_id } => write!(
        formatter,
        "host operation {operation_id} must not contain callback or stream use-sites"
      ),
      Self::MissingReceiver { operation_id } => {
        write!(
          formatter,
          "operation {operation_id} has no Rust receiver binding"
        )
      }
      Self::UnexpectedReceiver { operation_id } => write!(
        formatter,
        "operation {operation_id} unexpectedly has a Rust receiver binding"
      ),
      Self::InvalidValueReceiver { operation_id } => write!(
        formatter,
        "value receiver operation {operation_id} requires a regular value lowering"
      ),
      Self::InvalidResourceReceiver { operation_id } => write!(
        formatter,
        "resource receiver operation {operation_id} requires a matching resource lease binding"
      ),
      Self::InvalidResourceResult { operation_id } => write!(
        formatter,
        "operation {operation_id} has a return binding incompatible with its resource result"
      ),
      Self::MissingStructuredUseSite {
        operation_id,
        argument,
        role,
      } => write!(
        formatter,
        "operation {operation_id} argument {argument} has no structured {role} use-site contract"
      ),
      Self::UnexpectedHostOperationCodegen { operation_id } => write!(
        formatter,
        "host operation {operation_id} reached native operation code generation"
      ),
      Self::MissingResourceHook { role } => {
        write!(formatter, "Rust bridge plan has no structured {role} hook")
      }
      Self::OperationOrder { expected, actual } => write!(
        formatter,
        "operation dispatch order mismatch: expected {expected}, found {actual}"
      ),
      Self::MissingNativeCallback { operation_id } => {
        write!(
          formatter,
          "native operation {operation_id} has no generated callback"
        )
      }
      Self::UnexpectedNativeCallback { operation_id } => write!(
        formatter,
        "host operation {operation_id} unexpectedly has a native callback"
      ),
      Self::InvalidHostOperationSignature {
        operation_id,
        reason,
      } => write!(
        formatter,
        "host operation {operation_id} is invalid: {reason}"
      ),
      Self::UnsupportedUseSite {
        operation_id,
        role,
        path,
      } => write!(
        formatter,
        "operation {operation_id} has unsupported nested {role} use-site {path}"
      ),
      Self::BackendCodegen(error) => write!(formatter, "napi backend codegen failed: {error}"),
    }
  }
}

impl Error for EngineError {}

impl From<FamilyPlanError> for EngineError {
  fn from(value: FamilyPlanError) -> Self {
    Self::Family(value)
  }
}
