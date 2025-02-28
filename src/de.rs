use serde::de::DeserializeSeed;

/// Seed deserializing any `T` implementing `DeserializeSeeded<Q>`.
///
/// This type implements [`DeserializeSeed`] when `T` implements
/// [`DeserializeSeeded<Q>`].
pub struct Seed<'a, Q: ?Sized, T> {
	seed: &'a Q,
	t: core::marker::PhantomData<T>,
}

impl<'a, Q: ?Sized, T> Seed<'a, Q, T> {
	/// Creates a new deserializing seed.
	pub fn new(seed: &'a Q) -> Self {
		Self {
			seed,
			t: core::marker::PhantomData,
		}
	}
}

impl<Q: ?Sized, T> Clone for Seed<'_, Q, T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<Q: ?Sized, T> Copy for Seed<'_, Q, T> {}

impl<'de, Q, T> DeserializeSeed<'de> for Seed<'_, Q, T>
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

/// Any type that can be deserialized without that seed (meaning they implement [`serde::Deserialize`]),
/// automatically implement [`DeserializeSeeded`].
impl<'de, Q, T> DeserializeSeeded<'de, Q> for T
where
	Q: ?Sized,
	T: serde::Deserialize<'de>,
{
	fn deserialize_seeded<D>(_seed: &Q, deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		T::deserialize(deserializer)
	}
}
