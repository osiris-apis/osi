//! # Direct Derive Implementation
//!
//! This implements the `derive` attribute. It provides parsers for the
//! arguments to the attribute, as well as parsers for the target type.
//! Furthermore, code-generators for the different derive-kinds are provided.
//!
//! See [`derive`] for the entry-point of the attribute handler.

// XXX:
// - Add support for `union` and `enum`.
// - Implement infrastructure for proper diagnostics.
// - Look into alternatives for `#[proc_macro_derive]` to replace
//   `DeriveKind::Macro` with proper proc-macros.

use ::proc_macro2;
use ::quote::{self, ToTokens};
use ::syn;

// Union helper type
enum IdentOrLitInt {
    Ident(syn::Ident),
    LitInt(syn::LitInt),
}

/// ## Kind of Trait to Derive
///
/// This is the target trait that was passed to the derive macro. This is
/// usually a path to an external macro, which derives a particular trait.
///
/// Right now, no other kinds are supported. However, this can be extended
/// with builtin kinds, or future macro versions.
pub enum DeriveKind {
    /// Derive via Macro
    ///
    /// This kind is used to forward the trait implementation to a macro at
    /// the given path. This macro then gets the expanded information necessary
    /// to implement a trait following the rules of a direct derive.
    Macro(syn::Path),
}

/// ## Single Argument to Derive
///
/// This is a single argument that was passed to derive. Right now, it simply
/// encapsulates the target trait to derive. However, this can be extended to
/// support extra annotations or other information required for the derive.
pub struct DeriveArg {
    /// Kind of trait to derive.
    pub kind: DeriveKind,
}

/// ## List of Arguments to Derive
///
/// This is the full list of arguments that was passed to derive. It is simply
/// a comma-separated list of [`DeriveArg`] objects.
pub struct DeriveList {
    /// List of arguments to derive.
    pub list: syn::punctuated::Punctuated<DeriveArg, syn::Token![,]>,
}

/// ## Item to Derive for
///
/// This is the type of the item to derive traits for. It currently aliases
/// [`::syn::ItemStruct`], but may support more in the future.
pub type DeriveItem = syn::ItemStruct;

impl quote::ToTokens for IdentOrLitInt {
    fn to_tokens(&self, code: &mut proc_macro2::TokenStream) {
        match self {
            IdentOrLitInt::Ident(v) => v.to_tokens(code),
            IdentOrLitInt::LitInt(v) => v.to_tokens(code),
        }
    }
}

impl DeriveArg {
    fn new(kind: DeriveKind) -> Self {
        Self {
            kind: kind,
        }
    }
}

impl syn::parse::Parse for DeriveArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path: syn::Path = input.parse()?;

        match path.get_ident() {
            // No builtins supported, yet.
            _ => Ok(
                DeriveArg::new(DeriveKind::Macro(path)),
            ),
        }
    }
}

impl DeriveList {
    fn new(
        list: syn::punctuated::Punctuated<DeriveArg, syn::Token![,]>,
    ) -> Self {
        Self {
            list: list,
        }
    }

    #[allow(dead_code)]
    fn from_iter<T>(iter: T) -> Self
    where T: IntoIterator<Item = DeriveArg>
    {
        DeriveList::new(<_ as FromIterator<DeriveArg>>::from_iter(iter))
    }
}

impl syn::parse::Parse for DeriveList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(DeriveList::new(
            syn::punctuated::Punctuated::parse_terminated(input)?,
        ))
    }
}

/// ## Tokenize Generics as Declaration
///
/// Tokenize a `Generics` object as a list of generic-parameter declarations,
/// stripping any information that is only useful in definitions. That is, for
/// now, this simply strips default assignments.
///
/// This produces a list of generic-parameters with type bounds surrounded in
/// angle brackets, or nothing if no generics are present.
fn generics_quote_decls(
    generics: &syn::Generics,
    code: &mut proc_macro2::TokenStream,
) {
    // A cleaner approach would be to tokenize the original manually. However,
    // creating a duplicate makes this significantly simpler, and still
    // reasonably fast.
    let mut dup = generics.clone();

    for v in dup.params.iter_mut() {
        match v {
            syn::GenericParam::Type(v) => {
                v.eq_token = None;
                v.default = None;
            },
            syn::GenericParam::Const(v) => {
                v.eq_token = None;
                v.default = None;
            },
            syn::GenericParam::Lifetime(_) => {
            },
        }
    }

    dup.to_tokens(code);
}

