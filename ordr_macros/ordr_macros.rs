#![warn(missing_docs)]

//! Macros for working with Ordr.
//!
//! See documentation on ordr for examples on how to use these.

mod attr;
mod input_output;

use attr::Attr;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Ident, ItemFn, Type, parse_macro_input};

/// Marks an async function as the executor of a node, meaning it can produce that node/Type.
/// # Example
/// ```
/// #[derive(Clone)]
/// struct A(i32);
///
/// # use ordr_macros::executor;
/// # mod ordr { pub use ordr_core::*; }
/// #[executor]
/// async fn make_a(_: ()) -> Result<A, String> {
///     Ok(A(22))
/// }
/// ```
#[proc_macro_attribute]
pub fn executor(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let func_ident = &func.sig.ident;

    let mut attr = Attr::default();
    let parser = syn::meta::parser(|meta| attr.parse(&meta));
    parse_macro_input!(attrs with parser);

    let mut dep_tys = input_output::input(&func.sig);
    let ctx_ty = dep_tys.remove(0); // First one is the Ctx argument

    let dep_idents: Vec<_> = dep_tys.iter().map(ty_to_ident).collect();

    let (out_ty_ok, out_ty_err) = match (attr.out, attr.err) {
        (Some(out), Some(err)) => (out, err),
        (None, None) => (
            input_output::output(&func.sig, 0),
            input_output::output(&func.sig, 1),
        ),
        (None, Some(err)) => (input_output::output(&func.sig, 0), err),
        (Some(out), None) => (out, input_output::output(&func.sig, 1)),
    };

    let out_name = attr.name.unwrap_or_else(|| ty_to_string(&out_ty_ok));

    quote! {
        #func

        impl ordr::node::NodeBuilder<#ctx_ty, #out_ty_err> for #out_ty_ok {
            fn node() -> ordr::node::Node<#ctx_ty, #out_ty_err> {
                ordr::node::Node {
                    name: #out_name,

                    id: std::any::TypeId::of::<#out_ty_ok>(),

                    deps: vec![
                        #( std::any::TypeId::of::<#dep_tys>(), )*
                    ],

                    prepare: std::sync::Arc::new(|deps| {
                        let [ #(#dep_idents),* ] = deps.try_into().unwrap();

                        let payload_tuple = (
                            #(
                                #dep_idents
                                    .downcast_ref::<#dep_tys>()
                                    .unwrap()
                                    .clone()
                            ),*
                        );

                        Box::new(payload_tuple)
                    }),

                    execute: std::sync::Arc::new(|ctx, payload| {
                        let ( #(#dep_idents),* ) = *payload.downcast().unwrap();

                        Box::pin(async move {
                            #func_ident(ctx, #(#dep_idents),* )
                                .await
                                .map(|result| Box::new(result) as Box<dyn std::any::Any + Send>)
                        })
                    }),
                }
            }
        }
    }
    .into()
}

/// Turn a type into an ident, like `Foo::Bar` -> `bar`
fn ty_to_ident(ty: &Type) -> Ident {
    let Type::Path(type_path) = ty else {
        panic!("{ty:?} has no path")
    };
    let seg = type_path.path.segments.last().unwrap();
    let str = seg.ident.to_string().to_lowercase();
    Ident::new(&str, seg.ident.span())
}

/// Turn a type into an ident, like `Foo::Bar` -> "Bar"
fn ty_to_string(ty: &Type) -> String {
    let Type::Path(type_path) = ty else {
        panic!("{ty:?} has no path")
    };
    type_path.path.segments.last().unwrap().ident.to_string()
}

