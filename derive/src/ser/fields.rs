use proc_macro2::TokenStream;
use quote::quote;

use crate::SerializedFields;

use super::{
	attributes::{SerializeAttributes, TypeAttributes},
	value_serializer, Error, SerializedVariant,
};

pub fn derive(
	ident: &syn::Ident,
	generics: &syn::Generics,
	attrs: &TypeAttributes,
	ser: &SerializeAttributes,
	variant: Option<SerializedVariant>,
	fields: &SerializedFields,
) -> Result<TokenStream, Error> {
	let name = attrs.name(ident);

	match fields {
		SerializedFields::Unit => match variant {
			Some(variant) => {
				let variant_index = variant.index;
				let variant_name = variant.ident.to_string();

				Ok(quote! {
					serializer.serialize_unit_variant(#name, #variant_index, #variant_name)
				})
			}
			None => Ok(quote! {
				serializer.serialize_unit_struct(#name)
			}),
		},
		SerializedFields::Unnamed(fields) => {
			let unskipped_fields_count = fields.iter().filter(|f| !f.attrs.skip).count();

			match unskipped_fields_count {
				0 => match variant {
					Some(variant) => {
						let variant_index = variant.index;
						let variant_name = variant.ident.to_string();

						Ok(quote! {
							serializer.serialize_unit_variant(#name, #variant_index, #variant_name)
						})
					}
					None => Ok(quote! {
						serializer.serialize_unit_struct(#name)
					}),
				},
				1 => {
					let field = fields.iter().find(|f| !f.attrs.skip).unwrap();

					match variant {
						Some(variant) => {
							let variant_index = variant.index;
							let variant_name = variant.ident.to_string();

							let field_id = &field.id;
							let value_serializer = value_serializer(
								ident,
								generics,
								ser,
								&field.attrs,
								&field.ty,
								quote! { #field_id },
							)?;

							Ok(quote! {
								serializer.serialize_newtype_variant(
									#name,
									#variant_index,
									#variant_name,
									&#value_serializer
								)
							})
						}
						None => {
							let field_index = &field.index;
							let value_serializer = value_serializer(
								ident,
								generics,
								ser,
								&field.attrs,
								&field.ty,
								quote! { &self.#field_index },
							)?;

							Ok(quote! {
								serializer.serialize_newtype_struct(
									#name,
									&#value_serializer
								)
							})
						}
					}
				}
				_ => {
					let count = fields.len();

					let ser_fields = fields
						.iter()
						.map(|f| {
							if variant.is_some() {
								let arg = &f.id;
								let value_serializer = value_serializer(
									ident,
									generics,
									ser,
									&f.attrs,
									&f.ty,
									quote! { #arg },
								)?;

								Ok(quote! {
									::serde::ser::SerializeTupleVariant::serialize_field(
										&mut s,
										&#value_serializer
									)?;
								})
							} else {
								let index = &f.index;
								let value_serializer = value_serializer(
									ident,
									generics,
									ser,
									&f.attrs,
									&f.ty,
									quote! { &self.#index },
								)?;

								Ok(quote! {
									::serde::ser::SerializeTupleStruct::serialize_field(
										&mut s,
										&#value_serializer
									)?;
								})
							}
						})
						.collect::<Result<Vec<_>, Error>>()?;

					match variant {
						Some(variant) => {
							let variant_index = variant.index;
							let variant_name = variant.ident.to_string();

							Ok(quote! {
								let mut s = serializer.serialize_tuple_variant(#name, #variant_index, #variant_name, #count)?;

								#(#ser_fields)*

								::serde::ser::SerializeTupleVariant::end(s)
							})
						}
						None => Ok(quote! {
							let mut s = serializer.serialize_tuple_struct(#name, #count)?;

							#(#ser_fields)*

							::serde::ser::SerializeTupleStruct::end(s)
						}),
					}
				}
			}
		}
		SerializedFields::Named(fields) => {
			// let unskipped_fields_count = fields.iter().filter(|f| !f.attrs.skip).count();

			let count_expr_terms = fields.iter().filter_map(|f| {
				if f.attrs.skip {
					return None;
				}

				let field_ident = &f.id;
				let field_accessor = if variant.is_some() {
					quote! { #field_ident }
				} else {
					quote! { &self.#field_ident }
				};

				match &f.attrs.skip_serializing_if {
					Some(path) => Some(quote! { if #path (#field_accessor) { 1 } else { 0 } }),
					None => Some(quote! { 1 }),
				}
			});

			let count_expr = quote! { 0 #( + #count_expr_terms )* };

			let mut ser_fields = Vec::new();
			for f in fields {
				if f.attrs.skip {
					continue;
				}

				let field_ident = &f.id;
				let field_name = f.name();
				let field_accessor = if variant.is_some() {
					quote! { #field_ident }
				} else {
					quote! { &self.#field_ident }
				};

				let serialize_field = if variant.is_some() {
					let value_serializer = value_serializer(
						ident,
						generics,
						ser,
						&f.attrs,
						&f.ty,
						field_accessor.clone(),
					)?;

					quote! {
						::serde::ser::SerializeStructVariant::serialize_field(
							&mut s,
							#field_name,
							&#value_serializer
						)?;
					}
				} else {
					let value_serializer = value_serializer(
						ident,
						generics,
						ser,
						&f.attrs,
						&f.ty,
						field_accessor.clone(),
					)?;

					quote! {
						::serde::ser::SerializeStruct::serialize_field(
							&mut s,
							#field_name,
							&#value_serializer
						)?;
					}
				};

				let ser_field = match &f.attrs.skip_serializing_if {
					Some(predicate) => {
						let skip_field = if variant.is_some() {
							quote! {
								::serde::ser::SerializeStructVariant::skip_field(
									&mut s,
									#field_name
								)?;
							}
						} else {
							quote! {
								::serde::ser::SerializeStruct::skip_field(
									&mut s,
									#field_name
								)?;
							}
						};

						quote! {
							if #predicate ( #field_accessor ) {
								#serialize_field
							} else {
								#skip_field
							}
						}
					}
					None => serialize_field,
				};

				ser_fields.push(ser_field)
			}

			match variant {
				Some(variant) => {
					let variant_index = variant.index;
					let variant_name = variant.ident.to_string();

					Ok(quote! {
						let mut s = serializer.serialize_struct_variant(#name, #variant_index, #variant_name, #count_expr)?;

						#(#ser_fields)*

						::serde::ser::SerializeStructVariant::end(s)
					})
				}
				None => Ok(quote! {
					let mut s = serializer.serialize_struct(#name, #count_expr)?;

					#(#ser_fields)*

					::serde::ser::SerializeStruct::end(s)
				}),
			}
		}
	}
}
