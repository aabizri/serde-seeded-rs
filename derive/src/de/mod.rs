use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;

use crate::{
	attributes,
	utils::{SeedParam, SeededImplGenerics, SeededTypeGenerics},
	SerializedFields,
};

use self::attributes::{FieldAttributes, SerializeAttributes, TypeAttributes};

mod r#enum;
mod fields;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("unsupported `union` type")]
	Union(Span),

	#[error("`enum` cannot be transparent")]
	TransparentEnum(Span),

	#[error("cannot deserialize unit struct transparently")]
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

	for de in &attrs.de {
		let seed_ty = de.require_seed()?;
		let additional_predicates =
			additional_predicates(&input.generics, seed_ty, &de.override_bounds);
		let mut generics = input.generics.clone();
		generics
			.make_where_clause()
			.predicates
			.extend(additional_predicates);
		generics
			.make_where_clause()
			.predicates
			.extend(de.bounds.iter().cloned());
		generics
			.make_where_clause()
			.predicates
			.extend(de.override_bounds.iter().cloned());

		let body = if attrs.transparent {
			match &input.data {
				syn::Data::Struct(s) => {
					let fields = SerializedFields::new(&s.fields)?;

					match fields {
						SerializedFields::Unit => return Err(Error::TransparentUnit(input.span())),
						SerializedFields::Named(fields) => {
							let init_fields = fields.iter().map(|f| {
								let f_ident = &f.id;
								if f.attrs.skip {
									quote! {
										#f_ident: Default::default()
									}
								} else {
									quote! {
										#f_ident: ::serde_seeded::DeserializeSeeded::deserialize_seeded(
											seed,
											deserializer
										)?
									}
								}
							});

							quote! {
								Ok(Self {
									#(#init_fields),*
								})
							}
						}
						SerializedFields::Unnamed(fields) => {
							let init_fields = fields.iter().map(|f| {
								if f.attrs.skip {
									quote! {
										Default::default()
									}
								} else {
									quote! {
										::serde_seeded::DeserializeSeeded::deserialize_seeded(
											seed,
											deserializer
										)?
									}
								}
							});

							quote! {
								Ok(Self(#(#init_fields),*))
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
					fields::derive(ident, &generics, &attrs, de, None, &fields)?
				}
				syn::Data::Enum(e) => r#enum::derive(ident, &generics, &attrs, de, e)?,
				syn::Data::Union(u) => return Err(Error::Union(u.union_token.span)),
			}
		};

		let impl_generics = SeededImplGenerics::new(&generics)
			.with(SeedParam::DeLifetime)
			.with_extra_params(&de.params);
		let (_, ty_generics, where_clause) = generics.split_for_impl();

		tokens.extend(quote! {
			impl #impl_generics ::serde_seeded::DeserializeSeeded<'de, #seed_ty> for #ident #ty_generics #where_clause {
				fn deserialize_seeded<D>(
					seed: &#seed_ty,
					deserializer: D
				) -> Result<Self, D::Error> where D: ::serde::Deserializer<'de> {
					#body
				}
			}
		});
	}

	Ok(tokens)
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
					#ident: ::serde_seeded::DeserializeSeeded<'de, #seed_ty>
				})
				.unwrap(),
			)
		}
	}

	result
}

fn deserialize_seed(
	ident: &syn::Ident,
	generics: &syn::Generics,
	de: &SerializeAttributes,
	attrs: &FieldAttributes,
	ty: &syn::Type,
) -> Result<TokenStream, Error> {
	match &attrs.with {
		Some(id) => {
			let seed_ty = de.require_seed()?;

			let def_generics = SeededImplGenerics::new(generics)
				.with(SeedParam::SeedLifetime | SeedParam::DeLifetime)
				.with_extra_params(&de.params);
			let where_clause = generics.where_clause.as_ref();
			let impl_generics = SeededImplGenerics::new(generics)
				.with(SeedParam::SeedLifetime | SeedParam::DeLifetime)
				.with_extra_params(&de.params);
			let ty_generics = SeededTypeGenerics::new(generics)
				.with(SeedParam::SeedLifetime | SeedParam::DeLifetime)
				.with_extra_params(&de.params);
			let target_generics = SeededTypeGenerics::new(generics);

			Ok(quote! {
				{
					struct DeserializeWith #def_generics #where_clause {
						seed: &'seed #seed_ty,
						p: std::marker::PhantomData<(&'de (), #ident #target_generics)>
					}

					impl #impl_generics ::serde::de::DeserializeSeed<'de> for DeserializeWith #ty_generics #where_clause {
						type Value = #ty;

						fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
						where
							D: ::serde::Deserializer<'de>
						{
							#id :: deserialize_seeded(
								self.seed,
								deserializer
							)
						}
					}

					DeserializeWith {
						seed: self.seed,
						p: std::marker::PhantomData::<(&'de (), #ident #target_generics)>
					}
				}
			})
		}
		None => Ok(quote! {
			::serde_seeded::de::Seed::<_, #ty>::new(self.seed)
		}),
	}
}

/// Split the type generic parameters to create a visitor type.
///
/// ```ignore
/// let (def_generics, impl_generics, ty_generics, where_clause, value_generics) = split_visitor_generics(generics);
///
/// quote! {
///   struct Visitor #def_generics {}
///
///   impl #impl_generics serde::de::Visitor<'de> for Visitor #ty_generics #where_clause {
///     type Value = Value #value_generics;
///   }
/// }
/// ```
fn split_visitor_generics<'a>(
	generics: &'a syn::Generics,
	de: &'a SerializeAttributes,
) -> (
	SeededImplGenerics<'a>,
	SeededImplGenerics<'a>,
	SeededTypeGenerics<'a>,
	Option<&'a syn::WhereClause>,
	SeededTypeGenerics<'a>,
) {
	(
		SeededImplGenerics::new(generics)
			.with(SeedParam::SeedLifetime)
			.with_extra_params(&de.params),
		SeededImplGenerics::new(generics)
			.with(SeedParam::SeedLifetime | SeedParam::DeLifetime)
			.with_extra_params(&de.params),
		SeededTypeGenerics::new(generics)
			.with(SeedParam::SeedLifetime)
			.with_extra_params(&de.params),
		generics.where_clause.as_ref(),
		SeededTypeGenerics::new(generics),
	)
}
