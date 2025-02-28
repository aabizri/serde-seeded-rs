use proc_macro2::TokenStream;
use quote::quote;

use crate::{
	attributes::{SerializeAttributes, TypeAttributes},
	de::{deserialize_seed, split_visitor_generics, Error},
	SerializedUnnamedField,
};

pub fn derive(
	ident: &syn::Ident,
	generics: &syn::Generics,
	attrs: &TypeAttributes,
	de: &SerializeAttributes,
	variant_ident: Option<&syn::Ident>,
	fields: &[SerializedUnnamedField],
) -> Result<TokenStream, Error> {
	let name = attrs.name(ident);
	let seed_ty = de.require_seed()?;
	let variant_ext = variant_ident.map(|i| {
		quote! {
			:: #i
		}
	});

	if fields.len() == 1 {
		let v = fields.first().unwrap();
		let seed = deserialize_seed(ident, generics, de, &v.attrs, &v.ty)?;

		if variant_ident.is_some() {
			Ok(quote! {
				::serde::de::VariantAccess::newtype_variant_seed(variant, #seed)
					.map(#ident #variant_ext)
			})
		} else {
			let (def_generics, impl_generics, ty_generics, where_clause, value_generics) =
				split_visitor_generics(generics, de);

			Ok(quote! {
				struct NewtypeVisitor #def_generics {
					seed: &'seed #seed_ty,
					p: ::core::marker::PhantomData<#ident #value_generics>
				}

				impl #impl_generics ::serde::de::Visitor<'de> for NewtypeVisitor #ty_generics #where_clause {
					type Value = #ident #value_generics;

					fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
						write!(formatter, "a newtype struct")
					}

					fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
					where
						D: ::serde::Deserializer<'de>
					{
						Ok(#ident #variant_ext (
							::serde::de::DeserializeSeed::deserialize(#seed, deserializer)?
						))
					}
				}

				deserializer.deserialize_newtype_struct(#name, NewtypeVisitor {
					seed,
					p: ::core::marker::PhantomData::<#ident #value_generics>
				})
			})
		}
	} else {
		let count = fields.len();
		let error_message = format!("{count} arguments");
		let args: Vec<_> = fields
			.iter()
			.enumerate()
			.map(|(i, v)| {
				let seed = deserialize_seed(ident, generics, de, &v.attrs, &v.ty)?;
				Ok(quote! {
					seq.next_element_seed(#seed)?.ok_or_else(|| {
						::serde::de::Error::invalid_length(
							#i,
							&#error_message
						)
					})?
				})
			})
			.collect::<Result<_, Error>>()?;

		let (def_generics, impl_generics, ty_generics, where_clause, value_generics) =
			split_visitor_generics(generics, de);

		let visit = if variant_ident.is_some() {
			quote! {
				::serde::de::VariantAccess::tuple_variant(variant, #count, TupleVisitor {
					seed: self.seed,
					p: ::core::marker::PhantomData::<#ident #value_generics>
				})
			}
		} else {
			quote! {
				deserializer.deserialize_tuple_struct(#name, #count, TupleVisitor {
					seed,
					p: ::core::marker::PhantomData::<#ident #value_generics>
				})
			}
		};

		Ok(quote! {
			struct TupleVisitor #def_generics {
				seed: &'seed #seed_ty,
				p: ::core::marker::PhantomData<#ident #value_generics>
			}

			impl #impl_generics ::serde::de::Visitor<'de> for TupleVisitor #ty_generics #where_clause {
				type Value = #ident #value_generics;

				fn expecting(&self, formatter: &mut core::fmt::Formatter) -> ::core::fmt::Result {
					write!(formatter, "a tuple")
				}

				fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
				where
					A: ::serde::de::SeqAccess<'de>
				{
					Ok(#ident #variant_ext (#(#args),*))
				}
			}

			#visit
		})
	}
}
