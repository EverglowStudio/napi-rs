//! Engine-owned planning shared by the Node and OHOS N-API backends.
//!
//! This crate intentionally has no dependency on UniFFI.  The UniFFI frontend
//! projects its already-normalized operation table into the small mechanical
//! DTOs below.  The family planner validates only dispatch slots, value paths,
//! and resource lifecycle relationships; names, type graphs, and capability
//! policy remain owned by the UniFFI frontend.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// The two hosts that share the N-API family lowering.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HostFlavor {
  Node,
  Ohos,
}

/// Action to take when a session close grace period expires.
///
/// The canonical plan currently defines only detach.  Keeping this as an
/// engine-owned enum makes the policy mechanically carry the canonical action
/// without exposing another runtime configuration surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeadlineAction {
  Detach,
}

/// Immutable close policy projected by the UniFFI frontend.
///
/// There is deliberately no default or parser in the N-API family crate.  The
/// frontend owns the single default and supplies this value as part of every
/// [`FamilyPlanInput`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClosePolicy {
  pub grace_ms: u32,
  pub on_deadline: DeadlineAction,
}

impl HostFlavor {
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

/// Operation async/fallibility data projected by the UniFFI frontend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AsyncKind {
  Sync,
  Async,
}

/// Mechanical operation shape.  This is not a public naming or type model.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OperationKind {
  Function,
  Constructor,
  Method,
  CallbackMethod,
  InputStreamPull,
  InputStreamCancel,
  OutputStreamStart,
  OutputStreamNext,
  OutputStreamCancel,
}

/// Resource ownership is needed by the N-API lease lowering, but carries no
/// UniFFI type information.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceOwnership {
  Owned,
  Borrowed,
  ByArc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
  Object,
  InputStream,
  OutputStream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceBinding {
  pub kind: ResourceKind,
  pub ownership: ResourceOwnership,
}

/// Engine-owned receiver classification.  A method receiver may be an
/// ordinary value (for example a record or enum value) or an N-API resource
/// lease.  Keeping the distinction explicit prevents value receivers from
/// accidentally entering the resource retain/release machinery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiverBinding {
  Value,
  Resource(ResourceBinding),
}

/// Dispatch target.  Callback method IDs are supplied by the canonical
/// frontend; the family planner never derives or persists them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationDispatch {
  Native,
  CallbackHost {
    callback_type_id: u32,
    method_id: u32,
  },
  InputStreamHostPull,
  InputStreamHostCancel,
}

/// Mechanical path segments used by callback and stream use sites.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ValuePathSegment {
  Argument(u32),
  Return,
  /// Select the payload of a canonical output-stream `item` step.
  ///
  /// This selector is only valid immediately below a `Return` root on an
  /// `OutputStreamNext` operation.  Keeping the step branch in the path
  /// model lets the session use the same resource walker for object-bearing
  /// stream values as for ordinary operation returns.
  StreamItem,
  /// Select the payload of a canonical output-stream `error` step.
  ///
  /// This selector is only valid immediately below a `Return` root on an
  /// `OutputStreamNext` operation.
  StreamError,
  Field(String),
  Variant(String),
  Optional,
  SequenceElement,
  MapKey,
  MapValue,
  SetElement,
}

