use proc_macro2::Span;
use syn::{punctuated::Punctuated, spanned::Spanned, Token, WherePredicate};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("expected an attribute of the form `#[seeded(...)]`")]
	ExpectedMetaList(Span),

	#[error(transparent)]
	Parse(#[from] syn::parse::Error),

	#[error("missing seed type")]
	MissingSeed,
}

impl Error {
	pub fn span(&self) -> Span {
		match self {
			Self::ExpectedMetaList(s) => *s,
			Self::Parse(e) => e.span(),
			Self::MissingSeed => Span::call_site(),
		}
	}
}

#[derive(Default)]
pub struct FieldAttributes {
	pub skip: bool,
	pub default: bool,
	pub with: Option<syn::Path>,
	pub skip_serializing_if: Option<syn::Path>,
	pub rename: Option<String>,
}

impl FieldAttributes {
	pub fn parse_attributes(attrs: &[syn::Attribute]) -> Result<Self, Error> {
		let mut result = Self::default();

		for attr in attrs {
			if attr.path().is_ident("seeded") {
				match &attr.meta {
					syn::Meta::List(list) => {
						let a: Self = syn::parse2(list.tokens.clone())?;
						result.merge_with(a)
					}
					_ => return Err(Error::ExpectedMetaList(attr.span())),
				}
			}
		}

		Ok(result)
	}

	pub fn merge_with(&mut self, other: Self) {
		self.skip |= other.skip;
		self.default |= other.default;

		if let Some(path) = other.with {
			self.with = Some(path)
		}

		if let Some(path) = other.skip_serializing_if {
			self.skip_serializing_if = Some(path)
		}

		if let Some(name) = other.rename {
			self.rename = Some(name)
		}
	}

	pub fn name(&self, ident: &syn::Ident) -> String {
		match &self.rename {
			Some(name) => name.clone(),
			None => {
				let name = ident.to_string();

				match name.strip_prefix("r#") {
					Some(suffix) => suffix.to_owned(),
					None => name,
				}
			}
		}
	}
}

impl syn::parse::Parse for FieldAttributes {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let mut result = Self::default();

		let attributes: Punctuated<FieldAttribute, Token![,]> =
			Punctuated::parse_terminated(input)?;
		for attr in attributes {
			match attr {
				FieldAttribute::Skip => result.skip = true,
				FieldAttribute::Default => result.default = true,
				FieldAttribute::With(path) => {
					result.with = Some(path);
				}
				FieldAttribute::SkipSerializingIf(path) => {
					result.skip_serializing_if = Some(path);
				}
				FieldAttribute::Rename(name) => result.rename = Some(name.value()),
			}
		}

		Ok(result)
	}
}

pub enum FieldAttribute {
	Skip,
	Default,
	With(syn::Path),
	SkipSerializingIf(syn::Path),
	Rename(syn::LitStr),
}

impl syn::parse::Parse for FieldAttribute {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let id: syn::Ident = input.parse()?;
		if id == "skip" {
			Ok(Self::Skip)
		} else if id == "default" {
			Ok(Self::Default)
		} else if id == "with" {
			let content;
			let _ = syn::parenthesized!(content in input);
			content.parse().map(Self::With)
		} else if id == "skip_serializing_if" {
			let content;
			let _ = syn::parenthesized!(content in input);
			content.parse().map(Self::SkipSerializingIf)
		} else if id == "rename" {
			let content;
			let _ = syn::parenthesized!(content in input);
			content.parse().map(Self::Rename)
		} else {
			Err(syn::Error::new(id.span(), "unexpected ident"))
		}
	}
}

#[derive(Default)]
pub struct TypeAttributes {
	pub ser: Vec<SerializeAttributes>,
	pub de: Vec<SerializeAttributes>,
	pub transparent: bool,
	pub rename: Option<String>,
}

impl TypeAttributes {
	pub fn parse_attributes(attrs: &[syn::Attribute]) -> Result<Self, Error> {
		let mut result = Self::default();

		for attr in attrs {
			if attr.path().is_ident("seeded") {
				match &attr.meta {
					syn::Meta::List(list) => {
						let a: Self = syn::parse2(list.tokens.clone())?;
						result.merge_with(a)
					}
					_ => return Err(Error::ExpectedMetaList(attr.span())),
				}
			}
		}

		Ok(result)
	}

	pub fn merge_with(&mut self, other: Self) {
		self.ser.extend(other.ser);
		self.de.extend(other.de);
		self.transparent |= other.transparent;

		if let Some(name) = other.rename {
			self.rename = Some(name)
		}
	}

