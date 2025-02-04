use std::{collections::BTreeMap, marker::PhantomData};

use serde::de::DeserializeSeed;

/// Seed deserializing any `T` implementing `DeserializeSeeded<Q>`.
///
/// This type implements [`DeserializeSeed`] when `T` implements
/// [`DeserializeSeeded<Q>`].
pub struct Seed<'a, Q: ?Sized, T> {
	seed: &'a Q,
	t: PhantomData<T>,
}

impl<'a, Q: ?Sized, T> Seed<'a, Q, T> {
	/// Creates a new deserializing seed.
	pub fn new(seed: &'a Q) -> Self {
		Self {
			seed,
			t: PhantomData,
		}
	}
}

impl<'a, Q: ?Sized, T> Clone for Seed<'a, Q, T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<'a, Q: ?Sized, T> Copy for Seed<'a, Q, T> {}

impl<'de, 'a, Q, T> DeserializeSeed<'de> for Seed<'a, Q, T>
where
	Q: ?Sized,
	T: DeserializeSeeded<'de, Q>,
{
	type Value = T;

	fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		T::deserialize_seeded(self.seed, deserializer)
	}
}

/// A data structure that can be deserialized with a seed of type `Q`.
pub trait DeserializeSeeded<'de, Q: ?Sized>: Sized {
	/// Deserializes `Self` using the given seed and deserializer.
	fn deserialize_seeded<D>(seed: &Q, deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>;
}

impl<'de, Q, T> DeserializeSeeded<'de, Q> for Box<T>
where
	Q: ?Sized,
	T: DeserializeSeeded<'de, Q>,
{
	fn deserialize_seeded<D>(seed: &Q, deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		T::deserialize_seeded(seed, deserializer).map(Box::new)
	}
}

impl<'de, Q, T> DeserializeSeeded<'de, Q> for Option<T>
where
	Q: ?Sized,
	T: DeserializeSeeded<'de, Q>,
{
	fn deserialize_seeded<D>(seed: &Q, deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		struct Visitor<'seed, Q: ?Sized, T>(&'seed Q, PhantomData<T>);

		impl<'de, 'seed, Q, T> serde::de::Visitor<'de> for Visitor<'seed, Q, T>
		where
			Q: ?Sized,
			T: DeserializeSeeded<'de, Q>,
		{
			type Value = Option<T>;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "an optional value")
			}

			fn visit_none<E>(self) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				Ok(None)
			}

			fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
			where
				D: serde::Deserializer<'de>,
			{
				T::deserialize_seeded(self.0, deserializer).map(Some)
			}
		}

		deserializer.deserialize_option(Visitor(seed, PhantomData))
	}
}

impl<'de, Q: ?Sized> DeserializeSeeded<'de, Q> for () {
	fn deserialize_seeded<D>(_seed: &Q, deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		serde::Deserialize::deserialize(deserializer)
	}
}

impl<'de, Q: ?Sized> DeserializeSeeded<'de, Q> for bool {
	fn deserialize_seeded<D>(_seed: &Q, deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		serde::Deserialize::deserialize(deserializer)
	}
}

impl<'de, Q: ?Sized> DeserializeSeeded<'de, Q> for u32 {
	fn deserialize_seeded<D>(_seed: &Q, deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		serde::Deserialize::deserialize(deserializer)
	}
}

impl<'de, Q: ?Sized> DeserializeSeeded<'de, Q> for String {
	fn deserialize_seeded<D>(_seed: &Q, deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		serde::Deserialize::deserialize(deserializer)
	}
}

impl<'de, Q, T> DeserializeSeeded<'de, Q> for Vec<T>
where
	Q: ?Sized,
	T: DeserializeSeeded<'de, Q>,
{
	fn deserialize_seeded<D>(seed: &Q, deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		struct Visitor<'a, Q: ?Sized, T>(Seed<'a, Q, T>);

		impl<'de, 'a, Q, T> serde::de::Visitor<'de> for Visitor<'a, Q, T>
		where
			Q: ?Sized,
			T: DeserializeSeeded<'de, Q>,
		{
			type Value = Vec<T>;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "a sequence")
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: serde::de::SeqAccess<'de>,
			{
				let mut result = Vec::new();

				while let Some(item) = seq.next_element_seed(self.0)? {
					result.push(item)
				}

				Ok(result)
			}
		}

		deserializer.deserialize_seq(Visitor(Seed::new(seed)))
	}
}

impl<'de, Q, K, V> DeserializeSeeded<'de, Q> for BTreeMap<K, V>
where
	Q: ?Sized,
	K: Ord + DeserializeSeeded<'de, Q>,
	V: DeserializeSeeded<'de, Q>,
{
	fn deserialize_seeded<D>(seed: &Q, deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		struct Visitor<'a, Q: ?Sized, K, V>(Seed<'a, Q, K>, Seed<'a, Q, V>);

		impl<'de, 'a, Q, K, V> serde::de::Visitor<'de> for Visitor<'a, Q, K, V>
		where
			Q: ?Sized,
			K: Ord + DeserializeSeeded<'de, Q>,
			V: DeserializeSeeded<'de, Q>,
		{
			type Value = BTreeMap<K, V>;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "a sequence")
			}

			fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
			where
				A: serde::de::MapAccess<'de>,
			{
				let mut result = BTreeMap::new();

				while let Some((key, value)) = map.next_entry_seed(self.0, self.1)? {
					result.insert(key, value);
				}

				Ok(result)
			}
		}

		deserializer.deserialize_seq(Visitor(Seed::new(seed), Seed::new(seed)))
	}
}
