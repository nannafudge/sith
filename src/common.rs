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
    let segments = attr.meta.path().segments.iter().rev();
    segments.last().map(| segment | steal(segment.ident.to_string().as_bytes()))
}

pub fn parse_group_with_delim(delim: Delimiter, input: ParseStream) -> Result<TokenStream> {
    input.step(| cursor | {
        if let Some((content, _, next)) = cursor.group(delim) {
            return Ok((content.token_stream(), next));
        }

        Err(cursor.error(format!("expected delimiter: {:?}", &delim)))
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
    match input.cursor().token_tree() {
        Some((tt, _)) => Ok(tt),
        _ => Err(input.error("expected token"))
    }
}

pub fn parse_next_tt(input: ParseStream) -> Result<TokenTree> {
    input.step(| cursor | {
        cursor.token_tree().ok_or(input.error("expected token"))
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
        ($error:literal, $item:expr) => {
            syn::Error::new(syn::spanned::Spanned::span($item), $error)
        };
        (format!($formatter:literal), $item:expr $(, $other_items:expr )*) => {
            syn::Error::new(syn::spanned::Spanned::span($item), &format!(
                $formatter $(, $other_items)*
            ))
        };
    }

    macro_rules! unwrap_or_err {
        ($target:expr, $($error:tt)+) => {
            if let Err(e) = $target {
                $($error)+.combine(e);
                return $($error)+.to_compile_error();
            } else {
                $target.unwrap()
            }
        };
        ($target:expr) => {
            match $target {
                Ok(t) => t,
                Err(e) => {
                    return e.to_compile_error();
                }
            }
        };
    }

    pub(crate) use error_spanned;
    pub(crate) use unwrap_or_err;
}