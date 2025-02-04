use proc_macro2::TokenStream;
use quote::quote;

use crate::{
	utils::{SeedParam, SeededImplGenerics},
	SerializedFields,
};

use super::{
	attributes::{SerializeAttributes, TypeAttributes},
	Error,
};

mod named;
mod unnamed;

pub fn derive(
	ident: &syn::Ident,
	generics: &syn::Generics,
	attrs: &TypeAttributes,
	de: &SerializeAttributes,
	variant_ident: Option<&syn::Ident>,
	fields: &SerializedFields,
) -> Result<TokenStream, Error> {
	match fields {
		SerializedFields::Unit => match variant_ident {
			Some(variant_ident) => Ok(quote! {
				::serde::de::VariantAccess::unit_variant(variant)?;
				Ok(#ident :: #variant_ident)
			}),
			None => {
				let name = attrs.name(ident);
				let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
				let visitor_impl_generics =
					SeededImplGenerics::new(generics).with(SeedParam::DeLifetime);

				Ok(quote! {
					struct UnitVisitor #impl_generics (::core::marker::PhantomData<#ident #ty_generics>);

					impl #visitor_impl_generics ::serde::de::Visitor<'de> for UnitVisitor #ty_generics #where_clause {
						type Value = #ident #ty_generics;

						fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
							write!(formatter, "unit")
						}

						fn visit_unit<E>(self) -> Result<Self::Value, E> where E: ::serde::de::Error {
							Ok(#ident)
						}
					}

					deserializer.deserialize_unit_struct(#name, UnitVisitor(::core::marker::PhantomData))
				})
			}
		},
		SerializedFields::Unnamed(fields) => {
			unnamed::derive(ident, generics, attrs, de, variant_ident, fields)
		}
		SerializedFields::Named(fields) => {
			named::derive(ident, generics, attrs, de, variant_ident, fields)
		}
	}
}
