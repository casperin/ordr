mod attr;
mod input_output;

use attr::Attr;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Ident, ItemFn, Type, parse_macro_input, spanned::Spanned};

#[proc_macro_attribute]
pub fn producer(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let func_ident = &func.sig.ident;

    let mut attr = Attr::default();
    let parser = syn::meta::parser(|meta| attr.parse(&meta));
    parse_macro_input!(attrs with parser);

    let mut dep_tys = input_output::input(&func.sig);
    let _ctx_ty = dep_tys.remove(0); // First one is the Context argument

    let node_ty = attr.out.unwrap(); // TODO
    let state_ty = attr.state.unwrap(); // TODO

    let node_name = attr.name.unwrap_or_else(|| ty_to_string(&node_ty));
    let dep_idents: Vec<_> = dep_tys.iter().map(ty_to_ident).collect();

    quote! {
        #func

        impl ordr::NodeBuilder<#state_ty> for #node_ty {
            fn node() -> ordr::Node<#state_ty> {
                ordr::Node {
                    id: std::any::TypeId::of::<#node_ty>(),
                    name: #node_name,
                    deps: std::sync::Arc::new(|| {
                        vec![
                            #(
                                #dep_tys::node()
                            ),*
                        ]
                    }),
                    producer: std::sync::Arc::new(|context, payloads| {
                        let [ #(#dep_idents),* ] = payloads.try_into().unwrap();
                        let ( #(#dep_idents),* ) = (
                            #(
                                ordr::serde_cbor::from_slice(&#dep_idents).unwrap()
                                // #dep_idents.downcast_ref::<#dep_tys>().unwrap()
                            ),*
                        );

                        Box::pin(async move {
                            let result = match #func_ident(context, #(#dep_idents),* ).await {
                                Ok(result) => result,
                                Err(e) => return Err(e),
                            };

                            let v = ordr::serde_cbor::to_vec(&result).unwrap();
                            Ok(v)
                        })
                    })
                }
            }
        }
    }
    .into()
}

fn ty_to_string(ty: &Type) -> String {
    let Type::Path(type_path) = ty else {
        panic!("{ty:?} has no path")
    };
    type_path.path.segments.last().unwrap().ident.to_string()
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