/// ## Tokenize Generics as Identifiers
///
/// Tokenize a `Generics` object as a list of identifiers. This simply produces
/// a list of identifiers surrounded by angle brackets, or nothing if no
/// generics are present.
fn generics_quote_idents(
    generics: &syn::Generics,
    code: &mut proc_macro2::TokenStream,
) {
    fn delim(empty: &mut bool, code: &mut proc_macro2::TokenStream) {
        if !*empty {
            <syn::Token![,]>::default().to_tokens(code);
        }
        *empty = false;
    }

    let mut empty = true;

    generics.lt_token.to_tokens(code);

    for v in generics.params.iter() {
        if let syn::GenericParam::Lifetime(v) = v {
            delim(&mut empty, code);
            v.lifetime.to_tokens(code);
        }
    }

    for v in generics.params.iter() {
        match v {
            syn::GenericParam::Type(v) => {
                delim(&mut empty, code);
                v.ident.to_tokens(code);
            }
            syn::GenericParam::Const(v) => {
                delim(&mut empty, code);
                v.ident.to_tokens(code);
            }
            syn::GenericParam::Lifetime(_) => {
                // Already tokenized.
            },
        }
    }

    generics.gt_token.to_tokens(code);
}

/// ## Extract Where Predicates
///
/// Extract all the where-predicates from the `Generics` object and return
/// them as a list to the caller. If no predicates are present, an empty list
/// will be returned.
fn generics_wheres(
    generics: &syn::Generics,
) -> Vec<&syn::WherePredicate> {
    match &generics.where_clause {
        None => Vec::new(),
        Some(v) => {
            v.predicates
                .iter()
                .collect::<Vec<&syn::WherePredicate>>()
        }
    }
}

/// ## Extract Field Identifiers
///
/// Extract the field identifiers of a struct-type and return them as a list
/// to the caller. For named fields the identifier of each field is returned.
/// For tuples, the integer index of each field is returned. For unit types,
/// an empty list is returned.
fn fields_idents(
    fields: &syn::Fields,
) -> Vec<IdentOrLitInt> {
    match fields {
        syn::Fields::Named(v) => {
            v.named
                .iter()
                .map(|v| IdentOrLitInt::Ident(v.ident.clone().unwrap()))
                .collect::<Vec<IdentOrLitInt>>()
        },
        syn::Fields::Unnamed(v) => {
            v.unnamed
                .iter()
                .enumerate()
                .map(
                    |(i, v)| IdentOrLitInt::LitInt(syn::LitInt::new(
                        &i.to_string(),
                        <syn::Type as syn::spanned::Spanned>::span(&v.ty),
                    ))
                )
                .collect::<Vec<IdentOrLitInt>>()
        },
        syn::Fields::Unit => {
            Vec::new()
        }
    }
}

/// ## Extract Field Types
///
/// Extract the types of all fields of a struct-type or tuple-type and return
/// them to the caller. Note that this is a shallow-copy which references the
/// types in the original.
fn fields_types(
    fields: &syn::Fields,
) -> Vec<&syn::Type> {
    match fields {
        syn::Fields::Named(v) => {
            v.named
                .iter()
                .map(|v| &v.ty)
                .collect::<Vec<&syn::Type>>()
        },
        syn::Fields::Unnamed(v) => {
            v.unnamed
                .iter()
                .map(|v| &v.ty)
                .collect::<Vec<&syn::Type>>()
        },
        syn::Fields::Unit => {
            Vec::new()
        }
    }
}

