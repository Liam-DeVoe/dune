//! `#[derive(Canon)]` for the engine's state types.
//!
//! The canonical state dump (see `treachery-online/Oracle/Canon/StateDumper.cs`
//! and `rust/docs/trace-format.md` section 7) walks every instance field of
//! `Game` and writes it under its C# member name. The derive generates that
//! walk for a Rust struct: an object `{"$type":"<C# type>", fields...}` with
//! the fields sorted by C# name. The runtime it calls is
//! `treachery_engine::canon` (paths are `crate::canon`, so the derive only
//! works inside the engine crate).
//!
//! Field attributes:
//!
//! - `#[canon(name = "CSharpName")]`: the C# member name (after the dumper's
//!   normalization: `<X>k__BackingField` becomes `X`). Needed on every field
//!   whose C# name is not the PascalCase form of the Rust name (each `_` starts
//!   a new capitalized word: `to_id` -> `ToId`).
//! - `#[canon(skip)]`: Rust-only field with no C# counterpart.
//! - `#[canon(flatten)]`: the field is a struct holding base-class fields
//!   (for example `GameEventBase`); dump its fields inline, as C# reflection
//!   sees inherited fields as fields of the derived object.
//! - `#[canon(omit)]`: a C# field the dumper deliberately leaves out (for
//!   example `LeaderState.TimeOfDeath`). Kept so the omission is visible.
//! - `#[canon(with = "path")]`: the field is written by `path::write(&field, w)`
//!   and described by `path::shape(reg)` (used for the dumper's special cases,
//!   such as `Game.History`, which is written as its count).
//!
//! Container attributes:
//!
//! - `#[canon(manual)]`: generate nothing; the type implements the canon
//!   traits by hand (references such as `PlayerIndex` and `GameRef`).
//! - `#[canon(type_name = "X")]`: the C# type name, when it differs from the
//!   Rust struct name.
//!
//! Generic parameters must implement `crate::expression::CsType`, which gives
//! the C# type argument names (`Deck<TreacheryCard>`, `Timer<MainPhase>`).

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitStr};

#[proc_macro_derive(Canon, attributes(canon))]
pub fn derive_canon(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[derive(Default)]
struct FieldAttrs {
    name: Option<String>,
    skip: bool,
    flatten: bool,
    omit: bool,
    with: Option<syn::Path>,
}

#[derive(Default)]
struct ContainerAttrs {
    manual: bool,
    type_name: Option<String>,
}

fn container_attrs(input: &DeriveInput) -> syn::Result<ContainerAttrs> {
    let mut out = ContainerAttrs::default();
    for attr in &input.attrs {
        if !attr.path().is_ident("canon") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("manual") {
                out.manual = true;
                Ok(())
            } else if meta.path.is_ident("type_name") {
                let s: LitStr = meta.value()?.parse()?;
                out.type_name = Some(s.value());
                Ok(())
            } else {
                Err(meta.error("unknown canon container attribute (expected `manual` or `type_name`)"))
            }
        })?;
    }
    Ok(out)
}

fn field_attrs(field: &syn::Field) -> syn::Result<FieldAttrs> {
    let mut out = FieldAttrs::default();
    for attr in &field.attrs {
        if !attr.path().is_ident("canon") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let s: LitStr = meta.value()?.parse()?;
                out.name = Some(s.value());
            } else if meta.path.is_ident("skip") {
                out.skip = true;
            } else if meta.path.is_ident("flatten") {
                out.flatten = true;
            } else if meta.path.is_ident("omit") {
                out.omit = true;
            } else if meta.path.is_ident("with") {
                let s: LitStr = meta.value()?.parse()?;
                out.with = Some(s.parse()?);
            } else {
                return Err(meta.error("unknown canon field attribute (expected name, skip, flatten, omit or with)"));
            }
            Ok(())
        })?;
    }
    Ok(out)
}

