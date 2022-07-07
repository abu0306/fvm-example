use crate::ir::selector::Selector;
use crate::format_err;
use proc_macro2::Span;
use syn::spanned::Spanned as _;

pub fn ensure_pub_visibility(
    name: &str,
    parent_span: Span,
    vis: &syn::Visibility,
) -> Result<(), syn::Error> {
    let bad_visibility = match vis {
        syn::Visibility::Inherited => Some(parent_span),
        syn::Visibility::Restricted(vis_restricted) => Some(vis_restricted.span()),
        syn::Visibility::Crate(vis_crate) => Some(vis_crate.span()),
        syn::Visibility::Public(_) => None,
    };
    if let Some(bad_visibility) = bad_visibility {
        return Err(format_err!(
            bad_visibility,
            "non `pub` fvm! {} are not supported",
            name
        ));
    }
    Ok(())
}

pub fn local_message_id(ident: &syn::Ident) -> u32 {
    let input = ident.to_string().into_bytes();
    let selector = Selector::compute(&input);
    selector.into_be_u32()
}
