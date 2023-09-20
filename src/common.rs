use proc_macro2::{
    Delimiter,
    TokenStream, TokenTree
};
use syn::{
    Attribute, Result,
    parse::{
        ParseStream, Parse
    }
};

pub fn attribute_name_to_bytes<'c>(attr: &Attribute) -> Option<&'c [u8]> {
    let name: Option<&'c [u8]> = attr.meta.path().get_ident().map(| ident: &syn::Ident | {
        steal(ident.to_string().as_bytes())
    });

    name
}

pub fn parse_delim<'c>(delim: Delimiter, input: ParseStream<'c>) -> Result<TokenStream> {
    input.step(| cursor | {
        if let Some((content, _, next)) = cursor.group(delim) {
            return Ok((content.token_stream(), next));
        }

        Err(cursor.error(format!("Expected delimiter: {:?}", delim)))
    })
}

pub fn greedy_parse_with<T, F, O>(input: ParseStream, after_hook: F) -> Result<Vec<T>> where
    T: Parse,
    F: for<'a> Fn(ParseStream<'a>) -> Result<O>
{
    let mut out: Vec<T> = Vec::with_capacity(1);
    while !input.is_empty() {
        out.push(input.parse::<T>()?);
        if !input.is_empty() {
            after_hook(input)?;
        }
    }

    Ok(out)
}

pub fn peek_next_tt(input: ParseStream) -> Result<TokenTree> {
    input.step(| cursor | {
        if let Some((tt, _)) = cursor.token_tree() {
            return Ok((tt, *cursor));
        }

        Err(cursor.error("Unexpected end of stream: Expected tokens"))
    })
}

#[inline]
pub fn steal<'c, T: ?Sized>(item: &T) -> &'c T {
    unsafe {
        core::mem::transmute::<&T, &'c T>(item)
    }
}

#[macro_use]
pub(crate) mod macros {
    macro_rules! error_spanned {
        ($formatter:literal, $item:expr $(, $other_items:expr )*) => {
            syn::Error::new(syn::spanned::Spanned::span($item), &format!(
                $formatter, quote::ToTokens::to_token_stream($item) $(, quote::ToTokens::to_token_stream($other_items))*
            ))
        };
    }

    macro_rules! unwrap_or_err {
        ($target:expr, $($error:tt)+) => {
            match $target {
                Ok(t) => t,
                Err(e) => {
                    let mut out = $($error)+;
                    out.combine(e);

                    return out.to_compile_error();
                }
            }
        };
    }

    pub(crate) use error_spanned;
    pub(crate) use unwrap_or_err;
}