/// `resources_after_bidding` -> `ResourcesAfterBidding`; `type_` -> `Type`.
fn pascal_case(rust_name: &str) -> String {
    let name = rust_name.strip_prefix("r#").unwrap_or(rust_name);
    let mut out = String::new();
    for part in name.split('_') {
        let mut chars = part.chars();
        if let Some(c) = chars.next() {
            out.extend(c.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let cattrs = container_attrs(input)?;
    if cattrs.manual {
        return Ok(TokenStream2::new());
    }

    let ident = &input.ident;
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(n) => n.named.iter().collect::<Vec<_>>(),
            Fields::Unit => Vec::new(),
            Fields::Unnamed(_) => {
                return Err(syn::Error::new_spanned(
                    ident,
                    "Canon derive needs named fields; use #[canon(manual)] and implement the traits by hand",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                ident,
                "Canon derive supports structs only; use #[canon(manual)] and implement the traits by hand",
            ))
        }
    };

    let mut push_fields = Vec::new();
    let mut field_types: Vec<syn::Type> = Vec::new();
    let mut push_shapes = Vec::new();
    for f in fields {
        let a = field_attrs(f)?;
        let fident = f.ident.as_ref().unwrap();
        let ty = &f.ty;
        if a.skip || a.omit {
            continue;
        }
        if a.flatten {
            push_fields.push(quote! {
                crate::canon::CanonObject::canon_fields(&self.#fident, out);
            });
            push_shapes.push(quote! {
                <#ty as crate::canon::CanonObject>::canon_schema_fields(reg, out);
            });
            continue;
        }
        let name = a.name.unwrap_or_else(|| pascal_case(&fident.to_string()));
        if a.with.is_none() {
            field_types.push(ty.clone());
        }
        match a.with {
            Some(path) => {
                push_fields.push(quote! {
                    out.push((#name, crate::canon::Field::Custom(::std::boxed::Box::new(move |w: &mut crate::canon::CanonWriter| #path::write(&self.#fident, w)))));
                });
                push_shapes.push(quote! {
                    { let s = #path::shape(reg); out.push((#name, s)); }
                });
            }
            None => {
                push_fields.push(quote! {
                    out.push((#name, crate::canon::Field::Value(&self.#fident)));
                });
                push_shapes.push(quote! {
                    { let s = <#ty as crate::canon::CanonShape>::canon_shape(reg); out.push((#name, s)); }
                });
            }
        }
    }

    // Generic parameters need the canon traits and a C# type name.
    let mut generics = input.generics.clone();
    {
        let where_clause = generics.make_where_clause();
        // Bound the field types rather than the parameters, so a field such as
        // `NetDictionary<T, V>` brings its own requirements on `T`.
        if input.generics.type_params().next().is_some() {
            for p in input.generics.type_params() {
                let t = &p.ident;
                where_clause.predicates.push(syn::parse_quote! { #t: crate::expression::CsType });
            }
            for ty in &field_types {
                where_clause.predicates.push(syn::parse_quote! {
                    #ty: crate::canon::Canon + crate::canon::CanonShape
                });
            }
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let base_name = cattrs.type_name.unwrap_or_else(|| ident.to_string());
    let type_params: Vec<_> = input.generics.type_params().map(|p| p.ident.clone()).collect();
    let type_name = if type_params.is_empty() {
        quote! { ::std::string::String::from(#base_name) }
    } else {
        quote! {
            {
                let args: ::std::vec::Vec<::std::string::String> = ::std::vec![
                    #(crate::canon::short_cs_type_name(&<#type_params as crate::expression::CsType>::cs_type_name())),*
                ];
                ::std::format!("{}<{}>", #base_name, args.join(","))
            }
        }
    };

    Ok(quote! {
        impl #impl_generics crate::canon::CanonObject for #ident #ty_generics #where_clause {
            fn canon_type_name() -> ::std::string::String {
                #type_name
            }

            #[allow(unused_variables)]
            fn canon_fields<'canon>(&'canon self, out: &mut ::std::vec::Vec<(&'static str, crate::canon::Field<'canon>)>) {
                #(#push_fields)*
            }

            #[allow(unused_variables)]
            fn canon_schema_fields(reg: &mut crate::canon::Registry, out: &mut ::std::vec::Vec<(&'static str, crate::canon::Shape)>) {
                #(#push_shapes)*
            }
        }

        impl #impl_generics crate::canon::Canon for #ident #ty_generics #where_clause {
            fn canon(&self, w: &mut crate::canon::CanonWriter) {
                crate::canon::write_object(self, w)
            }
        }

        impl #impl_generics crate::canon::CanonShape for #ident #ty_generics #where_clause {
            fn canon_shape(reg: &mut crate::canon::Registry) -> crate::canon::Shape {
                reg.object::<Self>()
            }
        }
    })
}
