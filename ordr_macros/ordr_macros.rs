mod attr;
mod input_output;

use attr::Attr;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, ItemFn, ReturnType, Type, parse_macro_input, spanned::Spanned};

/// Mark a function return a `Result<T, ordr::Error>` as a producer of `T`.
///
/// # Panics
/// There are several more or less implicit rules that the function (and output) needs to abide by.
/// If any of them are violated, we panic with a hopefully good error message.
#[proc_macro_attribute]
pub fn producer(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let func_ident = &func.sig.ident;

    let mut attr = Attr::default();
    let parser = syn::meta::parser(|meta| attr.parse(&meta));
    parse_macro_input!(attrs with parser);

    let mut dep_tys = input_output::input(&func.sig);
    let context_ty = dep_tys.remove(0); // First one is the Context argument

    let node_ty = match (attr.out, &func.sig.output) {
        (Some(ty), _) => ty,
        (None, ReturnType::Default) => panic!("The producer function must return a Result<T>"),
        (None, ReturnType::Type(_, box_ty)) => input_output::first_generic(box_ty),
    };

    let state_ty = attr
        .state
        .unwrap_or_else(|| input_output::first_generic(&context_ty));

    let node_name = attr.name.unwrap_or_else(|| ty_to_string(&node_ty));

    let mut dep_idents = vec![];
    for ty in &dep_tys {
        let Type::Path(type_path) = ty else {
            panic!("{ty:?} has no path")
        };
        for seg in &type_path.path.segments {
            if !seg.arguments.is_empty() {
                let e = syn::Error::new(
                    type_path.span(),
                    "Arguments to producer functions cannot take generics. Use a type alias instead.",
                );
                panic!("{e}");
            }
        }
        let seg = type_path.path.segments.last().unwrap();
        let str = seg.ident.to_string().to_lowercase();
        let ident = Ident::new(&str, seg.ident.span());
        dep_idents.push(ident);
    }

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
                                ordr::serde_json::from_value(#dep_idents).unwrap()
                            ),*
                        );
                        Box::pin(async move {
                            let result = match #func_ident(context, #(#dep_idents),* ).await {
                                Ok(result) => result,
                                Err(e) => return Err(e),
                            };
                            let v = ordr::serde_json::to_value(result).unwrap();
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
