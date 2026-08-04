//! Native implementation of the private BackendSession boundary.
//!
//! The generated factory binds exactly one JavaScript Host object and keeps
//! operation callbacks behind native references.  Only the session methods
//! below are observable from JavaScript; raw callbacks never enter the module
//! export table or become properties of the returned session.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::ffi::{c_void, CString};
use std::ptr;

use napi::bindgen_prelude::{JsValue, Object};
use napi::{sys, Env, Error, Result, Status};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionCallbackRetention {
  Scoped,
  Retained,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionCallbackThreading {
  CallingThread,
  MayCrossThread,
}

/// Whether the generated host callback proxy may be entered again before the
/// current invocation returns.
///
/// Proxy builders must enforce this use-site contract when it is `Forbidden`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionCallbackReentrancy {
  Allowed,
  Forbidden,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionCallbackArgument {
  pub argument_index: u32,
  pub callback_type_id: u32,
  pub retention: SessionCallbackRetention,
  pub threading: SessionCallbackThreading,
  pub reentrancy: SessionCallbackReentrancy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionStreamDirection {
  Input,
  Output,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionStreamArgument {
  pub argument_index: u32,
  pub direction: SessionStreamDirection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionResourceReceiver {
  Object,
  OutputStream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionOperationDispatch {
  NativeSync,
  NativeAsync,
  CallbackHostSync {
    callback_type_id: u32,
    method_id: u32,
  },
  CallbackHostAsync {
    callback_type_id: u32,
    method_id: u32,
  },
  InputStreamHostPull,
  InputStreamHostCancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionNativeCall {
  ArgumentsOnly,
  HostAndArguments,
}

/// One dense operation slot supplied by generated code.
pub struct SessionOperationDescriptor {
  pub dispatch: SessionOperationDispatch,
  pub callback: Option<sys::napi_value>,
  pub native_call: SessionNativeCall,
  pub receiver: Option<SessionResourceReceiver>,
  pub callback_arguments: Vec<SessionCallbackArgument>,
  pub stream_arguments: Vec<SessionStreamArgument>,
}

pub struct SessionResourceCallbacks {
  pub release_object: Option<sys::napi_value>,
  pub cancel_output_stream: Option<sys::napi_value>,
  pub release_output_stream: Option<sys::napi_value>,
}

struct ResourceCallbacks {
  release_object: Cell<sys::napi_ref>,
  cancel_output_stream: Cell<sys::napi_ref>,
  release_output_stream: Cell<sys::napi_ref>,
}

struct TrackedResource {
  reference: sys::napi_ref,
  kind: SessionResourceReceiver,
}

struct SessionOperation {
  dispatch: SessionOperationDispatch,
  callback: Cell<sys::napi_ref>,
  native_call: SessionNativeCall,
  receiver: Option<SessionResourceReceiver>,
  callback_arguments: Vec<SessionCallbackArgument>,
  stream_arguments: Vec<SessionStreamArgument>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CallbackKey {
  callback_type_id: u32,
  callback_id: u32,
}

struct SessionState {
  env: sys::napi_env,
  host: Cell<sys::napi_ref>,
  operations: Vec<SessionOperation>,
  closed: Cell<bool>,
  next_invocation_id: Cell<u32>,
  retained_callbacks: RefCell<BTreeSet<CallbackKey>>,
  input_streams: RefCell<BTreeSet<u32>>,
  resource_references: RefCell<Vec<TrackedResource>>,
  resource_callbacks: ResourceCallbacks,
}

impl SessionState {
  fn host_value(&self) -> Result<sys::napi_value> {
    reference_value(self.env, self.host.get(), "Host")
  }

  fn ensure_open(&self) -> Result<()> {
    if self.closed.get() {
      Err(Error::new(
        Status::GenericFailure,
        "UniFFI backend session is closed",
      ))
    } else {
      Ok(())
    }
  }

  fn operation(&self, id: u32) -> Result<&SessionOperation> {
    self.operations.get(id as usize).ok_or_else(|| {
      Error::new(
        Status::InvalidArg,
        format!("unknown UniFFI operation ID {id}"),
      )
    })
  }

  fn retain_argument_resources(
    &self,
    operation: &SessionOperation,
    args: &[sys::napi_value],
  ) -> Result<()> {
    let receiver_offset = usize::from(operation.receiver.is_some());
    for callback in &operation.callback_arguments {
      let index = receiver_offset + callback.argument_index as usize;
      let value = *args.get(index).ok_or_else(|| {
        Error::new(
          Status::InvalidArg,
          format!("missing callback argument at index {index}"),
        )
      })?;
      let callback_id = value_u32(self.env, value, "callback ID")?;
      if callback.retention == SessionCallbackRetention::Retained {
        let key = CallbackKey {
          callback_type_id: callback.callback_type_id,
          callback_id,
        };
        if self.retained_callbacks.borrow_mut().insert(key) {
          self.call_host(
            "retainCallback",
            &[
              js_u32(self.env, callback.callback_type_id)?,
              js_u32(self.env, callback_id)?,
            ],
          )?;
        }
      }
    }
    for stream in &operation.stream_arguments {
      if stream.direction != SessionStreamDirection::Input {
        continue;
      }
      let index = receiver_offset + stream.argument_index as usize;
      let value = *args.get(index).ok_or_else(|| {
        Error::new(
          Status::InvalidArg,
          format!("missing input-stream argument at index {index}"),
        )
      })?;
      self
        .input_streams
        .borrow_mut()
        .insert(value_u32(self.env, value, "input stream ID")?);
    }
    if let Some(receiver) = operation.receiver {
      let resource = *args
        .first()
        .ok_or_else(|| Error::new(Status::InvalidArg, "missing resource receiver"))?;
      self.retain_resource(resource, receiver)?;
    }
    Ok(())
  }

  fn retain_resource(
    &self,
    resource: sys::napi_value,
    kind: SessionResourceReceiver,
  ) -> Result<()> {
    let already_tracked = self.resource_references.borrow().iter().any(|tracked| {
      tracked.kind == kind
        && reference_value(self.env, tracked.reference, "resource lease")
          .ok()
          .is_some_and(|existing| strict_equals(self.env, existing, resource))
    });
    if !already_tracked {
      self.resource_references.borrow_mut().push(TrackedResource {
        reference: create_reference(self.env, resource, "resource lease")?,
        kind,
      });
    }
    Ok(())
  }

  fn release_resource(
    &self,
    resource: sys::napi_value,
    kind: SessionResourceReceiver,
    cancel: bool,
  ) -> Result<Option<sys::napi_value>> {
    let callback = match (kind, cancel) {
      (SessionResourceReceiver::Object, _) => self.resource_callbacks.release_object.get(),
      (SessionResourceReceiver::OutputStream, true) => {
        self.resource_callbacks.cancel_output_stream.get()
      }
      (SessionResourceReceiver::OutputStream, false) => {
        self.resource_callbacks.release_output_stream.get()
      }
    };
    let result = if callback.is_null() {
      None
    } else {
      let callback = reference_value(self.env, callback, "resource callback")?;
      let handle = named_property(self.env, resource, "handle")?;
      Some(call_function(self.env, resource, callback, &[handle])?)
    };
    let mut refs = self.resource_references.borrow_mut();
    if let Some(index) = refs.iter().position(|tracked| {
      tracked.kind == kind
        && reference_value(self.env, tracked.reference, "resource lease")
          .ok()
          .is_some_and(|existing| strict_equals(self.env, existing, resource))
    }) {
      let tracked = refs.swap_remove(index);
      delete_reference(self.env, tracked.reference);
    }
    Ok(result)
  }

  fn call_host(&self, method: &str, args: &[sys::napi_value]) -> Result<sys::napi_value> {
    let host = self.host_value()?;
    call_named(self.env, host, method, args)
  }

  fn dispatch(
    &self,
    operation_id: u32,
    args: Vec<sys::napi_value>,
    asynchronous: bool,
    this: sys::napi_value,
  ) -> Result<sys::napi_value> {
    self.ensure_open()?;
    let operation = self.operation(operation_id)?;
    let is_async = matches!(
      operation.dispatch,
      SessionOperationDispatch::NativeAsync
        | SessionOperationDispatch::CallbackHostAsync { .. }
        | SessionOperationDispatch::InputStreamHostPull
        | SessionOperationDispatch::InputStreamHostCancel
    );
    if asynchronous != is_async {
      return Err(Error::new(
        Status::InvalidArg,
        format!(
          "operation {operation_id} is {}, but was invoked through {}",
          if is_async { "async" } else { "sync" },
          if asynchronous {
            "invokeAsync"
          } else {
            "invokeSync"
          }
        ),
      ));
    }
    self.retain_argument_resources(operation, &args)?;

    match operation.dispatch {
      SessionOperationDispatch::NativeSync | SessionOperationDispatch::NativeAsync => {
        let callback = reference_value(self.env, operation.callback.get(), "operation callback")?;
        let mut args = args;
        if operation.receiver.is_some() {
          args[0] = named_property(self.env, args[0], "handle")?;
        }
        let mut native_args = Vec::with_capacity(
          args.len() + usize::from(operation.native_call == SessionNativeCall::HostAndArguments),
        );
        if operation.native_call == SessionNativeCall::HostAndArguments {
          native_args.push(self.host_value()?);
        }
        native_args.extend(args);
        call_function(self.env, this, callback, &native_args)
      }
      SessionOperationDispatch::CallbackHostSync {
        callback_type_id,
        method_id,
      } => self.dispatch_callback_host(callback_type_id, method_id, args, false),
      SessionOperationDispatch::CallbackHostAsync {
        callback_type_id,
        method_id,
      } => self.dispatch_callback_host(callback_type_id, method_id, args, true),
      SessionOperationDispatch::InputStreamHostPull => {
        let stream_id = *args
          .first()
          .ok_or_else(|| Error::new(Status::InvalidArg, "pull requires stream ID"))?;
        self.call_host("pullInputStream", &[stream_id])
      }
      SessionOperationDispatch::InputStreamHostCancel => {
        if args.is_empty() {
          return Err(Error::new(Status::InvalidArg, "cancel requires stream ID"));
        }
        self.call_host("cancelInputStream", &args)
      }
    }
  }

  fn dispatch_callback_host(
    &self,
    callback_type_id: u32,
    method_id: u32,
    args: Vec<sys::napi_value>,
    asynchronous: bool,
  ) -> Result<sys::napi_value> {
    let callback_id = *args.first().ok_or_else(|| {
      Error::new(
        Status::InvalidArg,
        "callback invocation requires callback ID",
      )
    })?;
    let callback_args = js_array(self.env, &args[1..])?;
    let mut host_args = vec![
      js_u32(self.env, callback_type_id)?,
      callback_id,
      js_u32(self.env, method_id)?,
    ];
    let method = if asynchronous {
      let invocation_id = self.next_invocation_id.get();
      self
        .next_invocation_id
        .set(invocation_id.checked_add(1).unwrap_or(u32::MAX));
      host_args.push(js_u32(self.env, invocation_id)?);
      "invokeCallbackAsync"
    } else {
      "invokeCallbackSync"
    };
    host_args.push(callback_args);
    self.call_host(method, &host_args)
  }

  fn close(&self) {
    if self.closed.replace(true) {
      return;
    }
    let callbacks = std::mem::take(&mut *self.retained_callbacks.borrow_mut());
    for callback in callbacks {
      if let (Ok(callback_type), Ok(callback_id)) = (
        js_u32(self.env, callback.callback_type_id),
        js_u32(self.env, callback.callback_id),
      ) {
        let _ = self.call_host("releaseCallback", &[callback_type, callback_id]);
      }
    }
    let streams = std::mem::take(&mut *self.input_streams.borrow_mut());
    for stream_id in streams {
      if let Ok(stream_id) = js_u32(self.env, stream_id) {
        let _ = self.call_host("releaseInputStream", &[stream_id]);
      }
    }
    let resources = self
      .resource_references
      .borrow()
      .iter()
      .filter_map(|tracked| {
        reference_value(self.env, tracked.reference, "resource lease")
          .ok()
          .map(|value| (value, tracked.kind))
      })
      .collect::<Vec<_>>();
    for (resource, kind) in resources {
      let _ = self.release_resource(resource, kind, false);
    }
  }

  fn cleanup_references(&self) {
    self.close();
    for operation in &self.operations {
      let reference = operation.callback.replace(ptr::null_mut());
      delete_reference(self.env, reference);
    }
    let host = self.host.replace(ptr::null_mut());
    delete_reference(self.env, host);
    for callback in [
      &self.resource_callbacks.release_object,
      &self.resource_callbacks.cancel_output_stream,
      &self.resource_callbacks.release_output_stream,
    ] {
      delete_reference(self.env, callback.replace(ptr::null_mut()));
    }
  }
}

/// Build the session object returned by the sole generated factory export.
pub fn create_backend_session(
  env: &Env,
  host: Object<'static>,
  flavor: &str,
  descriptors: Vec<SessionOperationDescriptor>,
  resource_callbacks: SessionResourceCallbacks,
) -> Result<Object<'static>> {
  let host_reference = create_reference(env.raw(), host.raw(), "Host")?;
  let mut operations = Vec::with_capacity(descriptors.len());
  for (id, descriptor) in descriptors.into_iter().enumerate() {
    let callback = match (descriptor.dispatch, descriptor.callback) {
      (
        SessionOperationDispatch::NativeSync | SessionOperationDispatch::NativeAsync,
        Some(value),
      ) => create_reference(env.raw(), value, "operation callback")?,
      (SessionOperationDispatch::NativeSync | SessionOperationDispatch::NativeAsync, None) => {
        return Err(Error::new(
          Status::InvalidArg,
          format!("native UniFFI operation slot {id} has no callback"),
        ));
      }
      (_, Some(_)) => {
        return Err(Error::new(
          Status::InvalidArg,
          format!("host UniFFI operation slot {id} unexpectedly has a native callback"),
        ));
      }
      (_, None) => ptr::null_mut(),
    };
    operations.push(SessionOperation {
      dispatch: descriptor.dispatch,
      callback: Cell::new(callback),
      native_call: descriptor.native_call,
      receiver: descriptor.receiver,
      callback_arguments: descriptor.callback_arguments,
      stream_arguments: descriptor.stream_arguments,
    });
  }

  let state = Box::new(SessionState {
    env: env.raw(),
    host: Cell::new(host_reference),
    operations,
    closed: Cell::new(false),
    next_invocation_id: Cell::new(0),
    retained_callbacks: RefCell::new(BTreeSet::new()),
    input_streams: RefCell::new(BTreeSet::new()),
    resource_references: RefCell::new(Vec::new()),
    resource_callbacks: ResourceCallbacks {
      release_object: Cell::new(optional_reference(
        env.raw(),
        resource_callbacks.release_object,
        "object release callback",
      )?),
      cancel_output_stream: Cell::new(optional_reference(
        env.raw(),
        resource_callbacks.cancel_output_stream,
        "output-stream cancel callback",
      )?),
      release_output_stream: Cell::new(optional_reference(
        env.raw(),
        resource_callbacks.release_output_stream,
        "output-stream release callback",
      )?),
    },
  });
  let mut session = Object::new(env)?;
  let raw_session = session.raw();
  let state = Box::into_raw(state);
  napi::check_status!(unsafe {
    sys::napi_wrap(
      env.raw(),
      raw_session,
      state.cast(),
      Some(finalize_session),
      ptr::null_mut(),
      ptr::null_mut(),
    )
  })?;
  session.set("hostFlavor", flavor)?;
  add_method(env.raw(), raw_session, "invokeSync", session_invoke_sync)?;
  add_method(env.raw(), raw_session, "invokeAsync", session_invoke_async)?;
  add_method(
    env.raw(),
    raw_session,
    "releaseObject",
    session_release_object,
  )?;
  add_method(
    env.raw(),
    raw_session,
    "cancelOutputStream",
    session_cancel_output_stream,
  )?;
  add_method(
    env.raw(),
    raw_session,
    "releaseOutputStream",
    session_release_output_stream,
  )?;
  add_method(env.raw(), raw_session, "close", session_close)?;
  Ok(session)
}

unsafe extern "C" fn finalize_session(_env: sys::napi_env, data: *mut c_void, _hint: *mut c_void) {
  if !data.is_null() {
    let state = unsafe { Box::<SessionState>::from_raw(data.cast()) };
    state.cleanup_references();
  }
}

unsafe extern "C" fn session_invoke_sync(
  env: sys::napi_env,
  info: sys::napi_callback_info,
) -> sys::napi_value {
  callback_result(env, || {
    let (this, args) = callback_args(env, info, 2)?;
    let operation_id = value_u32(env, args[0], "operation ID")?;
    let operation_args = array_values(env, args[1])?;
    state(env, this)?.dispatch(operation_id, operation_args, false, this)
  })
}

unsafe extern "C" fn session_invoke_async(
  env: sys::napi_env,
  info: sys::napi_callback_info,
) -> sys::napi_value {
  callback_result(env, || {
    let (this, args) = callback_args(env, info, 2)?;
    let operation_id = value_u32(env, args[0], "operation ID")?;
    let operation_args = array_values(env, args[1])?;
    state(env, this)?.dispatch(operation_id, operation_args, true, this)
  })
}

unsafe extern "C" fn session_release_object(
  env: sys::napi_env,
  info: sys::napi_callback_info,
) -> sys::napi_value {
  // The ABI requires release to be non-throwing and idempotent.  Invalid or
  // already-released leases therefore collapse to a no-op.
  let result = (|| {
    let (this, args) = callback_args(env, info, 1)?;
    let _ = state(env, this)?.release_resource(args[0], SessionResourceReceiver::Object, false);
    js_undefined(env)
  })();
  result.unwrap_or_else(|_| js_undefined(env).unwrap_or(ptr::null_mut()))
}

unsafe extern "C" fn session_cancel_output_stream(
  env: sys::napi_env,
  info: sys::napi_callback_info,
) -> sys::napi_value {
  callback_result(env, || {
    let (this, args) = callback_args(env, info, 1)?;
    Ok(
      state(env, this)?
        .release_resource(args[0], SessionResourceReceiver::OutputStream, true)?
        .unwrap_or(resolved_promise(env)?),
    )
  })
}

unsafe extern "C" fn session_release_output_stream(
  env: sys::napi_env,
  info: sys::napi_callback_info,
) -> sys::napi_value {
  let result = (|| {
    let (this, args) = callback_args(env, info, 1)?;
    let _ =
      state(env, this)?.release_resource(args[0], SessionResourceReceiver::OutputStream, false);
    js_undefined(env)
  })();
  result.unwrap_or_else(|_| js_undefined(env).unwrap_or(ptr::null_mut()))
}

unsafe extern "C" fn session_close(
  env: sys::napi_env,
  info: sys::napi_callback_info,
) -> sys::napi_value {
  callback_result(env, || {
    let (this, _) = callback_args(env, info, 0)?;
    state(env, this)?.close();
    resolved_promise(env)
  })
}

fn state(env: sys::napi_env, this: sys::napi_value) -> Result<&'static SessionState> {
  let mut data = ptr::null_mut();
  napi::check_status!(unsafe { sys::napi_unwrap(env, this, &mut data) })?;
  if data.is_null() {
    return Err(Error::new(Status::GenericFailure, "invalid UniFFI session"));
  }
  Ok(unsafe { &*data.cast::<SessionState>() })
}

fn callback_args(
  env: sys::napi_env,
  info: sys::napi_callback_info,
  expected: usize,
) -> Result<(sys::napi_value, Vec<sys::napi_value>)> {
  let mut argc = expected;
  let mut args = vec![ptr::null_mut(); expected];
  let mut this = ptr::null_mut();
  napi::check_status!(unsafe {
    sys::napi_get_cb_info(
      env,
      info,
      &mut argc,
      args.as_mut_ptr(),
      &mut this,
      ptr::null_mut(),
    )
  })?;
  if argc != expected {
    return Err(Error::new(
      Status::InvalidArg,
      format!("expected {expected} arguments, received {argc}"),
    ));
  }
  Ok((this, args))
}

fn array_values(env: sys::napi_env, array: sys::napi_value) -> Result<Vec<sys::napi_value>> {
  let mut is_array = false;
  napi::check_status!(unsafe { sys::napi_is_array(env, array, &mut is_array) })?;
  if !is_array {
    return Err(Error::new(
      Status::InvalidArg,
      "UniFFI invocation arguments must be an array",
    ));
  }
  let mut length = 0;
  napi::check_status!(unsafe { sys::napi_get_array_length(env, array, &mut length) })?;
  let mut result = Vec::with_capacity(length as usize);
  for index in 0..length {
    let mut value = ptr::null_mut();
    napi::check_status!(unsafe { sys::napi_get_element(env, array, index, &mut value) })?;
    result.push(value);
  }
  Ok(result)
}

fn add_method(
  env: sys::napi_env,
  object: sys::napi_value,
  name: &str,
  callback: unsafe extern "C" fn(sys::napi_env, sys::napi_callback_info) -> sys::napi_value,
) -> Result<()> {
  let name = CString::new(name)?;
  let mut function = ptr::null_mut();
  napi::check_status!(unsafe {
    sys::napi_create_function(
      env,
      name.as_ptr(),
      name.as_bytes().len() as isize,
      Some(callback),
      ptr::null_mut(),
      &mut function,
    )
  })?;
  napi::check_status!(unsafe { sys::napi_set_named_property(env, object, name.as_ptr(), function) })
}

fn call_named(
  env: sys::napi_env,
  this: sys::napi_value,
  name: &str,
  args: &[sys::napi_value],
) -> Result<sys::napi_value> {
  let name = CString::new(name)?;
  let mut function = ptr::null_mut();
  napi::check_status!(unsafe {
    sys::napi_get_named_property(env, this, name.as_ptr(), &mut function)
  })?;
  let mut value_type = sys::ValueType::napi_undefined;
  napi::check_status!(unsafe { sys::napi_typeof(env, function, &mut value_type) })?;
  if value_type != sys::ValueType::napi_function {
    return Err(Error::new(
      Status::InvalidArg,
      format!("Host.{name:?} is not callable"),
    ));
  }
  call_function(env, this, function, args)
}

fn named_property(
  env: sys::napi_env,
  object: sys::napi_value,
  name: &str,
) -> Result<sys::napi_value> {
  let name = CString::new(name)?;
  let mut value = ptr::null_mut();
  napi::check_status!(unsafe {
    sys::napi_get_named_property(env, object, name.as_ptr(), &mut value)
  })?;
  Ok(value)
}

fn call_function(
  env: sys::napi_env,
  this: sys::napi_value,
  function: sys::napi_value,
  args: &[sys::napi_value],
) -> Result<sys::napi_value> {
  let mut result = ptr::null_mut();
  napi::check_status!(unsafe {
    sys::napi_call_function(env, this, function, args.len(), args.as_ptr(), &mut result)
  })?;
  Ok(result)
}

fn create_reference(
  env: sys::napi_env,
  value: sys::napi_value,
  role: &str,
) -> Result<sys::napi_ref> {
  let mut reference = ptr::null_mut();
  napi::check_status!(
    unsafe { sys::napi_create_reference(env, value, 1, &mut reference) },
    "failed to retain {role}"
  )?;
  Ok(reference)
}

fn optional_reference(
  env: sys::napi_env,
  value: Option<sys::napi_value>,
  role: &str,
) -> Result<sys::napi_ref> {
  value
    .map(|value| create_reference(env, value, role))
    .transpose()
    .map(|value| value.unwrap_or(ptr::null_mut()))
}

fn reference_value(
  env: sys::napi_env,
  reference: sys::napi_ref,
  role: &str,
) -> Result<sys::napi_value> {
  if reference.is_null() {
    return Err(Error::new(
      Status::GenericFailure,
      format!("{role} reference is released"),
    ));
  }
  let mut value = ptr::null_mut();
  napi::check_status!(
    unsafe { sys::napi_get_reference_value(env, reference, &mut value) },
    "failed to resolve {role}"
  )?;
  Ok(value)
}

fn delete_reference(env: sys::napi_env, reference: sys::napi_ref) {
  if !reference.is_null() {
    let _ = unsafe { sys::napi_delete_reference(env, reference) };
  }
}

fn strict_equals(env: sys::napi_env, left: sys::napi_value, right: sys::napi_value) -> bool {
  let mut result = false;
  (unsafe { sys::napi_strict_equals(env, left, right, &mut result) }) == sys::Status::napi_ok
    && result
}

fn value_u32(env: sys::napi_env, value: sys::napi_value, role: &str) -> Result<u32> {
  let mut result = 0;
  napi::check_status!(
    unsafe { sys::napi_get_value_uint32(env, value, &mut result) },
    "{role} must be an unsigned 32-bit integer"
  )?;
  Ok(result)
}

fn js_u32(env: sys::napi_env, value: u32) -> Result<sys::napi_value> {
  let mut result = ptr::null_mut();
  napi::check_status!(unsafe { sys::napi_create_uint32(env, value, &mut result) })?;
  Ok(result)
}

fn js_array(env: sys::napi_env, values: &[sys::napi_value]) -> Result<sys::napi_value> {
  let mut result = ptr::null_mut();
  napi::check_status!(unsafe {
    sys::napi_create_array_with_length(env, values.len(), &mut result)
  })?;
  for (index, value) in values.iter().copied().enumerate() {
    napi::check_status!(unsafe { sys::napi_set_element(env, result, index as u32, value) })?;
  }
  Ok(result)
}

fn js_undefined(env: sys::napi_env) -> Result<sys::napi_value> {
  let mut value = ptr::null_mut();
  napi::check_status!(unsafe { sys::napi_get_undefined(env, &mut value) })?;
  Ok(value)
}

fn resolved_promise(env: sys::napi_env) -> Result<sys::napi_value> {
  let mut deferred = ptr::null_mut();
  let mut promise = ptr::null_mut();
  napi::check_status!(unsafe { sys::napi_create_promise(env, &mut deferred, &mut promise) })?;
  napi::check_status!(unsafe { sys::napi_resolve_deferred(env, deferred, js_undefined(env)?) })?;
  Ok(promise)
}

fn callback_result(
  env: sys::napi_env,
  callback: impl FnOnce() -> Result<sys::napi_value>,
) -> sys::napi_value {
  match callback() {
    Ok(value) => value,
    Err(error) => {
      let reason = CString::new(error.reason)
        .unwrap_or_else(|_| CString::new("UniFFI backend error").expect("literal is valid"));
      let _ = unsafe { sys::napi_throw_error(env, ptr::null(), reason.as_ptr()) };
      ptr::null_mut()
    }
  }
}
