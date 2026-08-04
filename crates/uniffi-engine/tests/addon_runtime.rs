use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use napi_family_core::HostFlavor;
use napi_uniffi_engine::{
  generate_napi_module, ArgumentBinding, ErrorBinding, ReturnBinding, RustArgumentPlan,
  RustBridgePlan, RustOperationPlan, RustOperationTarget, RustReceiverPlan, RustResourceHook,
  RustResourceHooks,
};
use proc_macro2::{Ident, Span};
use uniffi_js_abi::{
  ArgumentDefinition, AsyncKind, ComponentDefinition, ComponentId, ComponentKey,
  IdentifiedComponent, IdentifiedOperation, IdentifiedType, NamedTypeKind, OperationDefinition,
  OperationId, OperationKind, OperationOwner, OperationSignature, OperationSourceKey, Ownership,
  ScalarType, TypeDefinition, TypeId, TypeSourceKey, ValueType,
};
use uniffi_js_engine_schema::{
  BridgePlan, BridgePlanInput, CallbackCallStyle, CallbackContract, CallbackErrorStyle,
  CallbackReentrancy, CallbackRetention, CallbackThreading, CallbackUseSite, PlannedOperation,
  StreamContract, StreamUseSite, ValuePath,
};

struct TempFixture(PathBuf);

impl Drop for TempFixture {
  fn drop(&mut self) {
    let _ = fs::remove_dir_all(&self.0);
  }
}

fn operation(
  component: &ComponentKey,
  id: u32,
  name: &str,
  arguments: Vec<ArgumentDefinition>,
  return_type: ValueType,
  async_kind: AsyncKind,
) -> PlannedOperation {
  operation_with_owner(
    component,
    id,
    OperationOwner::Namespace,
    OperationKind::Function,
    name,
    arguments,
    Some(return_type),
    async_kind,
    None,
  )
}

