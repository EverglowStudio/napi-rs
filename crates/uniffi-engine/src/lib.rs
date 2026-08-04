//! Programmatic UniFFI frontend for napi-rs.
//!
//! The frontend accepts only an already validated [`BridgePlan`] and a
//! structured Rust call plan.  It performs no component discovery and has no
//! process or filesystem inputs.  Raw operation callbacks stay private; the
//! generated module registers one backend factory containing a dense function
//! table.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ffi::CString;
use std::fmt;
use std::ptr;

use napi::bindgen_prelude::{BigInt, ToNapiValue};
use napi::{sys, Result as NapiResult};
use napi_derive_backend::{NapiFn, NapiFnArg, NapiFnArgKind, NapiFnBuilder, TryToTokens};
use napi_family_core::{
  BigIntWords, FamilyOperation, FamilyOperationTarget, FamilyPlan, FamilyPlanError, HostFlavor,
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Path, Type};
use uniffi_js_abi::{
  AsyncKind, OperationId, OperationKind, OperationOwner, Ownership, ScalarType, ValueType,
};
use uniffi_js_engine_schema::{
  BridgePlan, CallbackReentrancy, CallbackRetention, CallbackThreading, StreamDirection,
  ValuePathSegment,
};

pub use napi_family_core;
mod session;
pub use session::{
  create_backend_session, SessionCallbackArgument, SessionCallbackReentrancy,
  SessionCallbackRetention, SessionCallbackThreading, SessionNativeCall,
  SessionOperationDescriptor, SessionOperationDispatch, SessionResourceCallbacks,
  SessionResourceReceiver, SessionStreamArgument, SessionStreamDirection,
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
  ObjectLease {
    carrier_type: Type,
    lower: Path,
    ownership: Ownership,
  },
  OutputStreamLease {
    carrier_type: Type,
    lower: Path,
    ownership: Ownership,
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
  pub operation_id: OperationId,
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
    bridge: &BridgePlan,
    operations: Vec<RustOperationPlan>,
  ) -> Result<Self, EngineError> {
    Self::build_with_resource_hooks(bridge, operations, RustResourceHooks::default())
  }

  pub fn build_with_resource_hooks(
    bridge: &BridgePlan,
    operations: Vec<RustOperationPlan>,
    resource_hooks: RustResourceHooks,
  ) -> Result<Self, EngineError> {
    let type_kinds = bridge
      .types()
      .iter()
      .map(|ty| (&ty.definition.source_key, &ty.definition.kind))
      .collect::<BTreeMap<_, _>>();
    let mut by_id = BTreeMap::new();
    for operation in operations {
      let id = operation.operation_id.index();
      if by_id.insert(id, operation).is_some() {
        return Err(EngineError::DuplicateRustOperation { id });
      }
    }
    if by_id.len() != bridge.operations().len() {
      return Err(EngineError::RustOperationCount {
        expected: bridge.operations().len(),
        actual: by_id.len(),
      });
    }

    let mut validated = Vec::with_capacity(by_id.len());
    for (expected, bridge_operation) in bridge.operations().iter().enumerate() {
      let expected = u32::try_from(expected).map_err(|_| EngineError::TooManyOperations)?;
      let Some(operation) = by_id.remove(&expected) else {
        return Err(EngineError::MissingRustOperation { id: expected });
      };
      let signature = &bridge_operation.operation.definition.signature;
      validate_operation_target(
        &bridge_operation.operation.definition.source_key,
        &operation,
      )?;
      let is_native = matches!(operation.target, RustOperationTarget::Native { .. });
      if is_native {
        if operation.arguments.len() != signature.arguments.len() {
          return Err(EngineError::ArgumentCount {
            operation_id: operation.operation_id,
            expected: signature.arguments.len(),
            actual: operation.arguments.len(),
          });
        }
        validate_receiver(
          &bridge_operation.operation.definition.source_key,
          &operation,
        )?;
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
        for (index, (argument, semantic)) in operation
          .arguments
          .iter()
          .zip(&signature.arguments)
          .enumerate()
        {
          validate_argument_binding(
            operation.operation_id,
            index,
            &semantic.ty,
            semantic.ownership,
            &argument.binding,
            &type_kinds,
          )?;
        }
        validate_return_binding(
          operation.operation_id,
          signature.return_type.as_ref(),
          &operation.return_binding,
          &type_kinds,
        )?;
        match (&signature.throws, &operation.error_binding) {
          (None, ErrorBinding::Infallible) | (Some(_), ErrorBinding::Descriptor { .. }) => {}
          (Some(_), ErrorBinding::Infallible) => {
            return Err(EngineError::MissingErrorDescriptor {
              operation_id: operation.operation_id,
            });
          }
          (None, ErrorBinding::Descriptor { .. }) => {
            return Err(EngineError::UnexpectedErrorDescriptor {
              operation_id: operation.operation_id,
            });
          }
        }
      }
      validated.push(operation);
    }

    validate_resource_hooks(bridge, &resource_hooks)?;
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
  bridge: &BridgePlan,
  hooks: &RustResourceHooks,
) -> Result<(), EngineError> {
  let needs_object = bridge.operations().iter().any(|operation| {
    operation
      .required_capabilities
      .contains(uniffi_js_engine_schema::Capability::ObjectLease)
      || matches!(
        operation.operation.definition.source_key.owner(),
        OperationOwner::Object(_)
      ) && matches!(
        operation.operation.definition.source_key.kind(),
        OperationKind::Method | OperationKind::Constructor
      )
  });
  let needs_output = bridge.operations().iter().any(|operation| {
    operation
      .required_capabilities
      .contains(uniffi_js_engine_schema::Capability::OutputStream)
      || matches!(
        operation.operation.definition.source_key.kind(),
        OperationKind::OutputStreamStart
          | OperationKind::OutputStreamNext
          | OperationKind::OutputStreamCancel
      )
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

fn validate_argument_binding(
  operation_id: OperationId,
  argument: usize,
  value: &ValueType,
  ownership: Ownership,
  binding: &ArgumentBinding,
  type_kinds: &BTreeMap<&uniffi_js_abi::TypeSourceKey, &uniffi_js_abi::NamedTypeKind>,
) -> Result<(), EngineError> {
  let valid = match value {
    ValueType::Scalar(ScalarType::I64) => matches!(binding, ArgumentBinding::I64BigInt),
    ValueType::Scalar(ScalarType::U64) => matches!(binding, ArgumentBinding::U64BigInt),
    ValueType::Scalar(
      ScalarType::Bool
      | ScalarType::I8
      | ScalarType::U8
      | ScalarType::I16
      | ScalarType::U16
      | ScalarType::I32
      | ScalarType::U32
      | ScalarType::F32
      | ScalarType::F64
      | ScalarType::String,
    ) => matches!(
      binding,
      ArgumentBinding::Direct { .. } | ArgumentBinding::LowerWith { .. }
    ),
    ValueType::Named(key) => match type_kinds.get(key) {
      Some(uniffi_js_abi::NamedTypeKind::Callback) => {
        matches!(binding, ArgumentBinding::CallbackProxy { .. })
      }
      Some(uniffi_js_abi::NamedTypeKind::Object) => matches!(
        binding,
        ArgumentBinding::ObjectLease {
          ownership: binding_ownership,
          ..
        } if *binding_ownership == ownership
      ),
      Some(_) => matches!(binding, ArgumentBinding::LowerWith { .. }),
      None => false,
    },
    ValueType::InputStream(_) => matches!(binding, ArgumentBinding::InputStreamProxy { .. }),
    ValueType::OutputStream(_) => false,
    _ => matches!(binding, ArgumentBinding::LowerWith { .. }),
  };
  if valid {
    Ok(())
  } else {
    Err(EngineError::InvalidArgumentBinding {
      operation_id,
      argument,
      expected: binding_expectation(value),
    })
  }
}

fn validate_return_binding(
  operation_id: OperationId,
  value: Option<&ValueType>,
  binding: &ReturnBinding,
  type_kinds: &BTreeMap<&uniffi_js_abi::TypeSourceKey, &uniffi_js_abi::NamedTypeKind>,
) -> Result<(), EngineError> {
  let valid = match value {
    None => matches!(binding, ReturnBinding::Unit),
    Some(ValueType::Scalar(ScalarType::I64)) => matches!(binding, ReturnBinding::I64BigInt),
    Some(ValueType::Scalar(ScalarType::U64)) => matches!(binding, ReturnBinding::U64BigInt),
    Some(ValueType::Scalar(
      ScalarType::Bool
      | ScalarType::I8
      | ScalarType::U8
      | ScalarType::I16
      | ScalarType::U16
      | ScalarType::I32
      | ScalarType::U32
      | ScalarType::F32
      | ScalarType::F64
      | ScalarType::String,
    )) => matches!(
      binding,
      ReturnBinding::Direct { .. } | ReturnBinding::LiftWith { .. }
    ),
    Some(ValueType::Named(key)) => match type_kinds.get(key) {
      Some(uniffi_js_abi::NamedTypeKind::Object) => {
        matches!(binding, ReturnBinding::ObjectLease { .. })
      }
      Some(uniffi_js_abi::NamedTypeKind::Callback) => {
        matches!(binding, ReturnBinding::CallbackLease { .. })
      }
      Some(_) => matches!(binding, ReturnBinding::LiftWith { .. }),
      None => false,
    },
    Some(ValueType::OutputStream(_)) => matches!(binding, ReturnBinding::OutputStreamLease { .. }),
    Some(_) => matches!(binding, ReturnBinding::LiftWith { .. }),
  };
  if valid {
    Ok(())
  } else {
    Err(EngineError::InvalidReturnBinding {
      operation_id,
      expected: value.map_or("unit", binding_expectation),
    })
  }
}

fn validate_operation_target(
  source: &uniffi_js_abi::OperationSourceKey,
  operation: &RustOperationPlan,
) -> Result<(), EngineError> {
  let valid = match source.kind() {
    OperationKind::CallbackMethod => matches!(operation.target, RustOperationTarget::CallbackHost),
    OperationKind::InputStreamPull => {
      matches!(operation.target, RustOperationTarget::InputStreamHostPull)
    }
    OperationKind::InputStreamCancel => {
      matches!(operation.target, RustOperationTarget::InputStreamHostCancel)
    }
    _ => matches!(operation.target, RustOperationTarget::Native { .. }),
  };
  if valid {
    Ok(())
  } else {
    Err(EngineError::InvalidOperationTarget {
      operation_id: operation.operation_id,
      kind: source.kind(),
    })
  }
}

fn validate_receiver(
  source: &uniffi_js_abi::OperationSourceKey,
  operation: &RustOperationPlan,
) -> Result<(), EngineError> {
  let required = matches!(
    (source.owner(), source.kind()),
    (OperationOwner::Object(_), OperationKind::Method)
      | (OperationOwner::Object(_), OperationKind::OutputStreamNext)
      | (OperationOwner::Object(_), OperationKind::OutputStreamCancel)
  );
  match (required, &operation.receiver) {
    (false, None) => Ok(()),
    (true, Some(receiver)) => {
      let valid = if matches!(
        source.kind(),
        OperationKind::OutputStreamNext | OperationKind::OutputStreamCancel
      ) {
        matches!(
          receiver.binding,
          ArgumentBinding::OutputStreamLease {
            ownership: Ownership::Borrowed,
            ..
          }
        )
      } else {
        matches!(
          receiver.binding,
          ArgumentBinding::ObjectLease {
            ownership: Ownership::Borrowed,
            ..
          }
        )
      };
      if valid {
        Ok(())
      } else {
        Err(EngineError::InvalidObjectReceiver {
          operation_id: operation.operation_id,
        })
      }
    }
    (true, None) => Err(EngineError::MissingObjectReceiver {
      operation_id: operation.operation_id,
    }),
    (false, Some(_)) => Err(EngineError::UnexpectedObjectReceiver {
      operation_id: operation.operation_id,
    }),
  }
}

fn binding_expectation(value: &ValueType) -> &'static str {
  match value {
    ValueType::Scalar(ScalarType::I64) => "lossless signed BigInt",
    ValueType::Scalar(ScalarType::U64) => "lossless unsigned BigInt",
    ValueType::Scalar(
      ScalarType::Bool
      | ScalarType::I8
      | ScalarType::U8
      | ScalarType::I16
      | ScalarType::U16
      | ScalarType::I32
      | ScalarType::U32
      | ScalarType::F32
      | ScalarType::F64
      | ScalarType::String,
    ) => "direct N-API carrier or explicit adapter",
    _ => "explicit carrier adapter",
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
  bridge: &BridgePlan,
  rust: &RustBridgePlan,
  flavor: HostFlavor,
) -> Result<GeneratedNapiModule, EngineError> {
  let family = FamilyPlan::build(bridge, flavor)?;
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
        expected: family_operation.id.index(),
        actual: operation.operation_id.index(),
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
  let factory = generate_factory(flavor, &family, &callbacks, &resource_callbacks)?;
  source.extend(factory.body);
  factory
    .function
    .try_to_tokens(&mut source)
    .map_err(|error| EngineError::BackendCodegen(format!("{error:?}")))?;

  Ok(GeneratedNapiModule {
    family,
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

fn generate_operation(
  operation: &RustOperationPlan,
  family: &FamilyOperation,
) -> Result<GeneratedOperation, EngineError> {
  let async_kind = family.async_kind;
  let id = operation.operation_id.index();
  let function_name = format_ident!("__uniffi_raw_operation_{id}");
  let callback_factory = format_ident!("_napi_rs_internal_register___uniffi_raw_operation_{id}");
  let return_carrier = operation.return_binding.carrier_type();
  let requires_host = operation.arguments.iter().any(|argument| {
    matches!(
      argument.binding,
      ArgumentBinding::CallbackProxy { .. } | ArgumentBinding::InputStreamProxy { .. }
    )
  });
  let mut function_builder = NapiFnBuilder::new(function_name.clone(), function_name.to_string());
  if requires_host {
    function_builder = function_builder.argument(NapiFnArg {
      kind: NapiFnArgKind::PatType(Box::new(
        syn::parse_quote!(__uniffi_host: napi::bindgen_prelude::Object<'static>),
      )),
      ts_arg_type: None,
    });
  }
  function_builder = function_builder
    .return_type(syn::parse_quote!(napi_uniffi_engine::NapiCallResult<#return_carrier>))
    .asynchronous(async_kind == AsyncKind::Async)
    .strict(true)
    .skip_typescript(true)
    .private(true)
    .register_name(format_ident!("__napi_register_uniffi_raw_operation_{id}"));

  let mut argument_names = Vec::with_capacity(operation.arguments.len());
  let mut lowerings = Vec::new();
  if let Some(receiver) = &operation.receiver {
    let name = &receiver.name;
    let carrier_type = receiver.binding.carrier_type();
    let pat_type: syn::PatType = syn::parse_quote!(#name: #carrier_type);
    function_builder = function_builder.argument(NapiFnArg {
      kind: NapiFnArgKind::PatType(Box::new(pat_type)),
      ts_arg_type: None,
    });
    argument_names.push(name.clone());
    let lower = match &receiver.binding {
      ArgumentBinding::ObjectLease { lower, .. }
      | ArgumentBinding::OutputStreamLease { lower, .. } => lower,
      _ => {
        return Err(EngineError::InvalidObjectReceiver {
          operation_id: operation.operation_id,
        });
      }
    };
    lowerings.push(quote! {
      let #name = match #lower(#name) {
        Ok(value) => value,
        Err(error) => return napi_uniffi_engine::NapiCallResult::Error(error),
      };
    });
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
      ArgumentBinding::I64BigInt => lowerings.push(quote! {
        let (__uniffi_value, __uniffi_lossless) = #name.get_i64();
        let #name = match napi_uniffi_engine::napi_family_core::require_lossless_i64(
          __uniffi_value,
          __uniffi_lossless,
        ) {
          Ok(value) => value,
          Err(error) => return napi_uniffi_engine::NapiCallResult::Error(
            napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()),
          ),
        };
      }),
      ArgumentBinding::U64BigInt => lowerings.push(quote! {
        let (__uniffi_negative, __uniffi_value, __uniffi_lossless) = #name.get_u64();
        let #name = match napi_uniffi_engine::napi_family_core::require_lossless_u64(
          __uniffi_negative,
          __uniffi_value,
          __uniffi_lossless,
        ) {
          Ok(value) => value,
          Err(error) => return napi_uniffi_engine::NapiCallResult::Error(
            napi_uniffi_engine::BridgeErrorDescriptor::validation(error.to_string()),
          ),
        };
      }),
      ArgumentBinding::LowerWith { lower, .. }
      | ArgumentBinding::ObjectLease { lower, .. }
      | ArgumentBinding::OutputStreamLease { lower, .. } => lowerings.push(quote! {
        let #name = match #lower(#name) {
          Ok(value) => value,
          Err(error) => return napi_uniffi_engine::NapiCallResult::Error(error),
        };
      }),
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
        let callback_type_id = callback.callback_type.index();
        let contract = callback_contract_tokens(callback, argument_index as u32);
        lowerings.push(quote! {
          let #name = match #build(
            &__uniffi_host,
            #callback_type_id,
            #name,
            #contract,
          ) {
            Ok(value) => value,
            Err(error) => return napi_uniffi_engine::NapiCallResult::Error(error),
          };
        });
      }
      ArgumentBinding::InputStreamProxy { build, .. } => {
        let found = family.streams.iter().any(|use_site| {
          use_site.contract.direction == StreamDirection::Input
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
        lowerings.push(quote! {
          let #name = match #build(&__uniffi_host, #name) {
            Ok(value) => value,
            Err(error) => return napi_uniffi_engine::NapiCallResult::Error(error),
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
    let name = &receiver.name;
    let carrier_type = receiver.binding.carrier_type();
    quote!(#name: #carrier_type,)
  });
  let host_declaration =
    requires_host.then(|| quote!(__uniffi_host: napi::bindgen_prelude::Object<'static>,));
  let keep_host_alive = requires_host.then(|| quote!(let _ = &__uniffi_host;));
  let return_carrier = operation.return_binding.carrier_type();
  let body = quote! {
    #[doc(hidden)]
    #async_token fn #function_name(
      #host_declaration
      #(#receiver_declaration)*
      #(#argument_declarations),*
    ) -> napi_uniffi_engine::NapiCallResult<#return_carrier> {
      #keep_host_alive
      #(#lowerings)*
      #value
      napi_uniffi_engine::NapiCallResult::Value(#lift)
    }
  };

  Ok(GeneratedOperation {
    function_name,
    callback_factory,
    body,
    function: function_builder.build(),
  })
}

fn callback_contract_tokens(
  use_site: &napi_family_core::FamilyCallbackUseSite,
  argument_index: u32,
) -> TokenStream {
  let callback_type_id = use_site.callback_type.index();
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
  quote! {
    napi_uniffi_engine::SessionCallbackArgument {
      argument_index: #argument_index,
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
  family: &FamilyPlan,
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
  let descriptors = family
    .operations()
    .iter()
    .zip(callbacks)
    .map(|(operation, callback)| session_descriptor(operation, callback.as_ref()))
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
  callback: Option<&Ident>,
) -> Result<TokenStream, EngineError> {
  let operation_id = operation.id;
  let dispatch = match operation.target {
    FamilyOperationTarget::Native => match operation.async_kind {
      AsyncKind::Sync => quote!(napi_uniffi_engine::SessionOperationDispatch::NativeSync),
      AsyncKind::Async => quote!(napi_uniffi_engine::SessionOperationDispatch::NativeAsync),
    },
    FamilyOperationTarget::CallbackHost(method) => {
      let callback_type_id = method.callback_type.index();
      let method_id = method.method_id;
      match operation.async_kind {
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
      }
    }
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
    Some(_)
      if matches!(
        operation.kind,
        uniffi_js_abi::OperationKind::OutputStreamNext
          | uniffi_js_abi::OperationKind::OutputStreamCancel
      ) =>
    {
      quote! {
        Some(napi_uniffi_engine::SessionResourceReceiver::OutputStream)
      }
    }
    Some(_) => quote!(Some(napi_uniffi_engine::SessionResourceReceiver::Object)),
  };
  let native_call = if operation.callbacks.is_empty()
    && !operation
      .streams
      .iter()
      .any(|use_site| use_site.contract.direction == StreamDirection::Input)
  {
    quote!(napi_uniffi_engine::SessionNativeCall::ArgumentsOnly)
  } else {
    quote!(napi_uniffi_engine::SessionNativeCall::HostAndArguments)
  };
  let callback_arguments = operation
    .callbacks
    .iter()
    .map(|use_site| {
      let [ValuePathSegment::Argument(argument_index)] = use_site.path.segments() else {
        return Err(EngineError::UnsupportedUseSite {
          operation_id,
          role: "callback",
          path: use_site.path.to_string(),
        });
      };
      let callback_type_id = use_site.callback_type.index();
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
          argument_index: #argument_index,
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
    .filter_map(|use_site| match use_site.contract.direction {
      StreamDirection::Input => Some(use_site),
      StreamDirection::Output => None,
    })
    .map(|use_site| {
      let [ValuePathSegment::Argument(argument_index)] = use_site.path.segments() else {
        return Err(EngineError::UnsupportedUseSite {
          operation_id,
          role: "input stream",
          path: use_site.path.to_string(),
        });
      };
      Ok(quote! {
        napi_uniffi_engine::SessionStreamArgument {
          argument_index: #argument_index,
          direction: napi_uniffi_engine::SessionStreamDirection::Input,
        }
      })
    })
    .collect::<Result<Vec<_>, _>>()?;
  for use_site in operation
    .streams
    .iter()
    .filter(|use_site| use_site.contract.direction == StreamDirection::Output)
  {
    if !matches!(use_site.path.segments(), [ValuePathSegment::Return]) {
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
      callback_arguments: vec![#(#callback_arguments),*],
      stream_arguments: vec![#(#stream_arguments),*],
    }
  })
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
    operation_id: OperationId,
    expected: usize,
    actual: usize,
  },
  DuplicateRustArgument {
    operation_id: OperationId,
    name: String,
  },
  InvalidArgumentBinding {
    operation_id: OperationId,
    argument: usize,
    expected: &'static str,
  },
  InvalidReturnBinding {
    operation_id: OperationId,
    expected: &'static str,
  },
  MissingErrorDescriptor {
    operation_id: OperationId,
  },
  UnexpectedErrorDescriptor {
    operation_id: OperationId,
  },
  InvalidOperationTarget {
    operation_id: OperationId,
    kind: OperationKind,
  },
  HostOperationHasRustBindings {
    operation_id: OperationId,
  },
  MissingObjectReceiver {
    operation_id: OperationId,
  },
  UnexpectedObjectReceiver {
    operation_id: OperationId,
  },
  InvalidObjectReceiver {
    operation_id: OperationId,
  },
  MissingStructuredUseSite {
    operation_id: OperationId,
    argument: usize,
    role: &'static str,
  },
  UnexpectedHostOperationCodegen {
    operation_id: OperationId,
  },
  MissingResourceHook {
    role: &'static str,
  },
  OperationOrder {
    expected: u32,
    actual: u32,
  },
  MissingNativeCallback {
    operation_id: OperationId,
  },
  UnexpectedNativeCallback {
    operation_id: OperationId,
  },
  InvalidHostOperationSignature {
    operation_id: OperationId,
    reason: &'static str,
  },
  UnsupportedUseSite {
    operation_id: OperationId,
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
      Self::MissingObjectReceiver { operation_id } => {
        write!(
          formatter,
          "object operation {operation_id} has no resource receiver"
        )
      }
      Self::UnexpectedObjectReceiver { operation_id } => write!(
        formatter,
        "non-object operation {operation_id} unexpectedly has a resource receiver"
      ),
      Self::InvalidObjectReceiver { operation_id } => write!(
        formatter,
        "object operation {operation_id} requires a borrowed structured object lease receiver"
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