// Derive backend for `DeriveKind::Macro`.
fn derive_macro(
    kind_macro: &syn::Path,
    item: &DeriveItem,
    code: &mut proc_macro2::TokenStream,
) {
    let q_item_fields = fields_idents(&item.fields);
    let q_item_types = fields_types(&item.fields);
    let q_item_ident = &item.ident;
    let q_generics_where = generics_wheres(&item.generics);

    let mut q_generics_decl = proc_macro2::TokenStream::new();
    generics_quote_decls(&item.generics, &mut q_generics_decl);

    let mut q_generics_ident = proc_macro2::TokenStream::new();
    generics_quote_idents(&item.generics, &mut q_generics_ident);

    let s_derive = match &item.fields {
        syn::Fields::Named(_) => "derive_struct",
        syn::Fields::Unnamed(_) => "derive_tuple",
        syn::Fields::Unit => "derive_struct",
    };
    let q_derive = syn::Ident::new(s_derive, proc_macro2::Span::call_site());

    // Rather than generating code to implement the requested trait, we instead
    // evaluate to a macro invocation and pass all information required to
    // suitably implement any trait for the target type.
    //
    // See the documentation of [`crate::derive`] for details on the
    // macro arguments.
    *code = quote::quote! {
        #code
        #kind_macro !(
            #q_derive,
            #q_item_ident,
            #q_item_ident #q_generics_ident,
            (#q_generics_decl),
            (
                #(
                    (#q_generics_where)
                ),*
            ),
            (
                #(
                    #q_item_fields
                ),*
            ),
            (
                #(
                    #q_item_types
                ),*
            )
        );
    };
}

/// ## Direct Derive
///
/// This is the implementation of the `derive` attribute. See
/// [`crate::derive`] for documentation.
///
/// This function is untangled from `proc_macro` types and solely uses the
/// types from `proc_macro2`, and can thus be used in unit tests and other
/// non-proc code.
pub fn derive(
    list: DeriveList,
    item: DeriveItem,
) -> proc_macro2::TokenStream {
    let mut code = proc_macro2::TokenStream::new();

    // Generate code for every derive-argument.
    for arg in &list.list {
        match arg.kind {
            // Only external macros are currently supported as target.
            DeriveKind::Macro(ref v) => derive_macro(v, &item, &mut code),
        }
    }

    quote::quote! {
        #item
        #code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify basic macro expansion
    //
    // Use the `DeriveKind::Macro` expansion and verify the generated code with
    // the most basic target type.
    #[test]
    fn kind_macro_basic() {
        let arg = syn::parse_quote! { ::foo::bar };
        let item: syn::ItemStruct = syn::parse_quote! { struct Foobar; };

        let list = DeriveList::from_iter(
            [DeriveArg::new(DeriveKind::Macro(arg))],
        );

        assert_eq!(
            derive(list, item).to_string(),
            quote::quote! {
                struct Foobar;
                ::foo::bar!(
                    derive_struct,
                    Foobar,
                    Foobar,
                    (),
                    (),
                    (),
                    ()
                );
            }.to_string(),
        );
    }

    // Verify macro expansion with generics
    //
    // Use the `DeriveKind::Macro` expansion and verify the generated code with
    // generic parameters and where clauses.
    #[test]
    fn kind_macro_generics() {
        let arg = syn::parse_quote! { ::foo::bar };
        let item: syn::ItemStruct = syn::parse_quote! {
            struct Foobar<'a, 'b, A: Copy = u8, B: Clone = u8>
            where
                A: Clone + Debug,
                B: Copy + Debug,
            {
                a: &'a A,
                b: &'b B,
            }
        };

        let list = DeriveList::from_iter(
            [DeriveArg::new(DeriveKind::Macro(arg))],
        );

        assert_eq!(
            derive(list, item).to_string(),
            quote::quote! {
                struct Foobar<'a, 'b, A: Copy = u8, B: Clone = u8>
                where
                    A: Clone + Debug,
                    B: Copy + Debug,
                {
                    a: &'a A,
                    b: &'b B,
                }
                ::foo::bar!(
                    derive_struct,
                    Foobar,
                    Foobar<'a, 'b, A, B>,
                    (<'a, 'b, A: Copy, B: Clone>),
                    ((A: Clone + Debug), (B: Copy + Debug)),
                    (a, b),
                    (&'a A, &'b B)
                );
            }.to_string(),
        );
    }
}
