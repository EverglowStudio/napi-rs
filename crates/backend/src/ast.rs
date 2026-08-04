use convert_case::Case;
use proc_macro2::{Ident, Literal, Span};
use syn::{Attribute, Expr, Type};

#[derive(Debug, Clone)]
pub struct NapiFn {
  pub name: Ident,
  pub js_name: String,
  pub module_exports: bool,
  pub attrs: Vec<Attribute>,
  pub args: Vec<NapiFnArg>,
  pub ret: Option<syn::Type>,
  pub is_ret_result: bool,
  pub is_async: bool,
  pub within_async_runtime: bool,
  pub fn_self: Option<FnSelf>,
  pub kind: FnKind,
  pub vis: syn::Visibility,
  pub parent: Option<Ident>,
  pub parent_js_name: Option<String>,
  pub strict: bool,
  pub return_if_invalid: bool,
  pub js_mod: Option<String>,
  pub ts_generic_types: Option<String>,
  pub ts_type: Option<String>,
  pub ts_args_type: Option<String>,
  pub ts_return_type: Option<String>,
  pub skip_typescript: bool,
  pub comments: Vec<String>,
  pub parent_is_generator: bool,
  pub parent_is_async_generator: bool,
  pub writable: bool,
  pub enumerable: bool,
  pub configurable: bool,
  pub catch_unwind: bool,
  pub unsafe_: bool,
  pub register_name: Ident,
  pub no_export: bool,
}

/// Programmatic construction API for [`NapiFn`].
///
/// The procedural macro parser remains the source of its existing defaults.
/// External frontends can use this builder to opt into the same backend AST
/// without manufacturing a synthetic annotated Rust item and parsing it
/// again.  Builder methods deliberately expose backend mechanics rather than
/// frontend semantics.
#[derive(Debug, Clone)]
pub struct NapiFnBuilder {
  function: NapiFn,
}

impl NapiFnBuilder {
  pub fn new(name: Ident, js_name: impl Into<String>) -> Self {
    let register_name = Ident::new(&format!("__napi_register_{}", name), Span::call_site());
    Self {
      function: NapiFn {
        name,
        js_name: js_name.into(),
        module_exports: false,
        attrs: Vec::new(),
        args: Vec::new(),
        ret: None,
        is_ret_result: false,
        is_async: false,
        within_async_runtime: false,
        fn_self: None,
        kind: FnKind::Normal,
        vis: syn::Visibility::Inherited,
        parent: None,
        parent_js_name: None,
        strict: false,
        return_if_invalid: false,
        js_mod: None,
        ts_generic_types: None,
        ts_type: None,
        ts_args_type: None,
        ts_return_type: None,
        skip_typescript: false,
        comments: Vec::new(),
        parent_is_generator: false,
        parent_is_async_generator: false,
        writable: true,
        enumerable: true,
        configurable: true,
        catch_unwind: false,
        unsafe_: false,
        register_name,
        no_export: false,
      },
    }
  }

  pub fn argument(mut self, argument: NapiFnArg) -> Self {
    self.function.args.push(argument);
    self
  }

  pub fn return_type(mut self, return_type: Type) -> Self {
    self.function.ret = Some(return_type);
    self.function.is_ret_result = false;
    self
  }

  pub fn result_return_type(mut self, return_type: Type) -> Self {
    self.function.ret = Some(return_type);
    self.function.is_ret_result = true;
    self
  }

  pub fn asynchronous(mut self, asynchronous: bool) -> Self {
    self.function.is_async = asynchronous;
    self
  }

  pub fn within_async_runtime(mut self, within_async_runtime: bool) -> Self {
    self.function.within_async_runtime = within_async_runtime;
    self
  }

  pub fn strict(mut self, strict: bool) -> Self {
    self.function.strict = strict;
    self
  }

  pub fn skip_typescript(mut self, skip_typescript: bool) -> Self {
    self.function.skip_typescript = skip_typescript;
    self
  }

  /// Keep the generated N-API callback available to sibling generated code,
  /// but do not register it in the module export table.
  pub fn private(mut self, private: bool) -> Self {
    self.function.no_export = private;
    self
  }

  pub fn register_name(mut self, register_name: Ident) -> Self {
    self.function.register_name = register_name;
    self
  }

  pub fn build(self) -> NapiFn {
    self.function
  }
}

#[derive(Debug, Clone)]
pub struct CallbackArg {
  pub pat: Box<syn::Pat>,
  pub args: Vec<syn::Type>,
  pub ret: Option<syn::Type>,
}

#[derive(Debug, Clone)]
pub struct NapiFnArg {
  pub kind: NapiFnArgKind,
  pub ts_arg_type: Option<String>,
}

impl NapiFnArg {
  /// if type was overridden with `#[napi(ts_arg_type = "...")]` use that instead
  pub fn use_overridden_type_or(&self, default: impl FnOnce() -> String) -> String {
    self.ts_arg_type.as_ref().cloned().unwrap_or_else(default)
  }
}