#[expect(clippy::too_many_arguments)]
fn operation_with_owner(
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
      format!("fixture_{name}"),
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

fn plan() -> (BridgePlan, RustBridgePlan) {
  let component = ComponentKey::new("runtime_fixture").unwrap();
  let callback_key = TypeSourceKey::new(component.clone(), "Observer").unwrap();
  let failure_key = TypeSourceKey::new(component.clone(), "ObserverFailure").unwrap();
  let object_key = TypeSourceKey::new(component.clone(), "Thing").unwrap();
  let output_key = TypeSourceKey::new(component.clone(), "OutputResource").unwrap();
  let bridge = BridgePlan::build(BridgePlanInput {
    components: vec![IdentifiedComponent {
      id: ComponentId::new(0),
      definition: ComponentDefinition::new(component.clone(), "runtimeFixture").unwrap(),
    }],
    types: vec![
      IdentifiedType {
        id: TypeId::new(0),
        definition: TypeDefinition::new(callback_key.clone(), "Observer", NamedTypeKind::Callback)
          .unwrap(),
      },
      IdentifiedType {
        id: TypeId::new(1),
        definition: TypeDefinition::new(
          failure_key.clone(),
          "ObserverFailure",
          NamedTypeKind::Error {
            variants: Vec::new(),
          },
        )
        .unwrap(),
      },
      IdentifiedType {
        id: TypeId::new(2),
        definition: TypeDefinition::new(object_key.clone(), "Thing", NamedTypeKind::Object)
          .unwrap(),
      },
      IdentifiedType {
        id: TypeId::new(3),
        definition: TypeDefinition::new(
          output_key.clone(),
          "OutputResource",
          NamedTypeKind::Object,
        )
        .unwrap(),
      },
    ],
    operations: vec![
      operation(
        &component,
        0,
        "answer",
        Vec::new(),
        ValueType::Scalar(ScalarType::I64),
        AsyncKind::Sync,
      ),
      operation_with_owner(
        &component,
        7,
        OperationOwner::Object(object_key),
        OperationKind::Method,
        "touchResource",
        Vec::new(),
        Some(ValueType::Scalar(ScalarType::U32)),
        AsyncKind::Sync,
        None,
      ),
      operation_with_owner(
        &component,
        8,
        OperationOwner::Object(output_key),
        OperationKind::OutputStreamNext,
        "nextResource",
        Vec::new(),
        Some(ValueType::Scalar(ScalarType::U32)),
        AsyncKind::Async,
        None,
      ),
      operation(
        &component,
        9,
        "releaseCount",
        Vec::new(),
        ValueType::Scalar(ScalarType::U32),
        AsyncKind::Sync,
      ),
      operation(
        &component,
        1,
        "plusOne",
        vec![ArgumentDefinition::new(
          "value",
          ValueType::Scalar(ScalarType::I64),
          Ownership::Owned,
        )
        .unwrap()],
        ValueType::Scalar(ScalarType::I64),
        AsyncKind::Async,
      ),
      operation_with_owner(
        &component,
        2,
        OperationOwner::Callback(callback_key),
        OperationKind::CallbackMethod,
        "onValue",
        vec![ArgumentDefinition::new(
          "value",
          ValueType::Scalar(ScalarType::I64),
          Ownership::Owned,
        )
        .unwrap()],
        Some(ValueType::Scalar(ScalarType::I64)),
        AsyncKind::Async,
        Some(failure_key),
      ),
      operation_with_owner(
        &component,
        3,
        OperationOwner::Namespace,
        OperationKind::InputStreamPull,
        "pullInput",
        vec![ArgumentDefinition::new(
          "streamId",
          ValueType::Scalar(ScalarType::U32),
          Ownership::Owned,
        )
        .unwrap()],
        Some(ValueType::Scalar(ScalarType::U32)),
        AsyncKind::Async,
        None,
      ),
      operation_with_owner(
        &component,
        4,
        OperationOwner::Namespace,
        OperationKind::InputStreamCancel,
        "cancelInput",
        vec![ArgumentDefinition::new(
          "streamId",
          ValueType::Scalar(ScalarType::U32),
          Ownership::Owned,
        )
        .unwrap()],
        None,
        AsyncKind::Async,
        None,
      ),
      operation(
        &component,
        5,
        "observe",
        vec![ArgumentDefinition::new(
          "observer",
          ValueType::Named(TypeSourceKey::new(component.clone(), "Observer").unwrap()),
          Ownership::Owned,
        )
        .unwrap()],
        ValueType::Scalar(ScalarType::U32),
        AsyncKind::Sync,
      ),
      operation(
        &component,
        6,
        "consumeInput",
        vec![ArgumentDefinition::new(
          "source",
          ValueType::input_stream(ValueType::Scalar(ScalarType::Bytes)),
          Ownership::Owned,
        )
        .unwrap()],
        ValueType::Scalar(ScalarType::U32),
        AsyncKind::Sync,
      ),
    ],
    callbacks: vec![CallbackUseSite {
      operation_id: OperationId::new(5),
      callback_type: TypeId::new(0),
      path: ValuePath::argument(0),
      contract: CallbackContract {
        retention: CallbackRetention::Retained,
        threading: CallbackThreading::MayCrossThread,
        call_style: CallbackCallStyle::Async,
        error_style: CallbackErrorStyle::Fallible,
        reentrancy: CallbackReentrancy::Forbidden,
      },
    }],
    streams: vec![StreamUseSite {
      operation_id: OperationId::new(6),
      path: ValuePath::argument(0),
      contract: StreamContract::input(),
    }],
    targets: vec![HostFlavor::Node.capabilities()],
  })
  .unwrap();
  let rust = RustBridgePlan::build_with_resource_hooks(
    &bridge,
    vec![
      RustOperationPlan {
        operation_id: OperationId::new(0),
        target: RustOperationTarget::Native {
          call: syn::parse_quote!(fixture::answer),
        },
        receiver: None,
        arguments: Vec::new(),
        return_binding: ReturnBinding::I64BigInt,
        error_binding: ErrorBinding::Infallible,
      },
      RustOperationPlan {
        operation_id: OperationId::new(7),
        target: RustOperationTarget::Native {
          call: syn::parse_quote!(fixture::touch_resource),
        },
        receiver: Some(RustReceiverPlan {
          name: Ident::new("resource", Span::call_site()),
          binding: ArgumentBinding::ObjectLease {
            carrier_type: syn::parse_quote!(u32),
            lower: syn::parse_quote!(fixture::lower_resource),
            ownership: Ownership::Borrowed,
          },
        }),
        arguments: Vec::new(),
        return_binding: ReturnBinding::Direct {
          carrier_type: syn::parse_quote!(u32),
        },
        error_binding: ErrorBinding::Infallible,
      },
      RustOperationPlan {
        operation_id: OperationId::new(8),
        target: RustOperationTarget::Native {
          call: syn::parse_quote!(fixture::next_resource),
        },
        receiver: Some(RustReceiverPlan {
          name: Ident::new("resource", Span::call_site()),
          binding: ArgumentBinding::OutputStreamLease {
            carrier_type: syn::parse_quote!(u32),
            lower: syn::parse_quote!(fixture::lower_resource),
            ownership: Ownership::Borrowed,
          },
        }),
        arguments: Vec::new(),
        return_binding: ReturnBinding::Direct {
          carrier_type: syn::parse_quote!(u32),
        },
        error_binding: ErrorBinding::Infallible,
      },
      RustOperationPlan {
        operation_id: OperationId::new(9),
        target: RustOperationTarget::Native {
          call: syn::parse_quote!(fixture::release_count),
        },
        receiver: None,
        arguments: Vec::new(),
        return_binding: ReturnBinding::Direct {
          carrier_type: syn::parse_quote!(u32),
        },
        error_binding: ErrorBinding::Infallible,
      },
      RustOperationPlan {
        operation_id: OperationId::new(5),
        target: RustOperationTarget::Native {
          call: syn::parse_quote!(fixture::observe),
        },
        receiver: None,
        arguments: vec![RustArgumentPlan {
          name: Ident::new("observer", Span::call_site()),
          binding: ArgumentBinding::CallbackProxy {
            rust_type: syn::parse_quote!(u32),
            build: syn::parse_quote!(fixture::build_callback_proxy),
          },
        }],
        return_binding: ReturnBinding::Direct {
          carrier_type: syn::parse_quote!(u32),
        },
        error_binding: ErrorBinding::Infallible,
      },
      RustOperationPlan {
        operation_id: OperationId::new(6),
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
        operation_id: OperationId::new(2),
        target: RustOperationTarget::CallbackHost,
        receiver: None,
        arguments: Vec::new(),
        return_binding: ReturnBinding::Unit,
        error_binding: ErrorBinding::Infallible,
      },
      RustOperationPlan {
        operation_id: OperationId::new(3),
        target: RustOperationTarget::InputStreamHostPull,
        receiver: None,
        arguments: Vec::new(),
        return_binding: ReturnBinding::Unit,
        error_binding: ErrorBinding::Infallible,
      },
      RustOperationPlan {
        operation_id: OperationId::new(4),
        target: RustOperationTarget::InputStreamHostCancel,
        receiver: None,
        arguments: Vec::new(),
        return_binding: ReturnBinding::Unit,
        error_binding: ErrorBinding::Infallible,
      },
      RustOperationPlan {
        operation_id: OperationId::new(1),
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
    ],
    RustResourceHooks {
      release_object: Some(RustResourceHook {
        call: syn::parse_quote!(fixture::release_object),
        carrier_type: syn::parse_quote!(u32),
      }),
      cancel_output_stream: Some(RustResourceHook {
        call: syn::parse_quote!(fixture::cancel_output_stream),
        carrier_type: syn::parse_quote!(u32),
      }),
      release_output_stream: Some(RustResourceHook {
        call: syn::parse_quote!(fixture::release_output_stream),
        carrier_type: syn::parse_quote!(u32),
      }),
    },
  )
  .unwrap();
  (bridge, rust)
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
fn generated_factory_runs_sync_bigint_async_and_close_in_node() {
  let (bridge, rust) = plan();
  let generated = generate_napi_module(&bridge, &rust, HostFlavor::Node).unwrap();
  let fixture = fixture_root();
  let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
    .ancestors()
    .nth(2)
    .unwrap();
  let napi = repository.join("crates/napi");
  let napi_derive = repository.join("crates/macro");
  let napi_build = repository.join("crates/build");
  let engine = repository.join("crates/uniffi-engine");
  let target = repository.join("target/uniffi-engine-generated-addon");

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
    napi, napi_derive, engine, napi_build
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
use std::sync::atomic::{{AtomicBool, Ordering}};
use std::sync::Mutex;

use napi::bindgen_prelude::{{
  register_async_runtime, AsyncRuntime, AsyncRuntimeRejection, AsyncRuntimeTask,
}};

struct FixtureRuntime {{
  active: AtomicBool,
  workers: Mutex<Vec<std::thread::JoinHandle<()>>>,
}}

unsafe impl AsyncRuntime for FixtureRuntime {{
  fn spawn(
    &self,
    task: AsyncRuntimeTask,
  ) -> std::result::Result<(), AsyncRuntimeRejection<AsyncRuntimeTask>> {{
    if !self.active.load(Ordering::Acquire) {{
      return Err(AsyncRuntimeRejection::new(
        task,
        napi::Error::new(napi::Status::GenericFailure, "fixture runtime is stopped"),
      ));
    }}
    self.workers.lock().unwrap().push(std::thread::spawn(move || {{
      futures::executor::block_on(task);
    }}));
    Ok(())
  }}

  fn block_on(&self, future: Pin<&mut dyn Future<Output = ()>>) -> napi::Result<()> {{
    futures::executor::block_on(future);
    Ok(())
  }}

  fn start(&self) -> napi::Result<()> {{
    self.active.store(true, Ordering::Release);
    Ok(())
  }}

  fn shutdown(&self) -> napi::Result<()> {{
    self.active.store(false, Ordering::Release);
    for worker in self.workers.lock().unwrap().drain(..) {{
      let _ = worker.join();
    }}
    Ok(())
  }}
}}

#[napi_derive::module_init]
fn install_runtime() {{
  register_async_runtime(FixtureRuntime {{
    active: AtomicBool::new(false),
    workers: Mutex::new(Vec::new()),
  }});
}}

mod fixture {{
  use std::sync::atomic::{{AtomicU32, Ordering}};

  static RELEASE_COUNT: AtomicU32 = AtomicU32::new(0);

  pub fn answer() -> i64 {{ 42 }}
  pub async fn plus_one(value: i64) -> i64 {{ value + 1 }}
  pub fn build_callback_proxy(
    _host: &napi::bindgen_prelude::Object<'static>,
    callback_type_id: u32,
    callback_id: u32,
    contract: napi_uniffi_engine::SessionCallbackArgument,
  ) -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{
    assert_eq!(callback_type_id, 0);
    assert_eq!(contract.callback_type_id, 0);
    assert_eq!(contract.retention, napi_uniffi_engine::SessionCallbackRetention::Retained);
    assert_eq!(contract.threading, napi_uniffi_engine::SessionCallbackThreading::MayCrossThread);
    assert_eq!(contract.call_style, napi_uniffi_engine::SessionCallbackCallStyle::Async);
    assert_eq!(contract.error_style, napi_uniffi_engine::SessionCallbackErrorStyle::Fallible);
    assert_eq!(contract.reentrancy, napi_uniffi_engine::SessionCallbackReentrancy::Forbidden);
    Ok(callback_id)
  }}
  pub fn observe(callback_id: u32) -> u32 {{ callback_id }}
  pub fn build_input_stream_proxy(
    _host: &napi::bindgen_prelude::Object<'static>,
    stream_id: u32,
  ) -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{
    Ok(stream_id)
  }}
  pub fn consume_input(stream_id: u32) -> u32 {{ stream_id }}
  pub fn lower_resource(handle: u32) -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{
    Ok(handle)
  }}
  pub fn touch_resource(handle: u32) -> u32 {{ handle }}
  pub async fn next_resource(handle: u32) -> u32 {{ handle }}
  pub fn release_count() -> u32 {{ RELEASE_COUNT.load(Ordering::Acquire) }}
  pub fn release_object(_handle: u32) -> Result<(), napi_uniffi_engine::BridgeErrorDescriptor> {{
    RELEASE_COUNT.fetch_add(1, Ordering::AcqRel);
    Ok(())
  }}
  pub async fn cancel_output_stream(
    _handle: u32,
  ) -> Result<(), napi_uniffi_engine::BridgeErrorDescriptor> {{
    RELEASE_COUNT.fetch_add(10, Ordering::AcqRel);
    Ok(())
  }}
  pub fn release_output_stream(
    _handle: u32,
  ) -> Result<(), napi_uniffi_engine::BridgeErrorDescriptor> {{
    RELEASE_COUNT.fetch_add(100, Ordering::AcqRel);
    Ok(())
  }}
}}

{}
"#,
    generated.source()
  );
  fs::write(fixture.0.join("src/lib.rs"), source).unwrap();

  let build = Command::new(env!("CARGO"))
    .arg("build")
    .arg("--manifest-path")
    .arg(fixture.0.join("Cargo.toml"))
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
const addon = require(process.argv[1]);
assert.deepEqual(Object.keys(addon), ['__uniffi_backend_factory']);
const callbackCalls = [];
const streamCalls = [];
const lifecycleCalls = [];
const host = {
  invokeCallbackAsync(callbackTypeId, callbackId, methodId, invocationId, args) {
    callbackCalls.push([callbackTypeId, callbackId, methodId, invocationId, args]);
    return Promise.resolve(args[0] + 7n);
  },
  pullInputStream(streamId) {
    streamCalls.push(['pull', streamId]);
    return Promise.resolve(streamId + 1);
  },
  cancelInputStream(streamId) {
    streamCalls.push(['cancel', streamId]);
    return Promise.resolve();
  },
  retainCallback(callbackTypeId, callbackId) {
    lifecycleCalls.push(['retainCallback', callbackTypeId, callbackId]);
  },
  releaseCallback(callbackTypeId, callbackId) {
    lifecycleCalls.push(['releaseCallback', callbackTypeId, callbackId]);
  },
  releaseInputStream(streamId) {
    lifecycleCalls.push(['releaseInputStream', streamId]);
  },
};
const session = addon.__uniffi_backend_factory(host);
assert.equal(session.hostFlavor, 'node');
assert.equal(session.invokeSync(0, []).kind, 'value');
assert.equal(session.invokeSync(0, []).value, 42n);
(async () => {
  const result = await session.invokeAsync(1, [41n]);
  assert.equal(result.kind, 'value');
  assert.equal(result.value, 42n);
  assert.equal(await session.invokeAsync(2, [7, 35n]), 42n);
  assert.deepEqual(callbackCalls, [[0, 7, 0, 0, [35n]]]);
  assert.equal(await session.invokeAsync(3, [9]), 10);
  await session.invokeAsync(4, [9]);
  assert.deepEqual(streamCalls, [['pull', 9], ['cancel', 9]]);
  assert.equal(session.invokeSync(5, [7]).value, 7);
  assert.equal(session.invokeSync(6, [9]).value, 9);
  assert.deepEqual(lifecycleCalls, [['retainCallback', 0, 7]]);
  const objectResource = { handle: 77 };
  assert.equal(session.invokeSync(7, [objectResource]).value, 77);
  session.releaseObject(objectResource);
  assert.equal(session.invokeSync(9, []).value, 1);
  const outputResource = { handle: 88 };
  assert.equal((await session.invokeAsync(8, [outputResource])).value, 88);
  const cancelResult = await session.cancelOutputStream(outputResource);
  assert.equal(cancelResult.kind, 'value');
  assert.equal(session.invokeSync(9, []).value, 11);
  const closeObject = { handle: 99 };
  const closeOutput = { handle: 100 };
  session.invokeSync(7, [closeObject]);
  await session.invokeAsync(8, [closeOutput]);
  await session.close();
  assert.deepEqual(lifecycleCalls, [
    ['retainCallback', 0, 7],
    ['releaseCallback', 0, 7],
    ['releaseInputStream', 9],
  ]);
  await session.close();
  assert.throws(() => session.invokeSync(0, []), /closed/);
  const nextSession = addon.__uniffi_backend_factory(host);
  assert.equal(nextSession.invokeSync(9, []).value, 112);
  await nextSession.close();
})().catch((error) => { console.error(error); process.exitCode = 1; });
"#;
  let node = Command::new("node")
    .arg("-e")
    .arg(script)
    .arg(&addon)
    .output()
    .unwrap();
  assert!(
    node.status.success(),
    "generated addon failed at runtime:\n{}\n{}",
    String::from_utf8_lossy(&node.stdout),
    String::from_utf8_lossy(&node.stderr)
  );
}
