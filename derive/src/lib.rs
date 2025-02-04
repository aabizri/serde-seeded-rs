//! This is a companion crate for [`serde-seeded`] defining the
//! `SerializeSeeded` and `DeserializeSeeded` derive macros. It is not
//! recommended to use this crate directly. Use the `serde-seeded` crate
//! directly instead with the `derive` feature enabled.
//!
//! [`serde-seeded`]: <https://crates.io/crates/serde-seeded>
use attributes::FieldAttributes;
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{abort, proc_macro_error};
use quote::format_ident;
use syn::{parse_macro_input, spanned::Spanned};

pub(crate) mod attributes;
mod de;
mod ser;
pub(crate) mod utils;

/// Derive macro implementing the `SerializeSeeded` trait automatically.
///
/// This macro is available through the `derive` feature.
#[proc_macro_derive(SerializeSeeded, attributes(seeded))]
#[proc_macro_error]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as syn::DeriveInput);
	match ser::derive(input) {
		Ok(tokens) => tokens.into(),
		Err(e) => {
			abort!(e.span(), e.to_string())
		}
	}
}

/// Derive macro implementing the `DeserializeSeeded` trait automatically.
///
/// This macro is available through the `derive` feature.
#[proc_macro_derive(DeserializeSeeded, attributes(seeded))]
#[proc_macro_error]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as syn::DeriveInput);
	match de::derive(input) {
		Ok(tokens) => tokens.into(),
		Err(e) => {
			abort!(e.span(), e.to_string())
		}
	}
}

enum SerializedFields {
	Unit,
	Unnamed(Vec<SerializedUnnamedField>),
	Named(Vec<SerializedNamedField>),
}

impl SerializedFields {
	fn new(fields: &syn::Fields) -> Result<Self, attributes::Error> {
		match fields {
			syn::Fields::Unit => Ok(Self::Unit),
			syn::Fields::Unnamed(fields) => fields
				.unnamed
				.iter()
				.enumerate()
				.map(|(i, f)| {
					Ok(SerializedUnnamedField {
						attrs: FieldAttributes::parse_attributes(&f.attrs)?,
						index: i.into(),
						id: format_ident!("arg_{i}"),
						ty: f.ty.clone(),
					})
				})
				.collect::<Result<_, _>>()
				.map(Self::Unnamed),
			syn::Fields::Named(fields) => fields
				.named
				.iter()
				.map(|f| {
					Ok(SerializedNamedField {
						attrs: FieldAttributes::parse_attributes(&f.attrs)?,
						id: f.ident.clone().unwrap(),
						ty: f.ty.clone(),
						span: f.span(),
					})
				})
				.collect::<Result<_, _>>()
				.map(Self::Named),
		}
	}
}

struct SerializedUnnamedField {
	attrs: FieldAttributes,
	index: syn::Index,
	id: syn::Ident,
	ty: syn::Type,
}

struct SerializedNamedField {
	attrs: FieldAttributes,
	id: syn::Ident,
	ty: syn::Type,
	span: Span,
}

impl SerializedNamedField {
	pub fn name(&self) -> String {
		self.attrs.name(&self.id)
	}
}
