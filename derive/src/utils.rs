use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, TokenStreamExt};

// pub fn split_for_def_and_impl<'a>(
// 	generics: &'a syn::Generics,
// 	impl_params: impl Into<SeedParams>,
// 	type_params: impl Into<SeedParams>
// ) -> (SeededImplGenerics<'a>, SeededImplGenerics<'a>, SeededTypeGenerics<'a>, Option<&'a syn::WhereClause>) {
// 	let type_params = type_params.into();
// 	(
// 		SeededImplGenerics(generics, type_params),
// 		SeededImplGenerics(generics, impl_params.into()),
// 		SeededTypeGenerics(generics, type_params),
// 		generics.where_clause.as_ref(),
// 	)
// }

// pub fn split_for_impl<'a>(
// 	generics: &'a syn::Generics,
// 	impl_params: impl Into<SeedParams>,
// 	type_params: impl Into<SeedParams>
// ) -> (SeededImplGenerics<'a>, SeededTypeGenerics<'a>, Option<&'a syn::WhereClause>) {
// 	(
// 		SeededImplGenerics(generics, impl_params.into()),
// 		SeededTypeGenerics(generics, type_params.into()),
// 		generics.where_clause.as_ref(),
// 	)
// }

macro_rules! seed_params {
	{ $( $field:ident : $variant:ident ),* } => {
		#[derive(Clone, Copy)]
		#[allow(clippy::enum_variant_names)]
		pub enum SeedParam {
			$($variant),*
		}

		#[derive(Debug, Default, Clone, Copy)]
		pub struct SeedParams {
			$($field: bool),*
		}

		impl From<SeedParam> for SeedParams {
			fn from(p: SeedParam) -> Self {
				match p {
					$(
						SeedParam::$variant => Self {
							$field: true,
							..Default::default()
						}
					),*
				}
			}
		}

		impl SeedParams {
			pub fn is_empty(&self) -> bool {
				$(
					!self.$field
				)&&*
			}
		}

		impl std::ops::BitOr for SeedParam {
			type Output = SeedParams;

			fn bitor(self, rhs: Self) -> Self::Output {
				SeedParams::from(self) | SeedParams::from(rhs)
			}
		}

		impl std::ops::BitOr<SeedParam> for SeedParams {
			type Output = Self;

			fn bitor(self, rhs: SeedParam) -> Self::Output {
				self | SeedParams::from(rhs)
			}
		}


		impl std::ops::BitOrAssign<SeedParam> for SeedParams {
			fn bitor_assign(&mut self, rhs: SeedParam) {
				*self |= SeedParams::from(rhs);
			}
		}

		impl std::ops::BitOr for SeedParams {
			type Output = Self;

			fn bitor(mut self, rhs: Self) -> Self::Output {
				$(self.$field |= rhs.$field;)*
				self
			}
		}

		impl std::ops::BitOrAssign for SeedParams {
			fn bitor_assign(&mut self, rhs: Self) {
				$(self.$field |= rhs.$field;)*
			}
		}
	};
}

seed_params! {
	value_lft: ValueLifetime,
	seed_lft: SeedLifetime,
	de_lft: DeLifetime
}

pub struct SeededImplGenerics<'a> {
	generics: &'a syn::Generics,
	seed_params: SeedParams,
	extra_params: &'a [syn::GenericParam],
}

impl<'a> SeededImplGenerics<'a> {
	pub fn new(generics: &'a syn::Generics) -> Self {
		Self {
			generics,
			seed_params: SeedParams::default(),
			extra_params: &[],
		}
	}

	pub fn with(mut self, params: impl Into<SeedParams>) -> Self {
		self.seed_params |= params.into();
		self
	}

	pub fn with_extra_params(mut self, params: &'a [syn::GenericParam]) -> Self {
		self.extra_params = params;
		self
	}
}