	pub fn name(&self, ident: &syn::Ident) -> String {
		match &self.rename {
			Some(name) => name.clone(),
			None => ident.to_string(),
		}
	}
}

impl syn::parse::Parse for TypeAttributes {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let mut result = Self::default();

		let attributes: Punctuated<TypeAttribute, Token![,]> = Punctuated::parse_terminated(input)?;
		for attr in attributes {
			match attr {
				TypeAttribute::Ser(a) => {
					result.ser.push(a);
				}
				TypeAttribute::De(a) => result.de.push(a),
				TypeAttribute::Serde(a) => {
					result.ser.push(a.clone());
					result.de.push(a);
				}
				TypeAttribute::Transparent => result.transparent = true,
				TypeAttribute::Rename(name) => result.rename = Some(name.value()),
			}
		}

		Ok(result)
	}
}

pub enum TypeAttribute {
	Ser(SerializeAttributes),
	De(SerializeAttributes),
	Serde(SerializeAttributes),
	Transparent,
	Rename(syn::LitStr),
}

impl syn::parse::Parse for TypeAttribute {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let id: syn::Ident = input.parse()?;
		if id == "ser" {
			let content;
			let _ = syn::parenthesized!(content in input);
			SerializeAttributes::parse(&content).map(Self::Ser)
		} else if id == "de" {
			let content;
			let _ = syn::parenthesized!(content in input);
			SerializeAttributes::parse(&content).map(Self::De)
		} else if id == "serde" {
			let content;
			let _ = syn::parenthesized!(content in input);
			SerializeAttributes::parse(&content).map(Self::Serde)
		} else if id == "transparent" {
			Ok(Self::Transparent)
		} else if id == "rename" {
			let content;
			let _ = syn::parenthesized!(content in input);
			content.parse().map(Self::Rename)
		} else {
			Err(syn::Error::new(id.span(), "unexpected ident"))
		}
	}
}

#[derive(Default, Clone)]
pub struct SerializeAttributes {
	pub seed: Option<syn::Type>,
	pub params: Vec<syn::GenericParam>,
	pub bounds: Vec<WherePredicate>,
	pub override_bounds: Vec<WherePredicate>,
}

impl SerializeAttributes {
	pub fn require_seed(&self) -> Result<&syn::Type, Error> {
		self.seed
			.as_ref()
			.or(self.seed.as_ref())
			.ok_or(Error::MissingSeed)
	}
}

impl syn::parse::Parse for SerializeAttributes {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let mut result = Self::default();

		let attributes: Punctuated<SerializeAttribute, Token![,]> =
			Punctuated::parse_terminated(input)?;
		for attr in attributes {
			match attr {
				SerializeAttribute::Seed(ty) => {
					result.seed = Some(ty);
				}
				SerializeAttribute::Params(params) => {
					result.params.extend(params);
				}
				SerializeAttribute::Bounds(predicates) => {
					result.bounds = predicates.into_iter().collect();
				}
				SerializeAttribute::OverrideBounds(predicates) => {
					result.override_bounds = predicates.into_iter().collect();
				}
			}
		}

		Ok(result)
	}
}

pub enum SerializeAttribute {
	Seed(syn::Type),
	Params(Punctuated<syn::GenericParam, Token![,]>),
	Bounds(Punctuated<WherePredicate, Token![,]>),
	OverrideBounds(Punctuated<WherePredicate, Token![,]>),
}

impl syn::parse::Parse for SerializeAttribute {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let id: syn::Ident = input.parse()?;
		if id == "seed" {
			let content;
			let _ = syn::parenthesized!(content in input);
			let ty = syn::Type::parse(&content)?;
			Ok(Self::Seed(ty))
		} else if id == "params" {
			let content;
			let _ = syn::parenthesized!(content in input);
			let params = syn::punctuated::Punctuated::parse_terminated(&content)?;
			Ok(Self::Params(params))
		} else if id == "bounds" {
			let content;
			let _ = syn::parenthesized!(content in input);
			let predicates: Punctuated<WherePredicate, Token![,]> =
				Punctuated::parse_terminated(&content)?;
			Ok(Self::Bounds(predicates))
		} else if id == "override_bounds" {
			let content;
			let _ = syn::parenthesized!(content in input);
			let predicates: Punctuated<WherePredicate, Token![,]> =
				Punctuated::parse_terminated(&content)?;
			Ok(Self::OverrideBounds(predicates))
		} else {
			Err(syn::Error::new(id.span(), "unexpected ident"))
		}
	}
}
