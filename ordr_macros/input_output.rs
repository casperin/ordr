//! Parse the execute function.

use syn::{
    FnArg, GenericArgument, PathArguments, ReturnType, Signature, Type, TypePath, spanned::Spanned,
};

/// Return a list of the argument names and another of their respective types.
pub(super) fn input(sig: &Signature) -> Vec<Type> {
    sig.inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Typed(p) => Some(*p.ty.clone()),
            FnArg::Receiver(_) => None,
        })
        .collect::<Vec<_>>()
}

/// Given something like `Result<T, E>` or `Context<T>` this function will return `T`.
pub(super) fn first_generic(ty: &Type) -> Type {
    let e = syn::Error::new(
        ty.span(),
        "Expected something like `Result<T, E>`, got {ty:?}",
    );
    let Type::Path(TypePath { path, .. }) = ty else {
        panic!("{ty:?}");
    };
    // Grab the final segment, e.g. "Result<...>" of "std::result::Result<...>"
    let Some(seg) = path.segments.last() else {
        panic!("{e}");
    };
    // Are there angle‐bracketed args?
    let PathArguments::AngleBracketed(ref args) = seg.arguments else {
        panic!("{e}");
    };
    // Take the first of whatever is inside "<...>"
    let GenericArgument::Type(inner_ty) = &args.args[0] else {
        panic!("{e}");
    };
    inner_ty.clone()
}

/// Given a function signature with output of the form `Result<T, E>`, return the inner `T` as a `Type`.
pub(super) fn output(sig: &Signature, i: usize) -> Type {
    let return_ty = &sig.output;
    let e = syn::Error::new(return_ty.span(), "Function must return `Result<T, Error>`");

    // Get return type.
    let ReturnType::Type(_, boxed_ty) = return_ty else {
        panic!("{e}");
    };

    // We expect a path type: something like Result<...>
    let Type::Path(TypePath { path, .. }) = &**boxed_ty else {
        panic!("{e}");
    };

    // Grab the final segment, e.g. "Result<...>"
    let Some(last_seg) = path.segments.last() else {
        panic!("{e}");
    };

    // // Is it literally "Result"?
    assert!(last_seg.ident == "Result", "{e}");

    // Are there angle‐bracketed args?
    let PathArguments::AngleBracketed(ref gen_args) = last_seg.arguments else {
        panic!("{e}");
    };

    let GenericArgument::Type(inner_ty) = &gen_args.args[i] else {
        panic!("{e}");
    };

    inner_ty.clone()
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::{ItemFn, parse_quote};

    use super::*;

    fn make_fn() -> ItemFn {
        parse_quote! {
            async fn exec(_ctx: (), x: i32, y: i32) -> Result<i32, ordr::Error> {
                Ok(x + y)
            }
        }
    }

    #[test]
    fn parse_input() {
        let f = make_fn();
        let types = input(&f.sig);
        assert_eq!(types.len(), 3);
        assert_eq!(types[0].to_token_stream().to_string(), "()");
        assert_eq!(types[1].to_token_stream().to_string(), "i32");
        assert_eq!(types[2].to_token_stream().to_string(), "i32");
    }

    #[test]
    fn parse_output() {
        let f = make_fn();
        let ty = output(&f.sig, 0);
        // Notice that it "unwraps" the Result<i32, Error> to just i32
        assert_eq!(ty.to_token_stream().to_string(), "i32");
    }

    #[test]
    #[should_panic(expected = "Function must return `Result<T, Error>`")]
    fn parse_output_wrong_type() {
        let f: ItemFn = parse_quote! {
            async fn exec(_ctx: (), x: i32, y: i32) -> i32 {
                x + y
            }
        };
        output(&f.sig, 0); // boom
    }

    #[test]
    fn parse_no_name_for_ctx() {
        let f: ItemFn = parse_quote! {
            async fn exec(_: (), x: i32, y: i32) -> Result<i32, ordr::Error> {
                Ok(x + y)
            }
        };
        let fn_args = input(&f.sig);
        assert_eq!(fn_args.len(), 3);
    }
}