#[derive(Debug, Clone)]
pub enum NapiFnArgKind {
  PatType(Box<syn::PatType>),
  Callback(Box<CallbackArg>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FnKind {
  Normal,
  Constructor,
  Factory,
  Getter,
  Setter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FnSelf {
  Value,
  Ref,
  MutRef,
}

#[derive(Debug, Clone)]
pub struct NapiStruct {
  pub name: Ident,
  pub js_name: String,
  pub comments: Vec<String>,
  pub js_mod: Option<String>,
  pub use_nullable: bool,
  pub register_name: Ident,
  pub kind: NapiStructKind,
  pub has_lifetime: bool,
  pub is_generator: bool,
  pub is_async_generator: bool,
  /// Optional crate-unique salt from `#[napi(type_tag = "...")]`. When set on a
  /// class it REPLACES the default `crate@version` identity component of the
  /// content-derived class type tag (module_path + ClassName still apply), so a
  /// class's tag cannot collide with an unrelated addon that happens to share
  /// the same crate name@version + module path + class name. `None` keeps the
  /// default `crate@version::module_path::ClassName` derivation. Runtime-only;
  /// never emitted into TypeScript.
  pub type_tag: Option<String>,
}

#[derive(Debug, Clone)]
pub enum NapiStructKind {
  Transparent(NapiTransparent),
  Class(NapiClass),
  Object(NapiObject),
  StructuredEnum(NapiStructuredEnum),
  Array(NapiArray),
}

#[derive(Debug, Clone)]
pub struct NapiTransparent {
  pub ty: Type,
  pub object_from_js: bool,
  pub object_to_js: bool,
}

#[derive(Debug, Clone)]
pub struct NapiClass {
  pub fields: Vec<NapiStructField>,
  pub ctor: bool,
  pub implement_iterator: bool,
  pub implement_async_iterator: bool,
  pub is_tuple: bool,
  pub use_custom_finalize: bool,
}

#[derive(Debug, Clone)]
pub struct NapiObject {
  pub fields: Vec<NapiStructField>,
  pub object_from_js: bool,
  pub object_to_js: bool,
  pub is_tuple: bool,
}

#[derive(Debug, Clone)]
pub struct NapiArray {
  pub fields: Vec<NapiStructField>,
  pub object_from_js: bool,
  pub object_to_js: bool,
}

#[derive(Debug, Clone)]
pub struct NapiStructuredEnum {
  pub variants: Vec<NapiStructuredEnumVariant>,
  pub object_from_js: bool,
  pub object_to_js: bool,
  pub discriminant: String,
  pub discriminant_case: Option<Case<'static>>,
}

#[derive(Debug, Clone)]
pub struct NapiStructuredEnumVariant {
  pub name: Ident,
  pub fields: Vec<NapiStructField>,
  pub is_tuple: bool,
}

#[derive(Debug, Clone)]
pub struct NapiStructField {
  pub name: syn::Member,
  pub js_name: String,
  pub ty: syn::Type,
  pub getter: bool,
  pub setter: bool,
  pub writable: bool,
  pub enumerable: bool,
  pub configurable: bool,
  pub comments: Vec<String>,
  pub skip_typescript: bool,
  pub ts_type: Option<String>,
  pub has_lifetime: bool,
}

#[derive(Debug, Clone)]
pub struct NapiImpl {
  pub name: Ident,
  pub js_name: String,
  pub has_lifetime: bool,
  pub items: Vec<NapiFn>,
  pub task_output_type: Option<Type>,
  pub iterator_yield_type: Option<Type>,
  pub iterator_next_type: Option<Type>,
  pub iterator_return_type: Option<Type>,
  pub async_iterator_yield_type: Option<Type>,
  pub async_iterator_next_type: Option<Type>,
  pub async_iterator_return_type: Option<Type>,
  pub js_mod: Option<String>,
  pub comments: Vec<String>,
  pub register_name: Ident,
}

#[derive(Debug, Clone)]
pub struct NapiEnum {
  pub name: Ident,
  pub js_name: String,
  pub variants: Vec<NapiEnumVariant>,
  pub js_mod: Option<String>,
  pub comments: Vec<String>,
  pub skip_typescript: bool,
  pub register_name: Ident,
  pub is_string_enum: bool,
  pub object_from_js: bool,
  pub object_to_js: bool,
}

#[derive(Debug, Clone)]
pub enum NapiEnumValue {
  String(String),
  Number(i32),
}

impl From<&NapiEnumValue> for Literal {
  fn from(val: &NapiEnumValue) -> Self {
    match val {
      NapiEnumValue::String(string) => Literal::string(string),
      NapiEnumValue::Number(number) => Literal::i32_unsuffixed(number.to_owned()),
    }
  }
}

#[derive(Debug, Clone)]
pub struct NapiEnumVariant {
  pub name: Ident,
  pub val: NapiEnumValue,
  pub comments: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NapiConst {
  pub name: Ident,
  pub js_name: String,
  pub type_name: Type,
  pub value: Expr,
  pub js_mod: Option<String>,
  pub comments: Vec<String>,
  pub skip_typescript: bool,
  pub register_name: Ident,
}

#[derive(Debug, Clone)]
pub struct NapiMod {
  pub name: Ident,
  pub js_name: String,
}

#[derive(Debug, Clone)]
pub struct NapiType {
  pub name: Ident,
  pub js_name: String,
  pub value: Type,
  pub register_name: Ident,
  pub skip_typescript: bool,
  pub js_mod: Option<String>,
  pub comments: Vec<String>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn programmatic_function_builder_preserves_private_codegen_policy() {
    let function = NapiFnBuilder::new(
      Ident::new("__raw_operation_0", Span::call_site()),
      "__raw_operation_0",
    )
    .argument(NapiFnArg {
      kind: NapiFnArgKind::PatType(Box::new(syn::parse_quote!(value: u32))),
      ts_arg_type: None,
    })
    .return_type(syn::parse_quote!(u32))
    .asynchronous(true)
    .strict(true)
    .skip_typescript(true)
    .private(true)
    .build();

    assert_eq!(function.args.len(), 1);
    assert!(function.is_async);
    assert!(function.strict);
    assert!(function.skip_typescript);
    assert!(function.no_export);
    assert!(function.writable);
    assert!(function.enumerable);
    assert!(function.configurable);
    assert_eq!(function.js_name, "__raw_operation_0");
  }
}
