//! This library provides types, traits and derive macros to deal with seeded
//! serialization/deserialization with serde.
//! - A `SerializeSeeded` trait and derive macro to serialize types with a seed.
//! - A `Seeded<Q, T>` type that implements `Serialize` calling
//!   `T::serialize_seeded` with a seed `Q`.
//! - A `DeserializeSeeded` trait and derive macro to deserialize types with a
//!   seed.
//! - A `Seed<Q, T>` type implementing `DeserializeSeed` calling
//!   `T::deserialize_seeded` with a seed `Q`.
//!
//! See the `tests` folder to find some examples.
#[cfg(feature = "derive")]
pub use serde_seeded_derive::{DeserializeSeeded, SerializeSeeded};

pub mod ser;
pub use ser::SerializeSeeded;

pub mod de;
pub use de::DeserializeSeeded;

pub mod unseeded {
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	pub fn serialize_seeded<T, Q, S>(value: &T, _seed: &Q, serializer: S) -> Result<S::Ok, S::Error>
	where
		T: Serialize,
		S: Serializer,
	{
		value.serialize(serializer)
	}

	pub fn deserialize_seeded<'de, T, Q, D>(_seed: &Q, deserializer: D) -> Result<T, D::Error>
	where
		T: Deserialize<'de>,
		D: Deserializer<'de>,
	{
		T::deserialize(deserializer)
	}
}

pub mod unseeded_btreemap_key {
	use crate::{de::Seed, ser::Seeded, DeserializeSeeded, SerializeSeeded};
	use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};
	use std::{collections::BTreeMap, marker::PhantomData};

	pub fn serialize_seeded<K, V, Q, S>(
		value: &BTreeMap<K, V>,
		seed: &Q,
		serializer: S,
	) -> Result<S::Ok, S::Error>
	where
		K: Serialize,
		V: SerializeSeeded<Q>,
		S: Serializer,
	{
		let mut s = serializer.serialize_map(Some(value.len()))?;

		for (key, value) in value {
			s.serialize_entry(key, &Seeded::new(seed, value))?;
		}

		s.end()
	}

	pub fn deserialize_seeded<'de, K, V, Q, D>(
		seed: &Q,
		deserializer: D,
	) -> Result<BTreeMap<K, V>, D::Error>
	where
		K: Ord + Deserialize<'de>,
		V: DeserializeSeeded<'de, Q>,
		D: Deserializer<'de>,
	{
		struct Visitor<'seed, Q, K, V>(&'seed Q, PhantomData<BTreeMap<K, V>>);

		impl<'de, 'seed, Q, K, V> ::serde::de::Visitor<'de> for Visitor<'seed, Q, K, V>
		where
			K: Ord + Deserialize<'de>,
			V: DeserializeSeeded<'de, Q>,
		{
			type Value = BTreeMap<K, V>;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "a map")
			}

			fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
			where
				A: serde::de::MapAccess<'de>,
			{
				let mut result = BTreeMap::new();

				while let Some(key) = map.next_key()? {
					let value = map.next_value_seed(Seed::new(self.0))?;
					result.insert(key, value);
				}

				Ok(result)
			}
		}

		deserializer.deserialize_map(Visitor(seed, PhantomData))
	}
}

pub mod unseeded_hashmap_key {
	use crate::{de::Seed, ser::Seeded, DeserializeSeeded, SerializeSeeded};
	use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};
	use std::{collections::HashMap, hash::Hash, marker::PhantomData};

	pub fn serialize_seeded<K, V, Q, S>(
		value: &HashMap<K, V>,
		seed: &Q,
		serializer: S,
	) -> Result<S::Ok, S::Error>
	where
		K: Serialize,
		V: SerializeSeeded<Q>,
		S: Serializer,
	{
		let mut s = serializer.serialize_map(Some(value.len()))?;

		for (key, value) in value {
			s.serialize_entry(key, &Seeded::new(seed, value))?;
		}

		s.end()
	}

	pub fn deserialize_seeded<'de, K, V, Q, D>(
		seed: &Q,
		deserializer: D,
	) -> Result<HashMap<K, V>, D::Error>
	where
		K: Eq + Hash + Deserialize<'de>,
		V: DeserializeSeeded<'de, Q>,
		D: Deserializer<'de>,
	{
		struct Visitor<'seed, Q, K, V>(&'seed Q, PhantomData<HashMap<K, V>>);

		impl<'de, 'seed, Q, K, V> ::serde::de::Visitor<'de> for Visitor<'seed, Q, K, V>
		where
			K: Eq + Hash + Deserialize<'de>,
			V: DeserializeSeeded<'de, Q>,
		{
			type Value = HashMap<K, V>;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "a map")
			}

			fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
			where
				A: serde::de::MapAccess<'de>,
			{
				let mut result = HashMap::new();

				while let Some(key) = map.next_key()? {
					let value = map.next_value_seed(Seed::new(self.0))?;
					result.insert(key, value);
				}

				Ok(result)
			}
		}

		deserializer.deserialize_map(Visitor(seed, PhantomData))
	}
}