impl ToTokens for SeededImplGenerics<'_> {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		if self.generics.params.is_empty()
			&& self.seed_params.is_empty()
			&& self.extra_params.is_empty()
		{
			return;
		}

		TokensOrDefault(&self.generics.lt_token).to_tokens(tokens);

		// Print lifetimes before types and consts, regardless of their
		// order in self.params.
		let mut trailing_or_empty = true;

		if self.seed_params.value_lft {
			syn::Lifetime::new("'value", Span::call_site()).to_tokens(tokens);
			<syn::Token![,]>::default().to_tokens(tokens);
			trailing_or_empty = true;
		}

		if self.seed_params.seed_lft {
			syn::Lifetime::new("'seed", Span::call_site()).to_tokens(tokens);
			<syn::Token![,]>::default().to_tokens(tokens);
			trailing_or_empty = true;
		}

		if self.seed_params.de_lft {
			syn::Lifetime::new("'de", Span::call_site()).to_tokens(tokens);
			<syn::Token![,]>::default().to_tokens(tokens);
			trailing_or_empty = true;
		}

		for param in self.generics.params.pairs() {
			if let syn::GenericParam::Lifetime(_) = param.value() {
				param.to_tokens(tokens);
				trailing_or_empty = param.punct().is_some();
			}
		}

		for param in self.extra_params {
			if let syn::GenericParam::Lifetime(_) = param {
				param.to_tokens(tokens);
				<syn::Token![,]>::default().to_tokens(tokens);
				trailing_or_empty = true
			}
		}

		for param in self.generics.params.pairs() {
			if let syn::GenericParam::Lifetime(_) = **param.value() {
				continue;
			}
			if !trailing_or_empty {
				<syn::Token![,]>::default().to_tokens(tokens);
			}
			match param.value() {
				syn::GenericParam::Lifetime(_) => unreachable!(),
				syn::GenericParam::Type(param) => {
					// Leave off the type parameter defaults
					tokens.append_all(param.attrs.outer());
					param.ident.to_tokens(tokens);
					if !param.bounds.is_empty() {
						TokensOrDefault(&param.colon_token).to_tokens(tokens);
						param.bounds.to_tokens(tokens);
					}
				}
				syn::GenericParam::Const(param) => {
					// Leave off the const parameter defaults
					tokens.append_all(param.attrs.outer());
					param.const_token.to_tokens(tokens);
					param.ident.to_tokens(tokens);
					param.colon_token.to_tokens(tokens);
					param.ty.to_tokens(tokens);
				}
			}
			param.punct().to_tokens(tokens);
			trailing_or_empty = param.punct().is_some();
		}

		for param in self.extra_params {
			if let syn::GenericParam::Lifetime(_) = param {
				continue;
			}
			if !trailing_or_empty {
				<syn::Token![,]>::default().to_tokens(tokens);
				trailing_or_empty = true;
			}
			match param {
				syn::GenericParam::Lifetime(_) => unreachable!(),
				syn::GenericParam::Type(param) => {
					// Leave off the type parameter defaults
					tokens.append_all(param.attrs.outer());
					param.ident.to_tokens(tokens);
					if !param.bounds.is_empty() {
						TokensOrDefault(&param.colon_token).to_tokens(tokens);
						param.bounds.to_tokens(tokens);
					}
				}
				syn::GenericParam::Const(param) => {
					// Leave off the const parameter defaults
					tokens.append_all(param.attrs.outer());
					param.const_token.to_tokens(tokens);
					param.ident.to_tokens(tokens);
					param.colon_token.to_tokens(tokens);
					param.ty.to_tokens(tokens);
				}
			}
			<syn::Token![,]>::default().to_tokens(tokens);
		}

		TokensOrDefault(&self.generics.gt_token).to_tokens(tokens);
	}
}

pub struct SeededTypeGenerics<'a> {
	generics: &'a syn::Generics,
	params: SeedParams,
	extra_params: &'a [syn::GenericParam],
}

impl<'a> SeededTypeGenerics<'a> {
	pub fn new(generics: &'a syn::Generics) -> Self {
		Self {
			generics,
			params: SeedParams::default(),
			extra_params: &[],
		}
	}

	pub fn with(mut self, params: impl Into<SeedParams>) -> Self {
		self.params |= params.into();
		self
	}

	pub fn with_extra_params(mut self, params: &'a [syn::GenericParam]) -> Self {
		self.extra_params = params;
		self
	}
}

