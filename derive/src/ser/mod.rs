use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::spanned::Spanned;

use crate::{
	attributes::{self, TypeAttributes},
	utils::{SeedParam, SeededImplGenerics, SeededTypeGenerics},
	SerializedFields,
};

use self::attributes::{FieldAttributes, SerializeAttributes};

mod fields;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("unsupported `union` type")]
	Union(Span),

	#[error("`enum` cannot be transparent")]
	TransparentEnum(Span),

	#[error("cannot serialize unit struct transparently")]
	TransparentUnit(Span),

	#[error(transparent)]
	Attribute(#[from] attributes::Error),
}

impl Error {
	pub fn span(&self) -> Span {
		match self {
			Self::Union(s) => *s,
			Self::TransparentEnum(s) => *s,
			Self::TransparentUnit(s) => *s,
			Self::Attribute(e) => e.span(),
		}
	}
}

pub fn derive(input: syn::DeriveInput) -> Result<TokenStream, Error> {
	let ident = &input.ident;

	let attrs = TypeAttributes::parse_attributes(&input.attrs)?;

	let mut tokens = TokenStream::new();

	for ser in &attrs.ser {
		let seed_ty = ser.require_seed()?;
		let additional_predicates =
			additional_predicates(&input.generics, seed_ty, &ser.override_bounds);
		let mut generics = input.generics.clone();
		generics
			.make_where_clause()
			.predicates
			.extend(additional_predicates);
		generics
			.make_where_clause()
			.predicates
			.extend(ser.bounds.iter().cloned());
		generics
			.make_where_clause()
			.predicates
			.extend(ser.override_bounds.iter().cloned());

		let body = if attrs.transparent {
			match &input.data {
				syn::Data::Struct(s) => {
					let fields = SerializedFields::new(&s.fields)?;

					match fields {
						SerializedFields::Unit => return Err(Error::TransparentUnit(input.span())),
						SerializedFields::Named(fields) => {
							let ser_field = fields.iter().find_map(|f| {
								let f_ident = &f.id;
								if f.attrs.skip {
									None
								} else {
									Some(quote! {
										::serde_seeded::SerializeSeeded::serialize_seeded(
											&self.#f_ident,
											seed,
											serializer
										)
									})
								}
							});

							quote! {
								#ser_field
							}
						}
						SerializedFields::Unnamed(fields) => {
							let ser_field = fields.iter().find_map(|f| {
								let f_index = &f.index;
								if f.attrs.skip {
									None
								} else {
									Some(quote! {
										::serde_seeded::SerializeSeeded::serialize_seeded(
											&self.#f_index,
											seed,
											serializer
										)
									})
								}
							});

							quote! {
								#ser_field
							}
						}
					}
				}
				syn::Data::Enum(e) => return Err(Error::TransparentEnum(e.enum_token.span)),
				syn::Data::Union(u) => return Err(Error::Union(u.union_token.span)),
			}
		} else {
			match &input.data {
				syn::Data::Struct(s) => {
					let fields = SerializedFields::new(&s.fields)?;
					fields::derive(ident, &generics, &attrs, ser, None, &fields)?
				}
				syn::Data::Enum(e) => {
					let cases = e
						.variants
						.iter()
						.enumerate()
						.map(|(i, v)| {
							let variant_ident = &v.ident;
							let variant = SerializedVariant {
								index: i as u32,
								ident: variant_ident,
							};
							let fields = SerializedFields::new(&v.fields)?;
							let ser_variant = fields::derive(
								ident,
								&generics,
								&attrs,
								ser,
								Some(variant),
								&fields,
							)?;

							let args = match &v.fields {
								syn::Fields::Unit => {
									quote! {}
								}
								syn::Fields::Unnamed(fields) => {
									let args =
										(0..fields.unnamed.len()).map(|i| format_ident!("arg_{i}"));

									quote! {
										( #(#args),* )
									}
								}
								syn::Fields::Named(fields) => {
									let args = fields.named.iter().map(|f| &f.ident);

									quote! {
										{ #(#args),* }
									}
								}
							};

							Ok(quote! {
								Self::#variant_ident #args => {
									#ser_variant
								}
							})
						})
						.collect::<Result<Vec<_>, Error>>()?;

					quote! {
						match self {
							#(#cases),*
						}
					}
				}
				syn::Data::Union(u) => return Err(Error::Union(u.union_token.span)),
			}
		};

		let (impl_generics, ty_generics, where_clause) = split_ser_generics(&generics, &ser.params);

		tokens.extend(quote! {
			impl #impl_generics ::serde_seeded::SerializeSeeded<#seed_ty> for #ident #ty_generics #where_clause {
				fn serialize_seeded<S>(
					&self,
					seed: &#seed_ty,
					serializer: S
				) -> Result<S::Ok, S::Error> where S: ::serde::Serializer {
					#body
				}
			}
		});
	}

	Ok(tokens)
}

