//! Parse the execute function.

use syn::{FnArg, GenericArgument, PathArguments, Signature, Type, TypePath, spanned::Spanned};

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
