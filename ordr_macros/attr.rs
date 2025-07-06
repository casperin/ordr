//! Parse the attributes part of calling the `node` macro.

use syn::{LitStr, Type, meta::ParseNestedMeta};

/// Describes the attibutes in a node(...) macro
#[derive(Default)]
pub(super) struct Attr {
    /// The name of the node
    pub(super) name: Option<String>,
    /// The type of the output
    pub(super) out: Option<Type>,
    /// The type of the input state
    pub(super) state: Option<Type>,
}

impl Attr {
    /// Parses the attributes on a node(...)
    pub(super) fn parse(&mut self, meta: &ParseNestedMeta) -> syn::Result<()> {
        // name = "..."
        if meta.path.is_ident("name") {
            let lit: LitStr = meta.value()?.parse()?;
            self.name = Some(lit.value());
            return Ok(());
        }

        if meta.path.is_ident("output") {
            let ty: syn::Type = meta.value()?.parse()?;
            self.out = Some(ty);
            return Ok(());
        }

        if meta.path.is_ident("state") {
            let ty: syn::Type = meta.value()?.parse()?;
            self.state = Some(ty);
            return Ok(());
        }

        Err(meta.error("unknown key in `node(...)`, expected one of: name or output"))
    }
}

#[cfg(test)]
mod tests {
    use super::Attr;
    use quote::ToTokens;
    use syn::{meta::ParseNestedMeta, parse::Parser, parse_quote};

    /// Helper: run `#[node(...)]`‐style args through your parser
    /// and return the filled-in `NodeArgs`.
    fn parse_args(args: proc_macro2::TokenStream) -> Attr {
        let mut attr = Attr::default();
        // Build the meta‐parser closure
        let parser = syn::meta::parser(|meta: ParseNestedMeta| attr.parse(&meta));
        // Actually run it over our token‐stream
        parser.parse2(args).expect("failed to parse node args");
        attr
    }

    #[test]
    fn test_parse_results_name_output() {
        let args = parse_quote! { name = "foo", output = Foo, state = State };
        let args = parse_args(args);

        assert_eq!(args.out.into_token_stream().to_string(), "Foo");
        assert_eq!(args.name.as_deref(), Some("foo"));
        assert_eq!(args.state.into_token_stream().to_string(), "State");
    }
}