fn split_ser_generics<'a>(
	generics: &'a syn::Generics,
	extra_params: &'a [syn::GenericParam],
) -> (
	SeededImplGenerics<'a>,
	syn::TypeGenerics<'a>,
	Option<&'a syn::WhereClause>,
) {
	let (_, ty_generics, where_clause) = generics.split_for_impl();
	let impl_generics = SeededImplGenerics::new(generics).with_extra_params(extra_params);
	(impl_generics, ty_generics, where_clause)
}

fn additional_predicates(
	generics: &syn::Generics,
	seed_ty: &syn::Type,
	override_bounds: &[syn::WherePredicate],
) -> Vec<syn::WherePredicate> {
	let mut result = Vec::new();

	'generic_params: for p in &generics.params {
		if let syn::GenericParam::Type(t) = p {
			let ident = &t.ident;

			for bound in override_bounds {
				if let syn::WherePredicate::Type(bound) = bound {
					if let syn::Type::Path(path) = &bound.bounded_ty {
						if path.qself.is_none() && path.path.is_ident(ident) {
							continue 'generic_params;
						}
					}
				}
			}

			result.push(
				syn::parse2(quote! {
					#ident: ::serde_seeded::SerializeSeeded<#seed_ty>
				})
				.unwrap(),
			)
		}
	}

	result
}

#[derive(Clone, Copy)]
pub struct SerializedVariant<'a> {
	index: u32,
	ident: &'a syn::Ident,
}

fn value_serializer(
	ident: &syn::Ident,
	generics: &syn::Generics,
	ser: &SerializeAttributes,
	attrs: &FieldAttributes,
	ty: &syn::Type,
	value: TokenStream,
) -> Result<TokenStream, Error> {
	match &attrs.with {
		Some(id) => {
			let seed_ty = ser.require_seed()?;
			let def_generics = SeededImplGenerics::new(generics).with(SeedParam::ValueLifetime);
			let impl_generics = SeededImplGenerics::new(generics)
				.with(SeedParam::ValueLifetime)
				.with_extra_params(&ser.params);
			let ty_generics = SeededTypeGenerics::new(generics).with(SeedParam::ValueLifetime);
			let where_clause = generics.where_clause.as_ref();
			let target_generics = SeededTypeGenerics::new(generics);

			Ok(quote! {
				{
					struct SerializeWith #def_generics #where_clause {
						value: &'value #ty,
						p: ::core::marker::PhantomData<#ident #target_generics>
					}

					impl #impl_generics ::serde_seeded::SerializeSeeded<#seed_ty> for SerializeWith #ty_generics #where_clause {
						fn serialize_seeded<S>(&self, seed: &#seed_ty, serializer: S) -> Result<S::Ok, S::Error>
						where
							S: ::serde::Serializer
						{
							#id :: serialize_seeded (
								self.value,
								seed,
								serializer
							)
						}
					}

					::serde_seeded::ser::Seeded::new(seed, SerializeWith { value: #value, p: ::core::marker::PhantomData::<#ident #target_generics> })
				}
			})
		}
		None => Ok(quote! {
			::serde_seeded::ser::Seeded::new(seed, #value)
		}),
	}
}
