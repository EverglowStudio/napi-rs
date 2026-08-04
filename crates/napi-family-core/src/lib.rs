//! Shared, build-time-only planning for Node and OHOS N-API backends.
//!
//! This crate deliberately knows nothing about Node loaders, V8, Ark SDKs,
//! files, or process configuration.  It translates a validated UniFFI
//! [`BridgePlan`] into N-API carrier recipes and explicit host hooks.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use uniffi_js_abi::{
  AsyncKind, NamedTypeKind, OperationId, OperationKind, OperationOwner, Ownership, ScalarType,
  TypeId, TypeSourceKey, ValueType,
};
use uniffi_js_engine_schema::{
  BridgePlan, CallbackContract, CallbackUseSite, Capability, CapabilitySet, EngineCapabilities,
  EngineKind, StreamContract, StreamDirection, StreamUseSite, ValuePath,
};

/// The two hosts that share the N-API family lowering.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HostFlavor {
  Node,
  Ohos,
}

impl HostFlavor {
  pub const fn engine_kind(self) -> EngineKind {
    match self {
      Self::Node => EngineKind::Napi,
      Self::Ohos => EngineKind::OhosNapi,
    }
  }

  pub const fn hooks(self) -> HostHooks {
    match self {
      Self::Node => HostHooks {
        module_registration: ModuleRegistration::Node,
        async_scheduler: AsyncScheduler::NodeEventLoop,
        callback_dispatch: CallbackDispatch::ThreadsafeFunction,
        cleanup_queue: CleanupQueue::EnvironmentHook,
      },
      Self::Ohos => HostHooks {
        module_registration: ModuleRegistration::Ark,
        async_scheduler: AsyncScheduler::ArkEventLoop,
        callback_dispatch: CallbackDispatch::ArkPriorityThreadsafeFunction,
        cleanup_queue: CleanupQueue::ArkRuntime,
      },
    }
  }

