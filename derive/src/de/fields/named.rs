use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
	attributes::{SerializeAttributes, TypeAttributes},
	de::{deserialize_seed, split_visitor_generics, Error},
	utils::TryFilterMapExt,
	SerializedNamedField,
};

pub fn derive(
	ident: &syn::Ident,
	generics: &syn::Generics,
	attrs: &TypeAttributes,
	de: &SerializeAttributes,
	variant_ident: Option<&syn::Ident>,
	fields: &[SerializedNamedField],
) -> Result<TokenStream, Error> {
	let name = attrs.name(ident);
	let variant_ext = variant_ident.map(|i| {
		quote! {
			:: #i
		}
	});

	let count = fields.iter().filter(|f| !f.attrs.skip).count();

	let fields_names = fields
		.iter()
		.filter_map(|f| if f.attrs.skip { None } else { Some(f.name()) });

	let fields_variants = fields.iter().enumerate().filter_map(|(i, f)| {
		if f.attrs.skip {
			None
		} else {
			Some(format_ident!("Field{i}"))
		}
	});

	let define_fields = fields.iter().filter_map(|f| {
		if f.attrs.skip {
			None
		} else {
			let field_id = &f.id;
			Some(quote! {
				let mut #field_id = None;
			})
		}
	});

	let cases_u64 = fields
		.iter()
		.enumerate()
		.try_filter_map(|(i, f)| {
			if f.attrs.skip {
				Ok(None)
			} else {
				let i = i as u64;
				let id = format_ident!("Field{i}");
				Ok(Some(quote! {
					#i => Ok(Field__::#id)
				}))
			}
		})
		.collect::<Result<Vec<_>, Error>>()?;

	let cases_str = fields
		.iter()
		.enumerate()
		.try_filter_map(|(i, f)| {
			if f.attrs.skip {
				Ok(None)
			} else {
				let name = f.name();
				let id = format_ident!("Field{i}");
				Ok(Some(quote! {
					#name => Ok(Field__::#id)
				}))
			}
		})
		.collect::<Result<Vec<_>, Error>>()?;

	let cases_bytes = fields
		.iter()
		.enumerate()
		.try_filter_map(|(i, f)| {
			if f.attrs.skip {
				Ok(None)
			} else {
				let name = f.name();
				let bytes = syn::LitByteStr::new(name.as_bytes(), f.span);
				let id = format_ident!("Field{i}");
				Ok(Some(quote! {
					#bytes => Ok(Field__::#id)
				}))
			}
		})
		.collect::<Result<Vec<_>, Error>>()?;

	let cases = fields
		.iter()
		.enumerate()
		.try_filter_map(|(i, f)| {
			if f.attrs.skip {
				Ok(None)
			} else {
				let field_id = &f.id;
				let id = format_ident!("Field{i}");
				let seed = deserialize_seed(ident, generics, de, &f.attrs, &f.ty)?;
				Ok(Some(quote! {
					Field__::#id => {
						#field_id = Some(map__.next_value_seed(#seed)?)
					}
				}))
			}
		})
		.collect::<Result<Vec<_>, Error>>()?;

	let unwrap_fields = fields.iter().map(|f| {
		let field_id = &f.id;
		let field_name = f.name();

		if f.attrs.skip {
			quote! {
				#field_id: ::core::default::Default::default()
			}
		} else if f.attrs.default {
			quote! {
				#field_id: #field_id.unwrap_or_default()
			}
		} else {
			quote! {
				#field_id: #field_id.ok_or_else(|| serde::de::Error::missing_field(#field_name))?
			}
		}
	});

	let visit = if variant_ident.is_some() {
		quote! {
			const FIELDS: [&str; #count] = [
				#(#fields_names),*
			];

			::serde::de::VariantAccess::struct_variant(variant, &FIELDS, StructVisitor {
				seed: self.seed,
				t: std::marker::PhantomData
			})
		}
	} else {
		quote! {
			const FIELDS: [&str; #count] = [
				#(#fields_names),*
			];

			deserializer.deserialize_struct(#name, &FIELDS, StructVisitor {
				seed,
				t: std::marker::PhantomData
			})
		}
	};

	let seed_ty = de.require_seed()?;
	let (def_generics, impl_generics, ty_generics, where_clause, value_generics) =
		split_visitor_generics(generics, de);

	Ok(quote! {
		struct StructVisitor #def_generics {
			seed: &'seed #seed_ty,
			t: std::marker::PhantomData<#ident #value_generics>
		}

		impl #impl_generics ::serde::de::Visitor<'de> for StructVisitor #ty_generics #where_clause {
			type Value = #ident #value_generics;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "a struct")
			}

			fn visit_map<A>(self, mut map__: A) -> Result<Self::Value, A::Error>
			where
				A: ::serde::de::MapAccess<'de>
			{
				enum Field__ {
					#(#fields_variants),*
				}

				impl<'de> ::serde::Deserialize<'de> for Field__ {
					fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
					where
						D: ::serde::de::Deserializer<'de>
					{
						struct Visitor;

						impl<'de> ::serde::de::Visitor<'de> for Visitor {
							type Value = Field__;

							fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
								write!(formatter, "field identifier")
							}

							fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
							where
								E: ::serde::de::Error
							{
								match v {
									#(#cases_u64,)*
									_ => Err(::serde::de::Error::invalid_value(::serde::de::Unexpected::Unsigned(v), &"field index"))
								}
							}

							fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
							where
								E: ::serde::de::Error
							{
								match v {
									#(#cases_str,)*
									_ => Err(::serde::de::Error::unknown_variant(v, &FIELDS))
								}
							}

							fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
							where
								E: ::serde::de::Error
							{
								match v {
									#(#cases_bytes,)*
									_ => {
										let v = String::from_utf8_lossy(v);
										Err(::serde::de::Error::unknown_variant(&v, &FIELDS))
									}
								}
							}
						}

						deserializer.deserialize_identifier(Visitor)
					}
				}

				#(#define_fields)*

				while let Some(field) = map__.next_key()? {
					match field {
						#(#cases),*
					}
				}

				Ok(#ident #variant_ext {
					#(#unwrap_fields),*
				})
			}
		}

		#visit
	})
}
