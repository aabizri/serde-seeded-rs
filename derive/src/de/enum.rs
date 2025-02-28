use proc_macro2::TokenStream;
use quote::quote;

use crate::{de::fields, SerializedFields};

use super::{
	attributes::{SerializeAttributes, TypeAttributes},
	split_visitor_generics, Error,
};

pub fn derive(
	ident: &syn::Ident,
	generics: &syn::Generics,
	attrs: &TypeAttributes,
	de: &SerializeAttributes,
	e: &syn::DataEnum,
) -> Result<TokenStream, Error> {
	let name = attrs.name(ident);
	let count = e.variants.len();

	let variants_idents = e.variants.iter().map(|v| &v.ident);
	let variants_names = e.variants.iter().map(|v| v.ident.to_string());

	let variants_cases: Vec<_> = e
		.variants
		.iter()
		.map(|v| {
			let variant_ident = &v.ident;

			let fields = SerializedFields::new(&v.fields)?;
			let de_fields =
				fields::derive(ident, generics, attrs, de, Some(variant_ident), &fields)?;

			Ok(quote! {
				Discriminant::#variant_ident => {
					#de_fields
				}
			})
		})
		.collect::<Result<_, Error>>()?;

	let cases_u64 = e.variants.iter().enumerate().map(|(i, v)| {
		let i = i as u64;
		let variant_ident = &v.ident;

		quote! {
			#i => Ok(Discriminant::#variant_ident)
		}
	});

	let cases_str = e.variants.iter().map(|v| {
		let variant_name = v.ident.to_string();
		let variant_ident = &v.ident;

		quote! {
			#variant_name => Ok(Discriminant::#variant_ident)
		}
	});

	let cases_bytes = e.variants.iter().map(|v| {
		let variant_name = v.ident.to_string();
		let variant_bytes = syn::LitByteStr::new(variant_name.as_bytes(), v.ident.span());
		let variant_ident = &v.ident;

		quote! {
			#variant_bytes => Ok(Discriminant::#variant_ident)
		}
	});

	let seed_ty = de.require_seed()?;
	let (def_generics, impl_generics, ty_generics, where_clause, value_generics) =
		split_visitor_generics(generics, de);

	Ok(quote! {
		const VARIANTS: [&str; #count] = [
			#(#variants_names),*
		];

		struct Visitor #def_generics {
			seed: &'seed #seed_ty,
			p: ::core::marker::PhantomData<#ident #value_generics>
		}

		impl #impl_generics ::serde::de::Visitor<'de> for Visitor #ty_generics #where_clause {
			type Value = #ident #value_generics;

			fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
				write!(formatter, "enum value")
			}

			fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
			where
				A: ::serde::de::EnumAccess<'de>
			{
				enum Discriminant {
					#(#variants_idents),*
				}

				struct DiscriminantVisitor;

				impl<'de> ::serde::de::Visitor<'de> for DiscriminantVisitor {
					type Value = Discriminant;

					fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
						write!(formatter, "variant identifier")
					}

					fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
					where
						E: ::serde::de::Error
					{
						match v {
							#(#cases_u64,)*
							_ => Err(::serde::de::Error::invalid_value(::serde::de::Unexpected::Unsigned(v), &"variant index"))
						}
					}

					fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
					where
						E: ::serde::de::Error
					{
						match v {
							#(#cases_str,)*
							_ => Err(::serde::de::Error::unknown_variant(v, &VARIANTS))
						}
					}

					fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
					where
						E: ::serde::de::Error
					{
						match v {
							#(#cases_bytes,)*
							// See https://github.com/serde-rs/serde/blob/e3eaa6a3dd6edd701476097182313cdbd73da78c/serde/src/de/impls.rs#L1664C33-L1667C34
							_ => match str::from_utf8(v) {
								Ok(v) => Err(::serde::de::Error::unknown_variant(v, &VARIANTS)),
								Err(_) => Err(::serde::de::Error::invalid_value(::serde::de::Unexpected::Bytes(v), &self))
							}
						}
					}
				}

				impl<'de> ::serde::de::Deserialize<'de> for Discriminant {
					fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
					where
						D: ::serde::Deserializer<'de>
					{
						deserializer.deserialize_identifier(DiscriminantVisitor)
					}
				}

				let (discriminant, variant) = data.variant::<Discriminant>()?;

				match discriminant {
					#(#variants_cases),*
				}
			}
		}

		deserializer.deserialize_enum(#name, &VARIANTS, Visitor {
			seed,
			p: ::core::marker::PhantomData::<#ident #value_generics>
		})
	})
}
