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

use macros::error_spanned;

pub fn attribute_name_to_bytes<'c>(attr: &Attribute) -> Option<&'c [u8]> {
    let segments = attr.meta.path().segments.iter().rev();
    segments.last().map(| segment | steal(segment.ident.to_string().as_bytes()))
}

pub fn parse_group_with_delim(delim: Delimiter, input: ParseStream) -> Result<TokenStream> {
    input.step(| cursor | {
        if let Some((content, _, next)) = cursor.group(delim) {
            return Ok((content.token_stream(), next));
        }

        Err(cursor.error(format!("Expected delimiter: {:?}", delim)))
    })
}

pub fn greedy_parse_with_delim<T, D>(input: ParseStream) -> Result<Vec<T>> where
    T: Parse,
    D: Parse
{
    let mut out: Vec<T> = Vec::with_capacity(1);
    while !input.is_empty() {
        out.push(input.parse::<T>()?);
        if !input.is_empty() {
            input.parse::<D>()?;
        }
    }

    Ok(out)
}

pub fn peek_next_tt(input: ParseStream) -> Result<TokenTree> {
    if let Some((tt, _)) = input.cursor().token_tree() {
        return Ok(tt);
    }

    Err(error_spanned!("{} ^ expected token", &input.cursor().token_stream()))
}

pub fn parse_next_tt(input: ParseStream) -> Result<TokenTree> {
    input.step(| cursor | {
        if let Some((tt, next)) = cursor.token_tree() {
            return Ok((tt, next));
        }

        Err(error_spanned!("{} ^ expected token", &cursor.token_stream()))
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