/// Implements functions on your type, so you can easily get the results out of Outputs.
///
/// # Example
/// ```
/// # mod ordr { pub use ordr_core::*; pub use ordr_macros::*; }
/// use ordr::{build_graph, executor, Output, job::Job, error};
///
/// /// Create our own error. It's a little cumbersome. Maybe you use anyhow?
/// #[derive(Clone, Debug, Eq, PartialEq)]
/// struct Error(&'static str);
///
/// impl std::fmt::Display for Error {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.0)
///     }
/// }
///
/// // A context so we can control the output of `create_bar`.
/// #[derive(Clone)]
/// struct Ctx {
///     init: i32,
///     fail_bar: bool,
/// }
///
/// // A simple node, and its executor.
/// #[derive(Clone, Debug, PartialEq)]
/// struct Foo(i32);
///
/// #[executor]
/// async fn create_foo(ctx: Ctx) -> Result<Foo, Error> {
///     Ok(Foo(ctx.init + 1))
/// }
///
/// // Another node, depending on Foo.
/// #[derive(Clone, Debug, PartialEq)]
/// struct Bar(i32);
///
/// #[executor]
/// async fn create_bar(ctx: Ctx, foo: Foo) -> Result<Bar, Error> {
///     if ctx.fail_bar {
///         return Err(Error("Bar failed"));
///     }
///     Ok(Bar(foo.0 + 10))
/// }
///
/// // Our own results.
/// #[derive(Output, Default)]
/// struct MyResults {
///     foo: Option<Foo>, // These must be `Option<T>`s
///     bar: Option<Bar>,
/// }
///
/// # async {
/// let graph = build_graph!(Foo, Bar).unwrap();
///
/// let job = Job::new().with_target::<Bar>();
///
/// let ctx = Ctx {
///     init: 1,
///     fail_bar: true,
/// };
///
/// let e = graph.execute(job, ctx).await.unwrap_err();
///
/// let error::Error::NodeFailed { outputs, .. } = e else {
///     panic!("Expected bar to fail");
/// };
///
/// // We got the Foo value, but not the bar value.
/// assert_eq!(outputs.get::<Foo>(), Some(&Foo(2)));
/// assert_eq!(outputs.get::<Bar>(), None);
///
/// // We can copy the results from outputs into MyResults
/// let results = MyResults::default().with_output_from(&outputs);
///
/// assert_eq!(results.foo, Some(Foo(22)));
/// assert_eq!(results.bar, None);
///
/// // And we can also create a new job based on this output,
/// // meaning we don't execute `create_foo` again.
/// // We still have to set a target though.
/// let job2: Job<Ctx, Error>  = results.into_job().with_target::<Bar>();
/// let ctx2 = Ctx {
///     init: 100,
///     fail_bar: false,
/// };
/// let outputs = graph.execute(job2, ctx2).await.unwrap();
///
/// // Value of Foo is the same as before.
/// assert_eq!(outputs.get::<Foo>(), Some(&Foo(2)));
///
/// // But this time we got a Bar
/// assert_eq!(outputs.get::<Bar>(), Some(&Bar(12)));
/// # };
///
/// ```
#[proc_macro_derive(Output)]
pub fn output(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let Data::Struct(strct) = &input.data else {
        return Error::new_spanned(&input, "Only named fields supported")
            .to_compile_error()
            .into();
    };

    let name = &input.ident;
    let fields: Vec<_> = strct.fields.iter().map(|f| &f.ident).collect();

    quote! {
        #[automatically_derived]
        impl #name {
            pub fn clone_output_from(&mut self, outputs: &ordr::outputs::Outputs) {
                #(
                    self.#fields = outputs.get().cloned();
                )*
            }

            pub fn with_output_from(mut self, outputs: &ordr::outputs::Outputs) -> Self {
                self.clone_output_from(outputs);
                self
            }

            pub fn into_job<C, E>(self) -> ordr::job::Job<C, E>
            where
                C: Clone + Send + 'static,
                E: Send + 'static + std::fmt::Display
            {
                let mut job = ordr::job::Job::new();
                #(
                    if let Some(v) = &self.#fields {
                        let _ = job.input(v.clone());
                    }
                )*
                job
            }
        }

    }
    .into()
}