impl ToTokens for SeededTypeGenerics<'_> {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		if self.generics.params.is_empty() && self.params.is_empty() && self.extra_params.is_empty()
		{
			return;
		}

		TokensOrDefault(&self.generics.lt_token).to_tokens(tokens);

		// Print lifetimes before types and consts, regardless of their
		// order in self.params.
		let mut trailing_or_empty = true;

		if self.params.value_lft {
			syn::Lifetime::new("'value", Span::call_site()).to_tokens(tokens);
			<syn::Token![,]>::default().to_tokens(tokens);
			trailing_or_empty = true;
		}

		if self.params.seed_lft {
			syn::Lifetime::new("'seed", Span::call_site()).to_tokens(tokens);
			<syn::Token![,]>::default().to_tokens(tokens);
			trailing_or_empty = true;
		}

		if self.params.de_lft {
			syn::Lifetime::new("'de", Span::call_site()).to_tokens(tokens);
			<syn::Token![,]>::default().to_tokens(tokens);
			trailing_or_empty = true;
		}

		for param in self.generics.params.pairs() {
			if let syn::GenericParam::Lifetime(def) = param.value() {
				// Leave off the lifetime bounds and attributes
				def.lifetime.to_tokens(tokens);
				param.punct().to_tokens(tokens);
				trailing_or_empty = param.punct().is_some();
			}
		}

		for param in self.extra_params {
			if let syn::GenericParam::Lifetime(def) = param {
				// Leave off the lifetime bounds and attributes
				def.lifetime.to_tokens(tokens);
				<syn::Token![,]>::default().to_tokens(tokens);
				trailing_or_empty = true;
			}
		}

		for param in self.generics.params.pairs() {
			if let syn::GenericParam::Lifetime(_) = **param.value() {
				continue;
			}
			if !trailing_or_empty {
				<syn::Token![,]>::default().to_tokens(tokens);
			}
			match param.value() {
				syn::GenericParam::Lifetime(_) => unreachable!(),
				syn::GenericParam::Type(param) => {
					// Leave off the type parameter defaults
					param.ident.to_tokens(tokens);
				}
				syn::GenericParam::Const(param) => {
					// Leave off the const parameter defaults
					param.ident.to_tokens(tokens);
				}
			}
			param.punct().to_tokens(tokens);
			trailing_or_empty = param.punct().is_some();
		}

		for param in self.extra_params {
			if let syn::GenericParam::Lifetime(_) = param {
				continue;
			}
			if !trailing_or_empty {
				<syn::Token![,]>::default().to_tokens(tokens);
				trailing_or_empty = true;
			}
			match param {
				syn::GenericParam::Lifetime(_) => unreachable!(),
				syn::GenericParam::Type(param) => {
					// Leave off the type parameter defaults
					param.ident.to_tokens(tokens);
				}
				syn::GenericParam::Const(param) => {
					// Leave off the const parameter defaults
					param.ident.to_tokens(tokens);
				}
			}
			<syn::Token![,]>::default().to_tokens(tokens);
		}

		TokensOrDefault(&self.generics.gt_token).to_tokens(tokens);
	}
}

pub(crate) struct TokensOrDefault<'a, T: 'a>(pub &'a Option<T>);

impl<T> ToTokens for TokensOrDefault<'_, T>
where
	T: ToTokens + Default,
{
	fn to_tokens(&self, tokens: &mut TokenStream) {
		match self.0 {
			Some(t) => t.to_tokens(tokens),
			None => T::default().to_tokens(tokens),
		}
	}
}

pub(crate) trait FilterAttrs<'a> {
	type Ret: Iterator<Item = &'a syn::Attribute>;

	fn outer(self) -> Self::Ret;
}

impl<'a> FilterAttrs<'a> for &'a [syn::Attribute] {
	type Ret =
		std::iter::Filter<std::slice::Iter<'a, syn::Attribute>, fn(&&syn::Attribute) -> bool>;

	fn outer(self) -> Self::Ret {
		fn is_outer(attr: &&syn::Attribute) -> bool {
			match attr.style {
				syn::AttrStyle::Outer => true,
				syn::AttrStyle::Inner(_) => false,
			}
		}
		self.iter().filter(is_outer)
	}
}

pub trait TryFilterMapExt: Sized + Iterator {
	fn try_filter_map<F, U, E>(self, f: F) -> TryFilterMap<Self, F>
	where
		F: FnMut(Self::Item) -> Result<Option<U>, E>;
}

impl<I: Iterator> TryFilterMapExt for I {
	fn try_filter_map<F, U, E>(self, f: F) -> TryFilterMap<Self, F>
	where
		F: FnMut(Self::Item) -> Result<Option<U>, E>,
	{
		TryFilterMap { inner: self, f }
	}
}

pub struct TryFilterMap<I, F> {
	inner: I,
	f: F,
}

impl<I: Iterator, F, U, E> Iterator for TryFilterMap<I, F>
where
	F: FnMut(I::Item) -> Result<Option<U>, E>,
{
	type Item = Result<U, E>;

	fn next(&mut self) -> Option<Self::Item> {
		(self.f)(self.inner.next()?).transpose()
	}
}
