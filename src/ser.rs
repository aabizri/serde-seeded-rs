use std::collections::{BTreeMap, HashMap};

use serde::{
	ser::{SerializeMap, SerializeSeq},
	Serialize,
};

/// Seeded value, ready to be serialized.
///
/// This type implemented [`Serialize`] when `T` implements
/// [`SerializeSeeded<Q>`].
pub struct Seeded<'a, Q, T> {
	pub seed: &'a Q,
	pub value: T,
}

impl<'a, Q, T> Seeded<'a, Q, T> {
	/// Creates a new seeded value.
	pub fn new(seed: &'a Q, value: T) -> Self {
		Self { seed, value }
	}
}

impl<Q, T: Clone> Clone for Seeded<'_, Q, T> {
	fn clone(&self) -> Self {
		Self {
			seed: self.seed,
			value: self.value.clone(),
		}
	}
}

impl<Q, T: Copy> Copy for Seeded<'_, Q, T> {}

impl<Q, T> Serialize for Seeded<'_, Q, T>
where
	T: SerializeSeeded<Q>,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.value.serialize_seeded(self.seed, serializer)
	}
}

/// A data structure that can be serialized with a seed of type `Q`.
pub trait SerializeSeeded<Q> {
	/// Serializes the value using the given seed and serializer.
	fn serialize_seeded<S>(&self, seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer;
}

impl<Q, T> SerializeSeeded<Q> for &T
where
	T: SerializeSeeded<Q>,
{
	fn serialize_seeded<S>(&self, seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		T::serialize_seeded(*self, seed, serializer)
	}
}

impl<Q, T> SerializeSeeded<Q> for Box<T>
where
	T: SerializeSeeded<Q>,
{
	fn serialize_seeded<S>(&self, seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		T::serialize_seeded(self, seed, serializer)
	}
}

impl<Q> SerializeSeeded<Q> for () {
	fn serialize_seeded<S>(&self, _seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::Serialize::serialize(self, serializer)
	}
}

impl<Q> SerializeSeeded<Q> for bool {
	fn serialize_seeded<S>(&self, _seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::Serialize::serialize(self, serializer)
	}
}

impl<Q> SerializeSeeded<Q> for u32 {
	fn serialize_seeded<S>(&self, _seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::Serialize::serialize(self, serializer)
	}
}

impl<Q> SerializeSeeded<Q> for str {
	fn serialize_seeded<S>(&self, _seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::Serialize::serialize(self, serializer)
	}
}

impl<Q> SerializeSeeded<Q> for String {
	fn serialize_seeded<S>(&self, _seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::Serialize::serialize(self, serializer)
	}
}

impl<Q, T> SerializeSeeded<Q> for Option<T>
where
	T: SerializeSeeded<Q>,
{
	fn serialize_seeded<S>(&self, seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		match self {
			Some(t) => serializer.serialize_some(&Seeded::new(seed, t)),
			None => serializer.serialize_none(),
		}
	}
}

impl<Q, T> SerializeSeeded<Q> for [T]
where
	T: SerializeSeeded<Q>,
{
	fn serialize_seeded<S>(&self, seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut s = serializer.serialize_seq(Some(self.len()))?;

		for item in self {
			s.serialize_element(&Seeded::new(seed, item))?;
		}

		s.end()
	}
}

impl<Q, T> SerializeSeeded<Q> for Vec<T>
where
	T: SerializeSeeded<Q>,
{
	fn serialize_seeded<S>(&self, seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut s = serializer.serialize_seq(Some(self.len()))?;

		for item in self {
			s.serialize_element(&Seeded::new(seed, item))?;
		}

		s.end()
	}
}

impl<Q, K, V> SerializeSeeded<Q> for BTreeMap<K, V>
where
	K: SerializeSeeded<Q>,
	V: SerializeSeeded<Q>,
{
	fn serialize_seeded<S>(&self, seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut map = serializer.serialize_map(Some(self.len()))?;
		for (key, value) in self {
			map.serialize_entry(&Seeded::new(seed, key), &Seeded::new(seed, value))?;
		}
		map.end()
	}
}

impl<Q, K, V> SerializeSeeded<Q> for HashMap<K, V>
where
	K: SerializeSeeded<Q>,
	V: SerializeSeeded<Q>,
{
	fn serialize_seeded<S>(&self, seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut map = serializer.serialize_map(Some(self.len()))?;
		for (key, value) in self {
			map.serialize_entry(&Seeded::new(seed, key), &Seeded::new(seed, value))?;
		}
		map.end()
	}
}