  /// Capabilities implemented by the family core plus the flavor hooks.
  ///
  /// Both hosts expose the same callback contract capabilities.  Their
  /// runtime hooks differ, but callback reentrancy is part of the shared
  /// family contract and is enforced by each flavor's generated proxy.
  pub fn capabilities(self) -> EngineCapabilities {
    let mut supported = CapabilitySet::new([
      Capability::Primitive,
      Capability::String,
      Capability::Bytes,
      Capability::BigInt,
      Capability::Optional,
      Capability::Sequence,
      Capability::Map,
      Capability::Set,
      Capability::Record,
      Capability::Enum,
      Capability::DeclaredError,
      Capability::ObjectLease,
      Capability::SyncCall,
      Capability::AsyncCall,
      Capability::Callback,
      Capability::RetainedCallback,
      Capability::AsyncCallback,
      Capability::FallibleCallback,
      Capability::CrossThreadAsyncCallback,
      Capability::InputStream,
      Capability::OutputStream,
    ]);
    supported.insert(Capability::CallbackReentrancy);
    EngineCapabilities {
      engine: self.engine_kind(),
      supported,
    }
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostHooks {
  pub module_registration: ModuleRegistration,
  pub async_scheduler: AsyncScheduler,
  pub callback_dispatch: CallbackDispatch,
  pub cleanup_queue: CleanupQueue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleRegistration {
  Node,
  Ark,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AsyncScheduler {
  NodeEventLoop,
  ArkEventLoop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallbackDispatch {
  ThreadsafeFunction,
  ArkPriorityThreadsafeFunction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CleanupQueue {
  EnvironmentHook,
  ArkRuntime,
}

/// Canonical carrier used at the private N-API boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CarrierRecipe {
  Boolean,
  Number(NumberCarrier),
  BigInt(BigIntCarrier),
  String,
  Uint8Array,
  Optional(Box<CarrierRecipe>),
  Sequence(Box<CarrierRecipe>),
  Map(Box<CarrierRecipe>, Box<CarrierRecipe>),
  Set(Box<CarrierRecipe>),
  Record(TypeId),
  Enum(TypeId),
  ErrorDescriptor(TypeId),
  ObjectLease(TypeId),
  Callback(TypeId),
  InputStream(Box<CarrierRecipe>),
  OutputStream(Box<CarrierRecipe>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumberCarrier {
  I8,
  U8,
  I16,
  U16,
  I32,
  U32,
  F32,
  F64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BigIntCarrier {
  pub signed: bool,
  pub bits: u8,
  pub lossless_required: bool,
}

impl BigIntCarrier {
  pub const I64: Self = Self {
    signed: true,
    bits: 64,
    lossless_required: true,
  };
  pub const U64: Self = Self {
    signed: false,
    bits: 64,
    lossless_required: true,
  };
}

/// Runtime entrypoints required by the operations in one family plan.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RuntimeEntrypoint {
  CloseSession,
  ReleaseObject,
  RetainCallback,
  ReleaseCallback,
  InvokeCallbackSync,
  InvokeCallbackAsync,
  PullInputStream,
  CancelInputStream,
  ReleaseInputStream,
  NextOutputStream,
  CancelOutputStream,
  ReleaseOutputStream,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyArgument {
  pub carrier: CarrierRecipe,
  pub ownership: Ownership,
}

/// The implicit resource receiver carried before the public arguments of an
/// object method.  It is deliberately not forged into the public signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectReceiver {
  pub object_type: TypeId,
  pub ownership: Ownership,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyCallbackUseSite {
  pub callback_type: TypeId,
  pub path: ValuePath,
  pub contract: CallbackContract,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyStreamUseSite {
  pub path: ValuePath,
  pub contract: StreamContract,
}

/// Callback methods are host operations, not forward Rust calls.  `method_id`
/// is dense within the callback type and is the value passed to Host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallbackMethod {
  pub callback_type: TypeId,
  pub method_id: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FamilyOperationTarget {
  Native,
  CallbackHost(CallbackMethod),
  InputStreamHostPull,
  InputStreamHostCancel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyOperation {
  pub id: OperationId,
  pub owner: OperationOwner,
  pub kind: OperationKind,
  pub async_kind: AsyncKind,
  pub receiver: Option<ObjectReceiver>,
  pub arguments: Vec<FamilyArgument>,
  pub return_value: Option<CarrierRecipe>,
  pub declared_error: Option<TypeId>,
  pub callbacks: Vec<FamilyCallbackUseSite>,
  pub streams: Vec<FamilyStreamUseSite>,
  pub target: FamilyOperationTarget,
}

/// A deterministic N-API family plan.  Operation order is the dispatch order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyPlan {
  flavor: HostFlavor,
  hooks: HostHooks,
  operations: Vec<FamilyOperation>,
  runtime_entrypoints: BTreeSet<RuntimeEntrypoint>,
}

impl FamilyPlan {
  pub fn build(bridge: &BridgePlan, flavor: HostFlavor) -> Result<Self, FamilyPlanError> {
    let target = bridge
      .targets()
      .iter()
      .find(|target| target.engine == flavor.engine_kind())
      .ok_or(FamilyPlanError::MissingFlavorTarget { flavor })?;
    let supported = flavor.capabilities().supported;
    for capability in target.supported.iter() {
      if !supported.contains(capability) {
        return Err(FamilyPlanError::UnsupportedFlavorCapability { flavor, capability });
      }
    }

    let types: BTreeMap<&TypeSourceKey, (TypeId, &NamedTypeKind)> = bridge
      .types()
      .iter()
      .map(|ty| (&ty.definition.source_key, (ty.id, &ty.definition.kind)))
      .collect();
    let mut operations = Vec::with_capacity(bridge.operations().len());
    let mut entrypoints = BTreeSet::from([RuntimeEntrypoint::CloseSession]);
    let mut callback_method_ids = BTreeMap::<TypeId, u32>::new();

    for (expected, operation) in bridge.operations().iter().enumerate() {
      let expected = u32::try_from(expected).map_err(|_| FamilyPlanError::TooManyOperations)?;
      if operation.operation.id.index() != expected {
        return Err(FamilyPlanError::NonDenseOperationId {
          expected,
          actual: operation.operation.id.index(),
        });
      }
      let signature = &operation.operation.definition.signature;
      let arguments = signature
        .arguments
        .iter()
        .map(|argument| {
          Ok(FamilyArgument {
            carrier: carrier_for(&argument.ty, &types)?,
            ownership: argument.ownership,
          })
        })
        .collect::<Result<Vec<_>, _>>()?;
      let return_value = signature
        .return_type
        .as_ref()
        .map(|value| carrier_for(value, &types))
        .transpose()?;
      let declared_error = signature
        .throws
        .as_ref()
        .map(|key| named_type(key, &types, "declared error"))
        .transpose()?
        .map(|(id, kind)| {
          if matches!(kind, NamedTypeKind::Error { .. }) {
            Ok(id)
          } else {
            Err(FamilyPlanError::WrongNamedTypeKind {
              key: signature.throws.as_ref().expect("checked above").clone(),
              expected: "error",
            })
          }
        })
        .transpose()?;

      let source = &operation.operation.definition.source_key;
      let owner = source.owner().clone();
      let kind = source.kind();
      let (receiver, target_kind) = operation_semantics(
        operation.operation.id,
        &owner,
        kind,
        &types,
        &mut callback_method_ids,
      )?;
      let callbacks = bridge
        .callbacks()
        .iter()
        .filter(|use_site| use_site.operation_id == operation.operation.id)
        .map(callback_use_site)
        .collect::<Vec<_>>();
      let streams = bridge
        .streams()
        .iter()
        .filter(|use_site| use_site.operation_id == operation.operation.id)
        .map(stream_use_site)
        .collect::<Vec<_>>();

      add_runtime_entrypoints(
        &operation.required_capabilities,
        target_kind,
        signature.async_kind,
        &callbacks,
        &streams,
        &mut entrypoints,
      );
      operations.push(FamilyOperation {
        id: operation.operation.id,
        owner,
        kind,
        async_kind: signature.async_kind,
        receiver,
        arguments,
        return_value,
        declared_error,
        callbacks,
        streams,
        target: target_kind,
      });
    }

    Ok(Self {
      flavor,
      hooks: flavor.hooks(),
      operations,
      runtime_entrypoints: entrypoints,
    })
  }

  pub const fn flavor(&self) -> HostFlavor {
    self.flavor
  }

  pub const fn hooks(&self) -> HostHooks {
    self.hooks
  }

  pub fn operations(&self) -> &[FamilyOperation] {
    &self.operations
  }

  pub fn runtime_entrypoints(&self) -> impl Iterator<Item = RuntimeEntrypoint> + '_ {
    self.runtime_entrypoints.iter().copied()
  }
}

fn add_runtime_entrypoints(
  required: &CapabilitySet,
  target: FamilyOperationTarget,
  async_kind: AsyncKind,
  callbacks: &[FamilyCallbackUseSite],
  streams: &[FamilyStreamUseSite],
  entrypoints: &mut BTreeSet<RuntimeEntrypoint>,
) {
  if required.contains(Capability::ObjectLease) {
    entrypoints.insert(RuntimeEntrypoint::ReleaseObject);
  }
  for callback in callbacks {
    use uniffi_js_engine_schema::CallbackRetention;
    if callback.contract.retention == CallbackRetention::Retained {
      entrypoints.extend([
        RuntimeEntrypoint::RetainCallback,
        RuntimeEntrypoint::ReleaseCallback,
      ]);
    }
  }
  // Callback method dispatch is derived from the method operation signature,
  // never from a callback argument use-site.  One callback interface may mix
  // sync and async methods, so each host operation contributes its own entrypoint.
  if matches!(target, FamilyOperationTarget::CallbackHost(_)) {
    entrypoints.insert(match async_kind {
      AsyncKind::Sync => RuntimeEntrypoint::InvokeCallbackSync,
      AsyncKind::Async => RuntimeEntrypoint::InvokeCallbackAsync,
    });
  }
  for stream in streams {
    match stream.contract.direction {
      StreamDirection::Input => entrypoints.extend([
        RuntimeEntrypoint::PullInputStream,
        RuntimeEntrypoint::CancelInputStream,
        RuntimeEntrypoint::ReleaseInputStream,
      ]),
      StreamDirection::Output => entrypoints.extend([
        RuntimeEntrypoint::NextOutputStream,
        RuntimeEntrypoint::CancelOutputStream,
        RuntimeEntrypoint::ReleaseOutputStream,
      ]),
    }
  }
}

fn callback_use_site(use_site: &CallbackUseSite) -> FamilyCallbackUseSite {
  FamilyCallbackUseSite {
    callback_type: use_site.callback_type,
    path: use_site.path.clone(),
    contract: use_site.contract,
  }
}

fn stream_use_site(use_site: &StreamUseSite) -> FamilyStreamUseSite {
  FamilyStreamUseSite {
    path: use_site.path.clone(),
    contract: use_site.contract,
  }
}

fn operation_semantics(
  operation_id: OperationId,
  owner: &OperationOwner,
  kind: OperationKind,
  types: &BTreeMap<&TypeSourceKey, (TypeId, &NamedTypeKind)>,
  callback_method_ids: &mut BTreeMap<TypeId, u32>,
) -> Result<(Option<ObjectReceiver>, FamilyOperationTarget), FamilyPlanError> {
  match (owner, kind) {
    (OperationOwner::Namespace, OperationKind::Function) => {
      Ok((None, FamilyOperationTarget::Native))
    }
    (OperationOwner::Object(key), OperationKind::Constructor) => {
      require_owner_kind(operation_id, key, types, "object", NamedTypeKind::Object)?;
      Ok((None, FamilyOperationTarget::Native))
    }
    (OperationOwner::Object(key), OperationKind::Method) => {
      let object_type =
        require_owner_kind(operation_id, key, types, "object", NamedTypeKind::Object)?;
      Ok((
        Some(ObjectReceiver {
          object_type,
          ownership: Ownership::Borrowed,
        }),
        FamilyOperationTarget::Native,
      ))
    }
    (OperationOwner::Callback(key), OperationKind::CallbackMethod) => {
      let callback_type = require_owner_kind(
        operation_id,
        key,
        types,
        "callback",
        NamedTypeKind::Callback,
      )?;
      let method_id = callback_method_ids.entry(callback_type).or_default();
      let result = CallbackMethod {
        callback_type,
        method_id: *method_id,
      };
      *method_id = method_id
        .checked_add(1)
        .ok_or(FamilyPlanError::TooManyCallbackMethods { callback_type })?;
      Ok((None, FamilyOperationTarget::CallbackHost(result)))
    }
    (_, OperationKind::InputStreamPull) => Ok((None, FamilyOperationTarget::InputStreamHostPull)),
    (_, OperationKind::InputStreamCancel) => {
      Ok((None, FamilyOperationTarget::InputStreamHostCancel))
    }
    (OperationOwner::Object(key), OperationKind::OutputStreamNext)
    | (OperationOwner::Object(key), OperationKind::OutputStreamCancel) => {
      let object_type =
        require_owner_kind(operation_id, key, types, "object", NamedTypeKind::Object)?;
      Ok((
        Some(ObjectReceiver {
          object_type,
          ownership: Ownership::Borrowed,
        }),
        FamilyOperationTarget::Native,
      ))
    }
    (_, OperationKind::OutputStreamStart) => Ok((None, FamilyOperationTarget::Native)),
    _ => Err(FamilyPlanError::UnsupportedOperationShape {
      operation_id,
      owner: owner.clone(),
      kind,
    }),
  }
}

fn require_owner_kind(
  operation_id: OperationId,
  key: &TypeSourceKey,
  types: &BTreeMap<&TypeSourceKey, (TypeId, &NamedTypeKind)>,
  expected: &'static str,
  expected_kind: NamedTypeKind,
) -> Result<TypeId, FamilyPlanError> {
  let (id, kind) = named_type(key, types, "operation owner")?;
  if std::mem::discriminant(kind) == std::mem::discriminant(&expected_kind) {
    Ok(id)
  } else {
    Err(FamilyPlanError::WrongOperationOwnerKind {
      operation_id,
      key: key.clone(),
      expected,
    })
  }
}

fn carrier_for(
  value: &ValueType,
  types: &BTreeMap<&TypeSourceKey, (TypeId, &NamedTypeKind)>,
) -> Result<CarrierRecipe, FamilyPlanError> {
  Ok(match value {
    ValueType::Scalar(scalar) => match scalar {
      ScalarType::Bool => CarrierRecipe::Boolean,
      ScalarType::I8 => CarrierRecipe::Number(NumberCarrier::I8),
      ScalarType::U8 => CarrierRecipe::Number(NumberCarrier::U8),
      ScalarType::I16 => CarrierRecipe::Number(NumberCarrier::I16),
      ScalarType::U16 => CarrierRecipe::Number(NumberCarrier::U16),
      ScalarType::I32 => CarrierRecipe::Number(NumberCarrier::I32),
      ScalarType::U32 => CarrierRecipe::Number(NumberCarrier::U32),
      ScalarType::I64 => CarrierRecipe::BigInt(BigIntCarrier::I64),
      ScalarType::U64 => CarrierRecipe::BigInt(BigIntCarrier::U64),
      ScalarType::F32 => CarrierRecipe::Number(NumberCarrier::F32),
      ScalarType::F64 => CarrierRecipe::Number(NumberCarrier::F64),
      ScalarType::String => CarrierRecipe::String,
      ScalarType::Bytes => CarrierRecipe::Uint8Array,
    },
    ValueType::Named(key) => {
      let (id, kind) = named_type(key, types, "value")?;
      match kind {
        NamedTypeKind::Record { .. } => CarrierRecipe::Record(id),
        NamedTypeKind::Enum { .. } => CarrierRecipe::Enum(id),
        NamedTypeKind::Error { .. } => CarrierRecipe::ErrorDescriptor(id),
        NamedTypeKind::Object => CarrierRecipe::ObjectLease(id),
        NamedTypeKind::Callback => CarrierRecipe::Callback(id),
      }
    }
    ValueType::Optional(inner) => CarrierRecipe::Optional(Box::new(carrier_for(inner, types)?)),
    ValueType::Sequence(inner) => CarrierRecipe::Sequence(Box::new(carrier_for(inner, types)?)),
    ValueType::Map(key, value) => CarrierRecipe::Map(
      Box::new(carrier_for(key, types)?),
      Box::new(carrier_for(value, types)?),
    ),
    ValueType::Set(inner) => CarrierRecipe::Set(Box::new(carrier_for(inner, types)?)),
    ValueType::InputStream(inner) => {
      CarrierRecipe::InputStream(Box::new(carrier_for(inner, types)?))
    }
    ValueType::OutputStream(inner) => {
      CarrierRecipe::OutputStream(Box::new(carrier_for(inner, types)?))
    }
  })
}

fn named_type<'a>(
  key: &TypeSourceKey,
  types: &'a BTreeMap<&TypeSourceKey, (TypeId, &'a NamedTypeKind)>,
  role: &'static str,
) -> Result<(TypeId, &'a NamedTypeKind), FamilyPlanError> {
  types
    .get(key)
    .copied()
    .ok_or_else(|| FamilyPlanError::UnknownNamedType {
      role,
      key: key.clone(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FamilyPlanError {
  MissingFlavorTarget {
    flavor: HostFlavor,
  },
  UnsupportedFlavorCapability {
    flavor: HostFlavor,
    capability: Capability,
  },
  NonDenseOperationId {
    expected: u32,
    actual: u32,
  },
  TooManyOperations,
  UnknownNamedType {
    role: &'static str,
    key: TypeSourceKey,
  },
  WrongNamedTypeKind {
    key: TypeSourceKey,
    expected: &'static str,
  },
  WrongOperationOwnerKind {
    operation_id: OperationId,
    key: TypeSourceKey,
    expected: &'static str,
  },
  UnsupportedOperationShape {
    operation_id: OperationId,
    owner: OperationOwner,
    kind: OperationKind,
  },
  TooManyCallbackMethods {
    callback_type: TypeId,
  },
}

impl fmt::Display for FamilyPlanError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::MissingFlavorTarget { flavor } => {
        write!(formatter, "bridge plan has no {flavor:?} target")
      }
      Self::UnsupportedFlavorCapability { flavor, capability } => write!(
        formatter,
        "{flavor:?} N-API hooks do not support requested {capability:?}"
      ),
      Self::NonDenseOperationId { expected, actual } => write!(
        formatter,
        "N-API operation table is not dense: expected {expected}, found {actual}"
      ),
      Self::TooManyOperations => formatter.write_str("N-API operation table exceeds u32"),
      Self::UnknownNamedType { role, key } => {
        write!(formatter, "{role} references unknown named type {key}")
      }
      Self::WrongNamedTypeKind { key, expected } => {
        write!(formatter, "named type {key} is not a declared {expected}")
      }
      Self::WrongOperationOwnerKind {
        operation_id,
        key,
        expected,
      } => write!(
        formatter,
        "operation {operation_id} owner {key} is not a declared {expected}"
      ),
      Self::UnsupportedOperationShape {
        operation_id,
        owner,
        kind,
      } => write!(
        formatter,
        "operation {operation_id} has unsupported owner/kind combination {owner}/{kind:?}"
      ),
      Self::TooManyCallbackMethods { callback_type } => write!(
        formatter,
        "callback type {callback_type} has more than u32::MAX methods"
      ),
    }
  }
}

impl Error for FamilyPlanError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BigIntLossError {
  LossySigned,
  LossyUnsigned,
  NegativeUnsigned,
}

impl fmt::Display for BigIntLossError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::LossySigned => formatter.write_str("BigInt is outside the lossless i64 range"),
      Self::LossyUnsigned => formatter.write_str("BigInt is outside the lossless u64 range"),
      Self::NegativeUnsigned => formatter.write_str("negative BigInt is not valid for u64"),
    }
  }
}

impl Error for BigIntLossError {}

pub fn require_lossless_i64(value: i64, lossless: bool) -> Result<i64, BigIntLossError> {
  if lossless {
    Ok(value)
  } else {
    Err(BigIntLossError::LossySigned)
  }
}

pub fn require_lossless_u64(
  negative: bool,
  value: u64,
  lossless: bool,
) -> Result<u64, BigIntLossError> {
  if negative {
    Err(BigIntLossError::NegativeUnsigned)
  } else if !lossless {
    Err(BigIntLossError::LossyUnsigned)
  } else {
    Ok(value)
  }
}

/// Engine-independent words for creating an N-API BigInt result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BigIntWords {
  pub negative: bool,
  pub words: Vec<u64>,
}

impl From<i64> for BigIntWords {
  fn from(value: i64) -> Self {
    Self {
      negative: value.is_negative(),
      words: vec![value.unsigned_abs()],
    }
  }
}

impl From<u64> for BigIntWords {
  fn from(value: u64) -> Self {
    Self {
      negative: false,
      words: vec![value],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn bigint_boundaries_are_lossless_and_signed() {
    assert_eq!(require_lossless_i64(i64::MIN, true), Ok(i64::MIN));
    assert_eq!(require_lossless_i64(i64::MAX, true), Ok(i64::MAX));
    assert_eq!(
      require_lossless_i64(0, false),
      Err(BigIntLossError::LossySigned)
    );
    assert_eq!(require_lossless_u64(false, u64::MAX, true), Ok(u64::MAX));
    assert_eq!(
      require_lossless_u64(true, 1, true),
      Err(BigIntLossError::NegativeUnsigned)
    );
    assert_eq!(BigIntWords::from(i64::MIN).words, vec![1_u64 << 63]);
    assert!(BigIntWords::from(i64::MIN).negative);
    assert_eq!(BigIntWords::from(u64::MAX).words, vec![u64::MAX]);
  }

  #[test]
  fn node_and_ohos_differences_are_explicit_hooks() {
    assert_eq!(
      HostFlavor::Node.hooks().callback_dispatch,
      CallbackDispatch::ThreadsafeFunction
    );
    assert_eq!(
      HostFlavor::Ohos.hooks().callback_dispatch,
      CallbackDispatch::ArkPriorityThreadsafeFunction
    );
    assert!(HostFlavor::Node
      .capabilities()
      .supported
      .contains(Capability::CallbackReentrancy));
    assert!(HostFlavor::Ohos
      .capabilities()
      .supported
      .contains(Capability::CallbackReentrancy));
  }

  #[test]
  fn ohos_family_plan_accepts_allowed_callback_reentrancy() {
    use uniffi_js_abi::{
      ArgumentDefinition, AsyncKind, ComponentDefinition, ComponentId, ComponentKey,
      IdentifiedComponent, IdentifiedOperation, IdentifiedType, NamedTypeKind, OperationDefinition,
      OperationId, OperationKind, OperationOwner, OperationSignature, OperationSourceKey,
      Ownership, TypeDefinition, TypeId, TypeSourceKey, ValueType,
    };
    use uniffi_js_engine_schema::{
      BridgePlanInput, CallbackContract, CallbackReentrancy, CallbackRetention, CallbackThreading,
      CallbackUseSite, PlannedOperation,
    };

    let component = ComponentKey::new("ohos-callback-fixture").unwrap();
    let callback_key = TypeSourceKey::new(component.clone(), "Observer").unwrap();
    let observe = IdentifiedOperation {
      id: OperationId::new(0),
      definition: OperationDefinition::new(
        OperationSourceKey::new(
          component.clone(),
          OperationOwner::Namespace,
          OperationKind::Function,
          "observe",
        )
        .unwrap(),
        "observe",
        "ohos_callback_fixture::observe",
        "ohos_callback_fixture_private_0",
        OperationSignature {
          arguments: vec![ArgumentDefinition::new(
            "observer",
            ValueType::Named(callback_key.clone()),
            Ownership::Owned,
          )
          .unwrap()],
          return_type: None,
          async_kind: AsyncKind::Sync,
          throws: None,
        },
      )
      .unwrap(),
    };
    let callback_method = IdentifiedOperation {
      id: OperationId::new(1),
      definition: OperationDefinition::new(
        OperationSourceKey::new(
          component.clone(),
          OperationOwner::Callback(callback_key.clone()),
          OperationKind::CallbackMethod,
          "onEvent",
        )
        .unwrap(),
        "onEvent",
        "ohos_callback_fixture::Observer::onEvent",
        "ohos_callback_fixture_private_1",
        OperationSignature {
          arguments: Vec::new(),
          return_type: None,
          async_kind: AsyncKind::Sync,
          throws: None,
        },
      )
      .unwrap(),
    };
    let bridge = BridgePlan::build(BridgePlanInput {
      components: vec![IdentifiedComponent {
        id: ComponentId::new(0),
        definition: ComponentDefinition::new(component, "ohos-callback-fixture").unwrap(),
      }],
      types: vec![IdentifiedType {
        id: TypeId::new(0),
        definition: TypeDefinition::new(callback_key, "Observer", NamedTypeKind::Callback).unwrap(),
      }],
      operations: vec![
        PlannedOperation::new(observe),
        PlannedOperation::new(callback_method),
      ],
      callbacks: vec![CallbackUseSite {
        operation_id: OperationId::new(0),
        callback_type: TypeId::new(0),
        path: ValuePath::argument(0),
        contract: CallbackContract {
          retention: CallbackRetention::Scoped,
          threading: CallbackThreading::CallingThread,
          reentrancy: CallbackReentrancy::Allowed,
        },
      }],
      streams: Vec::new(),
      targets: vec![HostFlavor::Ohos.capabilities()],
    })
    .unwrap();

    let family = FamilyPlan::build(&bridge, HostFlavor::Ohos).unwrap();
    assert_eq!(family.operations()[0].callbacks.len(), 1);
    assert_eq!(
      family.operations()[0].callbacks[0].contract.reentrancy,
      CallbackReentrancy::Allowed
    );
  }

  #[test]
  fn carrier_recipes_keep_bigint_and_bytes_canonical() {
    let types = BTreeMap::new();
    assert_eq!(
      carrier_for(&ValueType::Scalar(ScalarType::I64), &types).unwrap(),
      CarrierRecipe::BigInt(BigIntCarrier::I64)
    );
    assert_eq!(
      carrier_for(&ValueType::Scalar(ScalarType::U64), &types).unwrap(),
      CarrierRecipe::BigInt(BigIntCarrier::U64)
    );
    assert_eq!(
      carrier_for(&ValueType::Scalar(ScalarType::Bytes), &types).unwrap(),
      CarrierRecipe::Uint8Array
    );
    assert_eq!(
      carrier_for(
        &ValueType::Map(
          Box::new(ValueType::Scalar(ScalarType::String)),
          Box::new(ValueType::Set(Box::new(
            ValueType::Scalar(ScalarType::U32,)
          ))),
        ),
        &types,
      )
      .unwrap(),
      CarrierRecipe::Map(
        Box::new(CarrierRecipe::String),
        Box::new(CarrierRecipe::Set(Box::new(CarrierRecipe::Number(
          NumberCarrier::U32,
        )))),
      )
    );
  }

  #[test]
  fn named_carriers_keep_semantic_roles() {
    let component = uniffi_js_abi::ComponentKey::new("fixture").unwrap();
    let record_key = TypeSourceKey::new(component.clone(), "Record").unwrap();
    let enum_key = TypeSourceKey::new(component.clone(), "Enum").unwrap();
    let error_key = TypeSourceKey::new(component.clone(), "Error").unwrap();
    let object_key = TypeSourceKey::new(component.clone(), "Object").unwrap();
    let callback_key = TypeSourceKey::new(component, "Callback").unwrap();
    let record = NamedTypeKind::Record { fields: Vec::new() };
    let enumeration = NamedTypeKind::Enum {
      variants: Vec::new(),
    };
    let error = NamedTypeKind::Error {
      variants: Vec::new(),
    };
    let object = NamedTypeKind::Object;
    let callback = NamedTypeKind::Callback;
    let types = BTreeMap::from([
      (&record_key, (TypeId::new(0), &record)),
      (&enum_key, (TypeId::new(1), &enumeration)),
      (&error_key, (TypeId::new(2), &error)),
      (&object_key, (TypeId::new(3), &object)),
      (&callback_key, (TypeId::new(4), &callback)),
    ]);

    assert_eq!(
      carrier_for(&ValueType::Named(record_key.clone()), &types).unwrap(),
      CarrierRecipe::Record(TypeId::new(0))
    );
    assert_eq!(
      carrier_for(&ValueType::Named(enum_key.clone()), &types).unwrap(),
      CarrierRecipe::Enum(TypeId::new(1))
    );
    assert_eq!(
      carrier_for(&ValueType::Named(error_key.clone()), &types).unwrap(),
      CarrierRecipe::ErrorDescriptor(TypeId::new(2))
    );
    assert_eq!(
      carrier_for(&ValueType::Named(object_key.clone()), &types).unwrap(),
      CarrierRecipe::ObjectLease(TypeId::new(3))
    );
    assert_eq!(
      carrier_for(&ValueType::Named(callback_key.clone()), &types).unwrap(),
      CarrierRecipe::Callback(TypeId::new(4))
    );
  }
}