/// A mechanical carrier class projected by the UniFFI engine adapter.  It is
/// deliberately smaller than the public type graph: engines only need to
/// know which local ABI carrier and conversion recipe to use.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CarrierKind {
  Primitive,
  BigInt,
  Bytes,
  Timestamp,
  Duration,
  LocalAdapter,
  OpaqueHandle,
  CallbackProxy,
  InputStream,
  OutputStream,
  StreamStep,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConversionRecipe {
  Identity,
  Optional(Box<Self>),
  Sequence(Box<Self>),
  Map(Box<Self>, Box<Self>),
  Set(Box<Self>),
  Record(u32),
  Enum(u32),
  Error(u32),
  Object(u32),
  Custom(u32, Box<Self>),
  Callback(u32),
  InputStream(Box<Self>),
  OutputStream(Box<Self>),
  StreamStep { item: Box<Self>, error: Box<Self> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamValueBinding {
  pub carrier: CarrierKind,
  pub conversion: ConversionRecipe,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ValuePath {
  segments: Vec<ValuePathSegment>,
}

impl ValuePath {
  pub fn new(segments: impl Into<Vec<ValuePathSegment>>) -> Self {
    Self {
      segments: segments.into(),
    }
  }

  pub fn argument(index: u32) -> Self {
    Self::new(vec![ValuePathSegment::Argument(index)])
  }

  pub fn return_value() -> Self {
    Self::new(vec![ValuePathSegment::Return])
  }

  pub fn segments(&self) -> &[ValuePathSegment] {
    &self.segments
  }
}

impl fmt::Display for ValuePath {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (index, segment) in self.segments.iter().enumerate() {
      if index != 0 {
        formatter.write_str(".")?;
      }
      match segment {
        ValuePathSegment::Argument(index) => write!(formatter, "argument[{index}]")?,
        ValuePathSegment::Return => formatter.write_str("return")?,
        ValuePathSegment::StreamItem => formatter.write_str("stream-item")?,
        ValuePathSegment::StreamError => formatter.write_str("stream-error")?,
        ValuePathSegment::Field(name) => write!(formatter, "field[{name}]")?,
        ValuePathSegment::Variant(name) => write!(formatter, "variant[{name}]")?,
        ValuePathSegment::Optional => formatter.write_str("optional")?,
        ValuePathSegment::SequenceElement => formatter.write_str("sequence")?,
        ValuePathSegment::MapKey => formatter.write_str("map-key")?,
        ValuePathSegment::MapValue => formatter.write_str("map-value")?,
        ValuePathSegment::SetElement => formatter.write_str("set")?,
      }
    }
    Ok(())
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallbackRetention {
  Scoped,
  Retained,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallbackThreading {
  CallingThread,
  MayCrossThread,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallbackReentrancy {
  Allowed,
  Forbidden,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallbackContract {
  pub retention: CallbackRetention,
  pub threading: CallbackThreading,
  pub reentrancy: CallbackReentrancy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallbackUseSite {
  pub operation_id: u32,
  pub callback_type_id: u32,
  pub path: ValuePath,
  pub contract: CallbackContract,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamDirection {
  Input,
  Output,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamUseSite {
  pub operation_id: u32,
  pub use_site_id: u32,
  pub path: ValuePath,
  pub direction: StreamDirection,
  pub item: StreamValueBinding,
  pub error: StreamValueBinding,
  pub is_send: bool,
  pub slots: Vec<StreamSlotIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamSlotIdentity {
  pub use_site_id: u32,
  pub operation_id: u32,
  pub kind: OperationKind,
}

/// A resource produced at a concrete use-site in an operation's return
/// value.  Return resources are deliberately represented as a list rather
/// than a single top-level binding: the returned value may contain resources
/// below optional, record/enum, sequence, set, or map selectors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResultResourceUseSite {
  pub operation_id: u32,
  pub path: ValuePath,
  pub binding: ResourceBinding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyOperationInput {
  pub id: u32,
  pub kind: OperationKind,
  pub async_kind: AsyncKind,
  pub fallible: bool,
  pub argument_count: usize,
  pub dispatch: OperationDispatch,
  pub receiver: Option<ReceiverBinding>,
  pub result_resources: Vec<ResultResourceUseSite>,
  pub callbacks: Vec<CallbackUseSite>,
  pub streams: Vec<StreamUseSite>,
  /// Synthetic stream slots carry their canonical use-site/slot identity so
  /// the family backend never derives a slot from an operation name.
  pub stream_slot: Option<StreamSlotIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyPlanInput {
  pub flavor: HostFlavor,
  pub close_policy: ClosePolicy,
  pub operations: Vec<FamilyOperationInput>,
}

/// Runtime entrypoints required by a family plan.
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FamilyOperationTarget {
  Native,
  CallbackHost {
    callback_type_id: u32,
    method_id: u32,
  },
  InputStreamHostPull,
  InputStreamHostCancel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyOperation {
  pub id: u32,
  pub kind: OperationKind,
  pub async_kind: AsyncKind,
  pub fallible: bool,
  pub argument_count: usize,
  pub receiver: Option<ReceiverBinding>,
  pub result_resources: Vec<ResultResourceUseSite>,
  pub callbacks: Vec<CallbackUseSite>,
  pub streams: Vec<StreamUseSite>,
  pub stream_slot: Option<StreamSlotIdentity>,
  pub target: FamilyOperationTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyPlan {
  flavor: HostFlavor,
  hooks: HostHooks,
  close_policy: ClosePolicy,
  operations: Vec<FamilyOperation>,
  runtime_entrypoints: BTreeSet<RuntimeEntrypoint>,
}

impl FamilyPlan {
  pub fn build(input: FamilyPlanInput) -> Result<Self, FamilyPlanError> {
    validate_close_policy(input.close_policy)?;
    let mut operations = input.operations;
    let mut seen_ids = BTreeSet::new();
    for operation in &operations {
      if !seen_ids.insert(operation.id) {
        return Err(FamilyPlanError::DuplicateOperationId { id: operation.id });
      }
    }
    operations.sort_by_key(|operation| operation.id);
    let operation_ids = operations
      .iter()
      .map(|operation| operation.id)
      .collect::<BTreeSet<_>>();
    let mut stream_use_site_ids = BTreeSet::new();
    let mut declared_stream_slots = BTreeSet::new();
    let mut referenced_stream_slots = BTreeSet::new();
    for (expected, operation) in operations.iter().enumerate() {
      let expected = u32::try_from(expected).map_err(|_| FamilyPlanError::TooManyOperations)?;
      if operation.id != expected {
        return Err(FamilyPlanError::NonDenseOperationId {
          expected,
          actual: operation.id,
        });
      }
      validate_operation_shape(operation)?;
      if let Some(slot) = &operation.stream_slot {
        if slot.operation_id != operation.id {
          return Err(FamilyPlanError::StreamSlotOperationMismatch {
            expected: operation.id,
            actual: slot.operation_id,
          });
        }
        if slot.kind != operation.kind {
          return Err(FamilyPlanError::StreamSlotKindMismatch {
            operation_id: operation.id,
            expected: operation.kind,
            actual: slot.kind,
          });
        }
        if !matches!(
          slot.kind,
          OperationKind::InputStreamPull
            | OperationKind::InputStreamCancel
            | OperationKind::OutputStreamStart
            | OperationKind::OutputStreamNext
            | OperationKind::OutputStreamCancel
        ) {
          return Err(FamilyPlanError::InvalidStreamSlot { id: operation.id });
        }
        if !declared_stream_slots.insert((slot.use_site_id, slot.operation_id, slot.kind)) {
          return Err(FamilyPlanError::DuplicateStreamSlot {
            use_site_id: slot.use_site_id,
            kind: slot.kind,
          });
        }
      }
      for callback in &operation.callbacks {
        if callback.operation_id != operation.id {
          return Err(FamilyPlanError::UseSiteOperationMismatch {
            expected: operation.id,
            actual: callback.operation_id,
            role: "callback",
          });
        }
        validate_path(operation, &callback.path, "callback")?;
      }
      let mut result_paths = BTreeSet::new();
      for result_resource in &operation.result_resources {
        if result_resource.operation_id != operation.id {
          return Err(FamilyPlanError::UseSiteOperationMismatch {
            expected: operation.id,
            actual: result_resource.operation_id,
            role: "result resource",
          });
        }
        validate_result_resource_path(operation, &result_resource.path)?;
        if result_resource.binding.ownership != ResourceOwnership::Owned {
          return Err(FamilyPlanError::NonOwnedResultResource {
            id: operation.id,
            ownership: result_resource.binding.ownership,
          });
        }
        let path = result_resource.path.to_string();
        if !result_paths.insert(path.clone()) {
          if let Some(previous) = operation
            .result_resources
            .iter()
            .find(|candidate| candidate.path.to_string() == path)
          {
            if previous.binding == result_resource.binding {
              return Err(FamilyPlanError::DuplicateResultResourceUseSite {
                id: operation.id,
                path,
              });
            }
          }
          return Err(FamilyPlanError::ConflictingResultResourceUseSite {
            id: operation.id,
            path,
          });
        }
      }
      for stream in &operation.streams {
        if stream.operation_id != operation.id {
          return Err(FamilyPlanError::UseSiteOperationMismatch {
            expected: operation.id,
            actual: stream.operation_id,
            role: "stream",
          });
        }
        validate_path(operation, &stream.path, "stream")?;
        if !stream_use_site_ids.insert(stream.use_site_id) {
          return Err(FamilyPlanError::DuplicateStreamUseSite {
            use_site_id: stream.use_site_id,
          });
        }
        let expected_kinds = match stream.direction {
          StreamDirection::Input => vec![
            OperationKind::InputStreamPull,
            OperationKind::InputStreamCancel,
          ],
          StreamDirection::Output => vec![
            OperationKind::OutputStreamStart,
            OperationKind::OutputStreamNext,
            OperationKind::OutputStreamCancel,
          ],
        };
        if stream.slots.len() != expected_kinds.len()
          || expected_kinds.iter().any(|kind| {
            stream
              .slots
              .iter()
              .filter(|slot| slot.kind == *kind)
              .count()
              != 1
          })
        {
          return Err(FamilyPlanError::InvalidStreamSlot { id: operation.id });
        }
        for slot in &stream.slots {
          if slot.use_site_id != stream.use_site_id {
            return Err(FamilyPlanError::StreamSlotUseSiteMismatch {
              expected: stream.use_site_id,
              actual: slot.use_site_id,
            });
          }
          if !operation_ids.contains(&slot.operation_id) {
            return Err(FamilyPlanError::UnknownStreamSlotOperation {
              use_site_id: stream.use_site_id,
              operation_id: slot.operation_id,
            });
          }
          if !referenced_stream_slots.insert((slot.use_site_id, slot.operation_id, slot.kind)) {
            return Err(FamilyPlanError::DuplicateStreamSlot {
              use_site_id: slot.use_site_id,
              kind: slot.kind,
            });
          }
          let Some(slot_operation) = operations
            .iter()
            .find(|candidate| candidate.id == slot.operation_id)
          else {
            unreachable!("operation ID was checked above")
          };
          if slot_operation.kind != slot.kind {
            return Err(FamilyPlanError::StreamSlotKindMismatch {
              operation_id: slot.operation_id,
              expected: slot_operation.kind,
              actual: slot.kind,
            });
          }
          if let Some(declared_slot) = slot_operation.stream_slot.as_ref() {
            if declared_slot.kind != slot.kind {
              return Err(FamilyPlanError::StreamSlotKindMismatch {
                operation_id: slot.operation_id,
                expected: slot_operation.kind,
                actual: declared_slot.kind,
              });
            }
          }
          if slot_operation.stream_slot.as_ref() != Some(slot) {
            return Err(FamilyPlanError::StreamSlotIdentityMismatch {
              use_site_id: stream.use_site_id,
              operation_id: slot.operation_id,
            });
          }
          if stream.direction == StreamDirection::Output
            && slot.kind == OperationKind::OutputStreamStart
            && slot.operation_id != operation.id
          {
            return Err(FamilyPlanError::InvalidStreamSlot { id: operation.id });
          }
          let direction_ok = match stream.direction {
            StreamDirection::Input => {
              matches!(
                slot.kind,
                OperationKind::InputStreamPull | OperationKind::InputStreamCancel
              )
            }
            StreamDirection::Output => matches!(
              slot.kind,
              OperationKind::OutputStreamStart
                | OperationKind::OutputStreamNext
                | OperationKind::OutputStreamCancel
            ),
          };
          if !direction_ok {
            return Err(FamilyPlanError::InvalidStreamSlot { id: operation.id });
          }
        }
      }
    }
    let mut stream_ids = stream_use_site_ids.iter().copied().collect::<Vec<_>>();
    stream_ids.sort_unstable();
    if stream_ids
      .iter()
      .enumerate()
      .any(|(expected, actual)| *actual != expected as u32)
    {
      return Err(FamilyPlanError::NonDenseStreamUseSiteId);
    }
    if let Some((use_site_id, operation_id, kind)) = declared_stream_slots
      .difference(&referenced_stream_slots)
      .next()
      .copied()
    {
      return Err(FamilyPlanError::OrphanStreamSlot {
        use_site_id,
        operation_id,
        kind,
      });
    }

    let mut entrypoints = BTreeSet::from([RuntimeEntrypoint::CloseSession]);
    let family_operations = operations
      .into_iter()
      .map(|operation| {
        let target = match operation.dispatch {
          OperationDispatch::Native => FamilyOperationTarget::Native,
          OperationDispatch::CallbackHost {
            callback_type_id,
            method_id,
          } => FamilyOperationTarget::CallbackHost {
            callback_type_id,
            method_id,
          },
          OperationDispatch::InputStreamHostPull => FamilyOperationTarget::InputStreamHostPull,
          OperationDispatch::InputStreamHostCancel => FamilyOperationTarget::InputStreamHostCancel,
        };
        add_runtime_entrypoints(&operation, target, &mut entrypoints);
        FamilyOperation {
          id: operation.id,
          kind: operation.kind,
          async_kind: operation.async_kind,
          fallible: operation.fallible,
          argument_count: operation.argument_count,
          receiver: operation.receiver,
          result_resources: operation.result_resources,
          callbacks: operation.callbacks,
          streams: operation.streams,
          stream_slot: operation.stream_slot,
          target,
        }
      })
      .collect();

    Ok(Self {
      flavor: input.flavor,
      hooks: input.flavor.hooks(),
      close_policy: input.close_policy,
      operations: family_operations,
      runtime_entrypoints: entrypoints,
    })
  }

  pub const fn flavor(&self) -> HostFlavor {
    self.flavor
  }

  pub const fn hooks(&self) -> HostHooks {
    self.hooks
  }

  pub const fn close_policy(&self) -> ClosePolicy {
    self.close_policy
  }

  pub fn operations(&self) -> &[FamilyOperation] {
    &self.operations
  }

  pub fn runtime_entrypoints(&self) -> impl Iterator<Item = RuntimeEntrypoint> + '_ {
    self.runtime_entrypoints.iter().copied()
  }
}

/// Node's timer API accepts a signed 32-bit millisecond delay.  Keep the
/// policy in the exact range that can be represented without clamping or
/// floating-point conversion surprises; the frontend remains responsible for
/// choosing the value/default.
fn validate_close_policy(policy: ClosePolicy) -> Result<(), FamilyPlanError> {
  if policy.grace_ms > i32::MAX as u32 {
    return Err(FamilyPlanError::ClosePolicyOutOfRange {
      grace_ms: policy.grace_ms,
    });
  }
  if !matches!(policy.on_deadline, DeadlineAction::Detach) {
    return Err(FamilyPlanError::UnsupportedDeadlineAction);
  }
  Ok(())
}

fn validate_operation_shape(operation: &FamilyOperationInput) -> Result<(), FamilyPlanError> {
  let required_receiver = matches!(
    operation.kind,
    OperationKind::Method
      | OperationKind::InputStreamPull
      | OperationKind::InputStreamCancel
      | OperationKind::OutputStreamNext
      | OperationKind::OutputStreamCancel
  );
  if required_receiver && operation.receiver.is_none() {
    return Err(FamilyPlanError::MissingReceiver { id: operation.id });
  }
  if !required_receiver && operation.receiver.is_some() {
    return Err(FamilyPlanError::UnexpectedReceiver { id: operation.id });
  }
  match operation.kind {
    // Methods can target either a normal value receiver or an object
    // resource.  The family plan preserves the category verbatim for the
    // session and Rust bridge; it never infers it from names or operation
    // IDs.
    OperationKind::Method => {
      if matches!(
        operation.receiver,
        Some(ReceiverBinding::Resource(ResourceBinding {
          kind: ResourceKind::InputStream | ResourceKind::OutputStream,
          ..
        }))
      ) {
        return Err(FamilyPlanError::WrongReceiverKind { id: operation.id });
      }
    }
    OperationKind::InputStreamPull | OperationKind::InputStreamCancel => {
      if !matches!(
        operation.receiver,
        Some(ReceiverBinding::Resource(ResourceBinding {
          kind: ResourceKind::InputStream,
          ..
        }))
      ) {
        return Err(FamilyPlanError::WrongReceiverKind { id: operation.id });
      }
    }
    OperationKind::OutputStreamNext | OperationKind::OutputStreamCancel => {
      if !matches!(
        operation.receiver,
        Some(ReceiverBinding::Resource(ResourceBinding {
          kind: ResourceKind::OutputStream,
          ..
        }))
      ) {
        return Err(FamilyPlanError::WrongReceiverKind { id: operation.id });
      }
    }
    _ => {}
  }
  if matches!(operation.kind, OperationKind::OutputStreamStart)
    && !operation.result_resources.iter().any(|resource| {
      matches!(resource.path.segments(), [ValuePathSegment::Return])
        && resource.binding.kind == ResourceKind::OutputStream
    })
  {
    return Err(FamilyPlanError::MissingResultResource { id: operation.id });
  }
  let dispatch_matches_kind = match operation.kind {
    OperationKind::CallbackMethod => {
      matches!(operation.dispatch, OperationDispatch::CallbackHost { .. })
    }
    OperationKind::InputStreamPull => operation.dispatch == OperationDispatch::InputStreamHostPull,
    OperationKind::InputStreamCancel => {
      operation.dispatch == OperationDispatch::InputStreamHostCancel
    }
    OperationKind::Function
    | OperationKind::Constructor
    | OperationKind::Method
    | OperationKind::OutputStreamStart
    | OperationKind::OutputStreamNext
    | OperationKind::OutputStreamCancel => operation.dispatch == OperationDispatch::Native,
  };
  if !dispatch_matches_kind {
    return Err(FamilyPlanError::WrongDispatch { id: operation.id });
  }
  if matches!(
    operation.kind,
    OperationKind::InputStreamCancel | OperationKind::OutputStreamCancel
  ) && !operation.result_resources.is_empty()
  {
    return Err(FamilyPlanError::CancelHasResult { id: operation.id });
  }
  if matches!(
    operation.dispatch,
    OperationDispatch::InputStreamHostPull | OperationDispatch::InputStreamHostCancel
  ) && operation.async_kind != AsyncKind::Async
  {
    return Err(FamilyPlanError::HostOperationMustBeAsync { id: operation.id });
  }
  Ok(())
}

fn validate_result_resource_path(
  operation: &FamilyOperationInput,
  path: &ValuePath,
) -> Result<(), FamilyPlanError> {
  if path.segments().is_empty() {
    return Err(FamilyPlanError::EmptyPath {
      id: operation.id,
      role: "result resource",
    });
  }
  if !matches!(path.segments().first(), Some(ValuePathSegment::Return)) {
    return Err(FamilyPlanError::InvalidPathRoot {
      id: operation.id,
      role: "result resource",
    });
  }
  if path.segments().iter().skip(1).any(|segment| {
    matches!(
      segment,
      ValuePathSegment::Argument(_) | ValuePathSegment::Return
    )
  }) {
    return Err(FamilyPlanError::NestedRootSegment {
      id: operation.id,
      role: "result resource",
    });
  }
  let mut stream_selector = None;
  for (index, segment) in path.segments().iter().skip(1).enumerate() {
    let selector = match segment {
      ValuePathSegment::StreamItem => Some("StreamItem"),
      ValuePathSegment::StreamError => Some("StreamError"),
      _ => None,
    };
    let Some(selector) = selector else {
      continue;
    };
    if operation.kind != OperationKind::OutputStreamNext || index != 0 {
      return Err(FamilyPlanError::InvalidStreamStepPath {
        id: operation.id,
        role: "result resource",
      });
    }
    if stream_selector.replace(selector).is_some() {
      return Err(FamilyPlanError::InvalidStreamStepPath {
        id: operation.id,
        role: "result resource",
      });
    }
  }
  Ok(())
}

fn validate_path(
  operation: &FamilyOperationInput,
  path: &ValuePath,
  role: &'static str,
) -> Result<(), FamilyPlanError> {
  if path.segments().is_empty() {
    return Err(FamilyPlanError::EmptyPath {
      id: operation.id,
      role,
    });
  }
  match path.segments().first() {
    Some(ValuePathSegment::Argument(index)) => {
      if (*index as usize) >= operation.argument_count {
        return Err(FamilyPlanError::ArgumentPathOutOfRange {
          id: operation.id,
          argument: *index,
          count: operation.argument_count,
          role,
        });
      }
    }
    Some(ValuePathSegment::Return) => {}
    Some(_) => {
      return Err(FamilyPlanError::InvalidPathRoot {
        id: operation.id,
        role,
      });
    }
    None => unreachable!("empty path handled above"),
  }
  if path.segments().iter().skip(1).any(|segment| {
    matches!(
      segment,
      ValuePathSegment::Argument(_) | ValuePathSegment::Return
    )
  }) {
    return Err(FamilyPlanError::NestedRootSegment {
      id: operation.id,
      role,
    });
  }
  if path.segments().iter().any(|segment| {
    matches!(
      segment,
      ValuePathSegment::StreamItem | ValuePathSegment::StreamError
    )
  }) {
    return Err(FamilyPlanError::InvalidStreamStepPath {
      id: operation.id,
      role,
    });
  }
  Ok(())
}

fn add_runtime_entrypoints(
  operation: &FamilyOperationInput,
  target: FamilyOperationTarget,
  entrypoints: &mut BTreeSet<RuntimeEntrypoint>,
) {
  if let Some(ReceiverBinding::Resource(resource)) = operation.receiver {
    match resource.kind {
      ResourceKind::Object => {
        entrypoints.insert(RuntimeEntrypoint::ReleaseObject);
      }
      ResourceKind::InputStream => {
        entrypoints.extend([
          RuntimeEntrypoint::PullInputStream,
          RuntimeEntrypoint::CancelInputStream,
          RuntimeEntrypoint::ReleaseInputStream,
        ]);
      }
      ResourceKind::OutputStream => {
        entrypoints.extend([
          RuntimeEntrypoint::NextOutputStream,
          RuntimeEntrypoint::CancelOutputStream,
          RuntimeEntrypoint::ReleaseOutputStream,
        ]);
      }
    }
  }
  for callback in &operation.callbacks {
    if callback.contract.retention == CallbackRetention::Retained {
      entrypoints.extend([
        RuntimeEntrypoint::RetainCallback,
        RuntimeEntrypoint::ReleaseCallback,
      ]);
    }
  }
  if matches!(target, FamilyOperationTarget::CallbackHost { .. }) {
    entrypoints.insert(match operation.async_kind {
      AsyncKind::Sync => RuntimeEntrypoint::InvokeCallbackSync,
      AsyncKind::Async => RuntimeEntrypoint::InvokeCallbackAsync,
    });
  }
  for result in &operation.result_resources {
    match result.binding.kind {
      ResourceKind::Object => {
        entrypoints.insert(RuntimeEntrypoint::ReleaseObject);
      }
      ResourceKind::InputStream => entrypoints.extend([
        RuntimeEntrypoint::PullInputStream,
        RuntimeEntrypoint::CancelInputStream,
        RuntimeEntrypoint::ReleaseInputStream,
      ]),
      ResourceKind::OutputStream => entrypoints.extend([
        RuntimeEntrypoint::NextOutputStream,
        RuntimeEntrypoint::CancelOutputStream,
        RuntimeEntrypoint::ReleaseOutputStream,
      ]),
    }
  }
  for stream in &operation.streams {
    match stream.direction {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FamilyPlanError {
  ClosePolicyOutOfRange {
    grace_ms: u32,
  },
  UnsupportedDeadlineAction,
  DuplicateOperationId {
    id: u32,
  },
  NonDenseOperationId {
    expected: u32,
    actual: u32,
  },
  TooManyOperations,
  MissingReceiver {
    id: u32,
  },
  UnexpectedReceiver {
    id: u32,
  },
  WrongReceiverKind {
    id: u32,
  },
  MissingResultResource {
    id: u32,
  },
  WrongDispatch {
    id: u32,
  },
  HostOperationMustBeAsync {
    id: u32,
  },
  CancelHasResult {
    id: u32,
  },
  NonOwnedResultResource {
    id: u32,
    ownership: ResourceOwnership,
  },
  DuplicateResultResourceUseSite {
    id: u32,
    path: String,
  },
  ConflictingResultResourceUseSite {
    id: u32,
    path: String,
  },
  UseSiteOperationMismatch {
    expected: u32,
    actual: u32,
    role: &'static str,
  },
  EmptyPath {
    id: u32,
    role: &'static str,
  },
  InvalidPathRoot {
    id: u32,
    role: &'static str,
  },
  NestedRootSegment {
    id: u32,
    role: &'static str,
  },
  InvalidStreamStepPath {
    id: u32,
    role: &'static str,
  },
  ArgumentPathOutOfRange {
    id: u32,
    argument: u32,
    count: usize,
    role: &'static str,
  },
  DuplicateStreamUseSite {
    use_site_id: u32,
  },
  NonDenseStreamUseSiteId,
  DuplicateStreamSlot {
    use_site_id: u32,
    kind: OperationKind,
  },
  StreamSlotOperationMismatch {
    expected: u32,
    actual: u32,
  },
  StreamSlotKindMismatch {
    operation_id: u32,
    expected: OperationKind,
    actual: OperationKind,
  },
  StreamSlotUseSiteMismatch {
    expected: u32,
    actual: u32,
  },
  UnknownStreamSlotOperation {
    use_site_id: u32,
    operation_id: u32,
  },
  StreamSlotIdentityMismatch {
    use_site_id: u32,
    operation_id: u32,
  },
  InvalidStreamSlot {
    id: u32,
  },
  OrphanStreamSlot {
    use_site_id: u32,
    operation_id: u32,
    kind: OperationKind,
  },
}

impl fmt::Display for FamilyPlanError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::ClosePolicyOutOfRange { grace_ms } => write!(
        formatter,
        "N-API close policy grace_ms {grace_ms} exceeds the JavaScript timer range"
      ),
      Self::UnsupportedDeadlineAction => {
        formatter.write_str("N-API close policy uses an unsupported deadline action")
      }
      Self::DuplicateOperationId { id } => write!(formatter, "duplicate N-API operation ID {id}"),
      Self::NonDenseOperationId { expected, actual } => {
        write!(
          formatter,
          "N-API operation table is not dense: expected {expected}, found {actual}"
        )
      }
      Self::TooManyOperations => formatter.write_str("N-API operation table exceeds u32"),
      Self::MissingReceiver { id } => {
        write!(formatter, "operation {id} requires a receiver")
      }
      Self::UnexpectedReceiver { id } => {
        write!(formatter, "operation {id} unexpectedly has a receiver")
      }
      Self::WrongReceiverKind { id } => {
        write!(formatter, "operation {id} has an incompatible receiver")
      }
      Self::MissingResultResource { id } => write!(
        formatter,
        "output-stream operation {id} has no result resource"
      ),
      Self::WrongDispatch { id } => write!(
        formatter,
        "operation {id} has an incompatible host dispatch"
      ),
      Self::HostOperationMustBeAsync { id } => {
        write!(formatter, "host stream operation {id} must be async")
      }
      Self::CancelHasResult { id } => {
        write!(
          formatter,
          "stream cancel operation {id} must not return a payload"
        )
      }
      Self::NonOwnedResultResource { id, ownership } => write!(
        formatter,
        "operation {id} has non-owned result resource binding {ownership:?}"
      ),
      Self::DuplicateResultResourceUseSite { id, path } => write!(
        formatter,
        "operation {id} declares duplicate result resource use-site {path}"
      ),
      Self::ConflictingResultResourceUseSite { id, path } => write!(
        formatter,
        "operation {id} declares conflicting result resource use-site {path}"
      ),
      Self::UseSiteOperationMismatch {
        expected,
        actual,
        role,
      } => write!(
        formatter,
        "{role} use-site belongs to operation {actual}, expected {expected}"
      ),
      Self::EmptyPath { id, role } => write!(formatter, "operation {id} has an empty {role} path"),
      Self::InvalidPathRoot { id, role } => {
        write!(formatter, "operation {id} has an invalid {role} path root")
      }
      Self::NestedRootSegment { id, role } => {
        write!(
          formatter,
          "operation {id} has a nested root segment in its {role} path"
        )
      }
      Self::InvalidStreamStepPath { id, role } => write!(
        formatter,
        "operation {id} has an invalid stream-step selector in its {role} path"
      ),
      Self::ArgumentPathOutOfRange {
        id,
        argument,
        count,
        role,
      } => write!(
        formatter,
        "operation {id} {role} path argument {argument} is outside {count} arguments"
      ),
      Self::DuplicateStreamUseSite { use_site_id } => {
        write!(formatter, "duplicate stream use-site ID {use_site_id}")
      }
      Self::NonDenseStreamUseSiteId => formatter.write_str("stream use-site IDs are not dense"),
      Self::DuplicateStreamSlot { use_site_id, kind } => write!(
        formatter,
        "stream use-site {use_site_id} has duplicate {kind:?} slot"
      ),
      Self::StreamSlotOperationMismatch { expected, actual } => write!(
        formatter,
        "stream slot operation ID {actual} does not match operation {expected}"
      ),
      Self::StreamSlotKindMismatch {
        operation_id,
        expected,
        actual,
      } => write!(
        formatter,
        "stream slot for operation {operation_id} has kind {actual:?}, expected {expected:?}"
      ),
      Self::StreamSlotUseSiteMismatch { expected, actual } => write!(
        formatter,
        "stream slot use-site ID {actual} does not match use-site {expected}"
      ),
      Self::UnknownStreamSlotOperation {
        use_site_id,
        operation_id,
      } => write!(
        formatter,
        "stream use-site {use_site_id} references unknown slot operation {operation_id}"
      ),
      Self::StreamSlotIdentityMismatch {
        use_site_id,
        operation_id,
      } => write!(
        formatter,
        "stream use-site {use_site_id} slot operation {operation_id} has mismatched identity"
      ),
      Self::InvalidStreamSlot { id } => {
        write!(formatter, "operation {id} has an invalid stream slot")
      }
      Self::OrphanStreamSlot {
        use_site_id,
        operation_id,
        kind,
      } => write!(
        formatter,
        "stream slot {kind:?} operation {operation_id} is not referenced by use-site {use_site_id}"
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

  const TEST_CLOSE_POLICY: ClosePolicy = ClosePolicy {
    grace_ms: 5_000,
    on_deadline: DeadlineAction::Detach,
  };

  fn operation(id: u32, kind: OperationKind, dispatch: OperationDispatch) -> FamilyOperationInput {
    FamilyOperationInput {
      id,
      kind,
      async_kind: AsyncKind::Sync,
      fallible: false,
      argument_count: 0,
      dispatch,
      receiver: None,
      result_resources: Vec::new(),
      callbacks: Vec::new(),
      streams: Vec::new(),
      stream_slot: None,
    }
  }

  #[test]
  fn bigint_boundaries_are_lossless_and_signed() {
    assert_eq!(require_lossless_i64(i64::MIN, true), Ok(i64::MIN));
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
  }

  #[test]
  fn close_policy_is_required_and_timer_safe() {
    let operation = operation(0, OperationKind::Function, OperationDispatch::Native);
    let mut plan_input = FamilyPlanInput {
      flavor: HostFlavor::Node,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![operation.clone()],
    };
    let plan = FamilyPlan::build(plan_input.clone()).unwrap();
    assert_eq!(plan.close_policy(), TEST_CLOSE_POLICY);

    plan_input.close_policy.grace_ms = i32::MAX as u32 + 1;
    assert!(matches!(
      FamilyPlan::build(plan_input),
      Err(FamilyPlanError::ClosePolicyOutOfRange { .. })
    ));
  }

  #[test]
  fn family_validates_dense_ids_method_dispatch_and_lifetimes() {
    let plan = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Node,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![
        operation(1, OperationKind::Function, OperationDispatch::Native),
        operation(0, OperationKind::Function, OperationDispatch::Native),
      ],
    })
    .unwrap();
    assert_eq!(plan.operations()[0].id, 0);
    assert!(plan
      .runtime_entrypoints()
      .any(|entry| entry == RuntimeEntrypoint::CloseSession));

    let bad = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Node,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![operation(
        0,
        OperationKind::CallbackMethod,
        OperationDispatch::Native,
      )],
    })
    .unwrap_err();
    assert!(bad.to_string().contains("incompatible host dispatch"));

    let mut misplaced_host = operation(
      0,
      OperationKind::Function,
      OperationDispatch::InputStreamHostPull,
    );
    misplaced_host.async_kind = AsyncKind::Async;
    let bad = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Node,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![misplaced_host],
    })
    .unwrap_err();
    assert!(matches!(bad, FamilyPlanError::WrongDispatch { id: 0 }));
  }

  #[test]
  fn method_receivers_preserve_value_or_resource_category() {
    let mut value_method = operation(0, OperationKind::Method, OperationDispatch::Native);
    value_method.receiver = Some(ReceiverBinding::Value);
    let value_plan = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Node,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![value_method],
    })
    .unwrap();
    assert_eq!(
      value_plan.operations()[0].receiver,
      Some(ReceiverBinding::Value)
    );
    assert!(!value_plan
      .runtime_entrypoints()
      .any(|entry| entry == RuntimeEntrypoint::ReleaseObject));

    let mut resource_method = operation(0, OperationKind::Method, OperationDispatch::Native);
    resource_method.receiver = Some(ReceiverBinding::Resource(ResourceBinding {
      kind: ResourceKind::Object,
      ownership: ResourceOwnership::Borrowed,
    }));
    let resource_plan = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Node,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![resource_method],
    })
    .unwrap();
    assert!(matches!(
      resource_plan.operations()[0].receiver,
      Some(ReceiverBinding::Resource(ResourceBinding {
        kind: ResourceKind::Object,
        ownership: ResourceOwnership::Borrowed,
      }))
    ));
    assert!(resource_plan
      .runtime_entrypoints()
      .any(|entry| entry == RuntimeEntrypoint::ReleaseObject));

    let mut stream = operation(
      0,
      OperationKind::InputStreamPull,
      OperationDispatch::InputStreamHostPull,
    );
    stream.async_kind = AsyncKind::Async;
    stream.receiver = Some(ReceiverBinding::Value);
    assert!(matches!(
      FamilyPlan::build(FamilyPlanInput {
        flavor: HostFlavor::Node,
        close_policy: TEST_CLOSE_POLICY,
        operations: vec![stream],
      }),
      Err(FamilyPlanError::WrongReceiverKind { id: 0 })
    ));
  }

  #[test]
  fn callback_contract_and_stream_paths_are_retained_without_type_graph() {
    let mut op = operation(0, OperationKind::Function, OperationDispatch::Native);
    op.argument_count = 1;
    op.callbacks.push(CallbackUseSite {
      operation_id: 0,
      callback_type_id: 7,
      path: ValuePath::argument(0),
      contract: CallbackContract {
        retention: CallbackRetention::Retained,
        threading: CallbackThreading::CallingThread,
        reentrancy: CallbackReentrancy::Forbidden,
      },
    });
    op.streams.push(StreamUseSite {
      operation_id: 0,
      use_site_id: 0,
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
          use_site_id: 0,
          operation_id: 1,
          kind: OperationKind::InputStreamPull,
        },
        StreamSlotIdentity {
          use_site_id: 0,
          operation_id: 2,
          kind: OperationKind::InputStreamCancel,
        },
      ],
    });
    let mut pull = operation(
      1,
      OperationKind::InputStreamPull,
      OperationDispatch::InputStreamHostPull,
    );
    pull.async_kind = AsyncKind::Async;
    pull.receiver = Some(ReceiverBinding::Resource(ResourceBinding {
      kind: ResourceKind::InputStream,
      ownership: ResourceOwnership::Borrowed,
    }));
    pull.stream_slot = Some(StreamSlotIdentity {
      use_site_id: 0,
      operation_id: 1,
      kind: OperationKind::InputStreamPull,
    });
    let mut cancel = operation(
      2,
      OperationKind::InputStreamCancel,
      OperationDispatch::InputStreamHostCancel,
    );
    cancel.async_kind = AsyncKind::Async;
    cancel.receiver = Some(ReceiverBinding::Resource(ResourceBinding {
      kind: ResourceKind::InputStream,
      ownership: ResourceOwnership::Borrowed,
    }));
    cancel.stream_slot = Some(StreamSlotIdentity {
      use_site_id: 0,
      operation_id: 2,
      kind: OperationKind::InputStreamCancel,
    });
    let mut bad_cancel = cancel.clone();
    bad_cancel.result_resources.push(ResultResourceUseSite {
      operation_id: bad_cancel.id,
      path: ValuePath::return_value(),
      binding: ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Owned,
      },
    });
    let error = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Ohos,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![op.clone(), pull.clone(), bad_cancel],
    })
    .unwrap_err();
    assert!(matches!(error, FamilyPlanError::CancelHasResult { id: 2 }));
    let mut orphan_owner = op.clone();
    orphan_owner.streams.clear();
    let error = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Ohos,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![orphan_owner, pull.clone(), cancel.clone()],
    })
    .unwrap_err();
    assert!(matches!(
      error,
      FamilyPlanError::OrphanStreamSlot {
        use_site_id: 0,
        operation_id: 1,
        kind: OperationKind::InputStreamPull,
      }
    ));
    let mut mismatched_slot = cancel.clone();
    mismatched_slot.stream_slot.as_mut().unwrap().kind = OperationKind::InputStreamPull;
    let error = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Ohos,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![op.clone(), pull.clone(), mismatched_slot],
    })
    .unwrap_err();
    assert!(matches!(
      error,
      FamilyPlanError::StreamSlotKindMismatch {
        operation_id: 2,
        expected: OperationKind::InputStreamCancel,
        actual: OperationKind::InputStreamPull,
      }
    ));
    let plan = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Ohos,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![op, pull, cancel],
    })
    .unwrap();
    assert_eq!(plan.operations()[0].callbacks[0].callback_type_id, 7);
    assert!(plan
      .runtime_entrypoints()
      .any(|entry| entry == RuntimeEntrypoint::RetainCallback));
    assert!(plan
      .runtime_entrypoints()
      .any(|entry| entry == RuntimeEntrypoint::PullInputStream));
  }

  #[test]
  fn paths_have_one_root_and_no_nested_operation_selectors() {
    let mut op = operation(0, OperationKind::Function, OperationDispatch::Native);
    op.argument_count = 1;
    op.callbacks.push(CallbackUseSite {
      operation_id: 0,
      callback_type_id: 1,
      path: ValuePath::new(vec![ValuePathSegment::Field("field".into())]),
      contract: CallbackContract {
        retention: CallbackRetention::Scoped,
        threading: CallbackThreading::CallingThread,
        reentrancy: CallbackReentrancy::Allowed,
      },
    });
    let error = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Node,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![op.clone()],
    })
    .unwrap_err();
    assert!(error.to_string().contains("invalid callback path root"));

    op.callbacks[0].path = ValuePath::new(vec![
      ValuePathSegment::Argument(0),
      ValuePathSegment::Return,
    ]);
    let error = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Node,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![op],
    })
    .unwrap_err();
    assert!(error.to_string().contains("nested root segment"));

    let mut valid = operation(0, OperationKind::Function, OperationDispatch::Native);
    valid.argument_count = 1;
    valid.callbacks.push(CallbackUseSite {
      operation_id: 0,
      callback_type_id: 1,
      path: ValuePath::new(vec![
        ValuePathSegment::Argument(0),
        ValuePathSegment::Field("field".into()),
      ]),
      contract: CallbackContract {
        retention: CallbackRetention::Scoped,
        threading: CallbackThreading::CallingThread,
        reentrancy: CallbackReentrancy::Allowed,
      },
    });
    FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Node,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![valid],
    })
    .unwrap();
  }

  #[test]
  fn result_resource_paths_are_return_rooted_owned_and_unique() {
    let mut operation = operation(0, OperationKind::Function, OperationDispatch::Native);
    operation.result_resources.push(ResultResourceUseSite {
      operation_id: 0,
      path: ValuePath::new(vec![
        ValuePathSegment::Return,
        ValuePathSegment::Field("object".into()),
        ValuePathSegment::Optional,
      ]),
      binding: ResourceBinding {
        kind: ResourceKind::Object,
        ownership: ResourceOwnership::Owned,
      },
    });
    operation.result_resources.push(ResultResourceUseSite {
      operation_id: 0,
      path: ValuePath::new(vec![
        ValuePathSegment::Return,
        ValuePathSegment::SequenceElement,
        ValuePathSegment::Field("stream".into()),
      ]),
      binding: ResourceBinding {
        kind: ResourceKind::OutputStream,
        ownership: ResourceOwnership::Owned,
      },
    });
    for (path, kind) in [
      (
        ValuePath::new(vec![
          ValuePathSegment::Return,
          ValuePathSegment::Variant("Ready".into()),
          ValuePathSegment::Field("object".into()),
        ]),
        ResourceKind::Object,
      ),
      (
        ValuePath::new(vec![
          ValuePathSegment::Return,
          ValuePathSegment::SetElement,
          ValuePathSegment::Field("object".into()),
        ]),
        ResourceKind::Object,
      ),
      (
        ValuePath::new(vec![
          ValuePathSegment::Return,
          ValuePathSegment::MapKey,
          ValuePathSegment::Field("object".into()),
        ]),
        ResourceKind::Object,
      ),
      (
        ValuePath::new(vec![
          ValuePathSegment::Return,
          ValuePathSegment::MapValue,
          ValuePathSegment::Field("object".into()),
        ]),
        ResourceKind::Object,
      ),
    ] {
      operation.result_resources.push(ResultResourceUseSite {
        operation_id: 0,
        path,
        binding: ResourceBinding {
          kind,
          ownership: ResourceOwnership::Owned,
        },
      });
    }
    let plan = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Node,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![operation.clone()],
    })
    .unwrap();
    assert_eq!(plan.operations()[0].result_resources.len(), 6);

    let mut invalid = operation.clone();
    invalid.result_resources[0].path = ValuePath::argument(0);
    assert!(matches!(
      FamilyPlan::build(FamilyPlanInput {
        flavor: HostFlavor::Node,
        close_policy: TEST_CLOSE_POLICY,
        operations: vec![invalid],
      }),
      Err(FamilyPlanError::InvalidPathRoot {
        role: "result resource",
        ..
      })
    ));

    let mut borrowed = operation.clone();
    borrowed.result_resources[0].binding.ownership = ResourceOwnership::Borrowed;
    assert!(matches!(
      FamilyPlan::build(FamilyPlanInput {
        flavor: HostFlavor::Node,
        close_policy: TEST_CLOSE_POLICY,
        operations: vec![borrowed],
      }),
      Err(FamilyPlanError::NonOwnedResultResource { .. })
    ));

    let mut by_arc = operation.clone();
    by_arc.result_resources[0].binding.ownership = ResourceOwnership::ByArc;
    assert!(matches!(
      FamilyPlan::build(FamilyPlanInput {
        flavor: HostFlavor::Node,
        close_policy: TEST_CLOSE_POLICY,
        operations: vec![by_arc],
      }),
      Err(FamilyPlanError::NonOwnedResultResource { .. })
    ));

    let mut nested_root = operation.clone();
    nested_root.result_resources[0].path = ValuePath::new(vec![
      ValuePathSegment::Return,
      ValuePathSegment::Field("object".into()),
      ValuePathSegment::Argument(0),
    ]);
    assert!(matches!(
      FamilyPlan::build(FamilyPlanInput {
        flavor: HostFlavor::Node,
        close_policy: TEST_CLOSE_POLICY,
        operations: vec![nested_root],
      }),
      Err(FamilyPlanError::NestedRootSegment {
        role: "result resource",
        ..
      })
    ));

    let mut duplicate = operation.clone();
    duplicate
      .result_resources
      .push(duplicate.result_resources[0].clone());
    assert!(matches!(
      FamilyPlan::build(FamilyPlanInput {
        flavor: HostFlavor::Node,
        close_policy: TEST_CLOSE_POLICY,
        operations: vec![duplicate],
      }),
      Err(FamilyPlanError::DuplicateResultResourceUseSite { .. })
    ));

    let mut conflict = operation;
    conflict.result_resources.push(ResultResourceUseSite {
      operation_id: 0,
      path: ValuePath::new(vec![
        ValuePathSegment::Return,
        ValuePathSegment::Field("object".into()),
        ValuePathSegment::Optional,
      ]),
      binding: ResourceBinding {
        kind: ResourceKind::InputStream,
        ownership: ResourceOwnership::Owned,
      },
    });
    assert!(matches!(
      FamilyPlan::build(FamilyPlanInput {
        flavor: HostFlavor::Node,
        close_policy: TEST_CLOSE_POLICY,
        operations: vec![conflict],
      }),
      Err(FamilyPlanError::ConflictingResultResourceUseSite { .. })
    ));
  }

  #[test]
  fn output_stream_step_selectors_are_first_class_and_strictly_placed() {
    let mut next = operation(
      0,
      OperationKind::OutputStreamNext,
      OperationDispatch::Native,
    );
    next.async_kind = AsyncKind::Async;
    next.receiver = Some(ReceiverBinding::Resource(ResourceBinding {
      kind: ResourceKind::OutputStream,
      ownership: ResourceOwnership::Borrowed,
    }));
    next.result_resources.push(ResultResourceUseSite {
      operation_id: 0,
      path: ValuePath::new(vec![
        ValuePathSegment::Return,
        ValuePathSegment::StreamItem,
        ValuePathSegment::Field("object".to_owned()),
      ]),
      binding: ResourceBinding {
        kind: ResourceKind::Object,
        ownership: ResourceOwnership::Owned,
      },
    });
    let plan = FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Node,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![next.clone()],
    })
    .unwrap();
    assert_eq!(
      plan.operations()[0].result_resources[0].path.to_string(),
      "return.stream-item.field[object]"
    );

    let mut error_path = next.clone();
    error_path.result_resources[0].path = ValuePath::new(vec![
      ValuePathSegment::Return,
      ValuePathSegment::StreamError,
      ValuePathSegment::Field("object".to_owned()),
    ]);
    FamilyPlan::build(FamilyPlanInput {
      flavor: HostFlavor::Ohos,
      close_policy: TEST_CLOSE_POLICY,
      operations: vec![error_path],
    })
    .unwrap();

    let mut wrong_operation = next.clone();
    wrong_operation.kind = OperationKind::Function;
    wrong_operation.receiver = None;
    assert!(matches!(
      FamilyPlan::build(FamilyPlanInput {
        flavor: HostFlavor::Node,
        close_policy: TEST_CLOSE_POLICY,
        operations: vec![wrong_operation],
      }),
      Err(FamilyPlanError::InvalidStreamStepPath {
        id: 0,
        role: "result resource"
      })
    ));

    let mut nested = next.clone();
    nested.result_resources[0].path = ValuePath::new(vec![
      ValuePathSegment::Return,
      ValuePathSegment::Field("step".to_owned()),
      ValuePathSegment::StreamItem,
    ]);
    assert!(matches!(
      FamilyPlan::build(FamilyPlanInput {
        flavor: HostFlavor::Node,
        close_policy: TEST_CLOSE_POLICY,
        operations: vec![nested],
      }),
      Err(FamilyPlanError::InvalidStreamStepPath { .. })
    ));

    let mut duplicate = next;
    duplicate.result_resources[0].path = ValuePath::new(vec![
      ValuePathSegment::Return,
      ValuePathSegment::StreamItem,
      ValuePathSegment::StreamError,
    ]);
    assert!(matches!(
      FamilyPlan::build(FamilyPlanInput {
        flavor: HostFlavor::Node,
        close_policy: TEST_CLOSE_POLICY,
        operations: vec![duplicate],
      }),
      Err(FamilyPlanError::InvalidStreamStepPath { .. })
    ));

    let mut callback = operation(0, OperationKind::Function, OperationDispatch::Native);
    callback.callbacks.push(CallbackUseSite {
      operation_id: 0,
      callback_type_id: 1,
      path: ValuePath::new(vec![ValuePathSegment::Return, ValuePathSegment::StreamItem]),
      contract: CallbackContract {
        retention: CallbackRetention::Scoped,
        threading: CallbackThreading::CallingThread,
        reentrancy: CallbackReentrancy::Allowed,
      },
    });
    assert!(matches!(
      FamilyPlan::build(FamilyPlanInput {
        flavor: HostFlavor::Node,
        close_policy: TEST_CLOSE_POLICY,
        operations: vec![callback],
      }),
      Err(FamilyPlanError::InvalidStreamStepPath { .. })
    ));
  }
}
