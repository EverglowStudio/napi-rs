use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use napi_family_core::{
  AsyncKind, CallbackContract, CallbackReentrancy, CallbackRetention, CallbackThreading,
  CallbackUseSite, FamilyOperationInput, FamilyPlan, FamilyPlanInput, HostFlavor,
  OperationDispatch, OperationKind, StreamDirection, StreamUseSite, ValuePath,
};
use napi_uniffi_engine::{
  generate_napi_module, ArgumentBinding, ErrorBinding, ReturnBinding, RustArgumentPlan,
  RustBridgePlan, RustOperationPlan, RustOperationTarget,
};
use proc_macro2::{Ident, Span};

struct TempFixture(PathBuf);

impl Drop for TempFixture {
  fn drop(&mut self) {
    let _ = fs::remove_dir_all(&self.0);
  }
}

fn family() -> FamilyPlan {
  let operations = vec![
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
      callbacks: vec![CallbackUseSite {
        operation_id: 3,
        callback_type_id: 0,
        path: ValuePath::argument(0),
        contract: CallbackContract {
          retention: CallbackRetention::Retained,
          threading: CallbackThreading::CallingThread,
          reentrancy: CallbackReentrancy::Forbidden,
        },
      }],
      streams: Vec::new(),
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
        path: ValuePath::argument(0),
        direction: StreamDirection::Input,
      }],
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
    },
  ];
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
  ];
  RustBridgePlan::build(family, ops).unwrap()
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
  pub fn answer() -> i64 {{ 42 }}
  pub async fn plus_one(value: i64) -> i64 {{ value + 1 }}
  pub fn build_callback_proxy(_host: &napi::bindgen_prelude::Object<'static>, callback_type_id: u32, callback_id: u32, contract: napi_uniffi_engine::SessionCallbackArgument) -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{
    assert_eq!(callback_type_id, 0); assert_eq!(contract.callback_type_id, 0); assert_eq!(contract.retention, napi_uniffi_engine::SessionCallbackRetention::Retained); assert_eq!(contract.reentrancy, napi_uniffi_engine::SessionCallbackReentrancy::Forbidden); Ok(callback_id)
  }}
  pub fn observe(callback_id: u32) -> u32 {{ callback_id }}
  pub fn build_input_stream_proxy(_host: &napi::bindgen_prelude::Object<'static>, stream_id: u32) -> Result<u32, napi_uniffi_engine::BridgeErrorDescriptor> {{ Ok(stream_id) }}
  pub fn consume_input(stream_id: u32) -> u32 {{ stream_id }}
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
  const host = {
    retainCallback(typeId, id) { retained.push([typeId, id]); },
    releaseCallback(typeId, id) { retained.splice(retained.findIndex(([t, i]) => t === typeId && i === id), 1); },
    invokeCallbackAsync() { return Promise.resolve(7); },
    pullInputStream(id) { return Promise.resolve(id + 1); },
    cancelInputStream(id) { return Promise.resolve(id); },
    releaseInputStream(_id) {},
  };
  const session = addon.__uniffi_backend_factory(host);
  assert.equal(session.hostFlavor, 'node');
  assert.equal(session.invokeSync(0, []).kind, 'value');
  assert.equal(session.invokeSync(0, []).value, 42n);
  assert.equal((await session.invokeAsync(1, [41n])).value, 42n);
  assert.equal(session.invokeSync(3, [9]).value, 9);
  assert.deepEqual(retained, [[0, 9]]);
  assert.equal(session.invokeSync(4, [11]).value, 11);
  await session.invokeAsync(5, [11]);
  await session.invokeAsync(6, [11]);
  await session.close();
  assert.equal(retained.length, 0);
})().catch(error => { console.error(error); process.exitCode = 1; });
"#;
  let run = Command::new("node")
    .arg("--unhandled-rejections=strict")
    .arg("-e")
    .arg(script)
    .arg(&addon)
    .output()
    .unwrap();
  assert!(
    run.status.success(),
    "generated addon runtime failed:\n{}\n{}",
    String::from_utf8_lossy(&run.stdout),
    String::from_utf8_lossy(&run.stderr)
  );
}
