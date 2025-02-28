#![no_std]
#![allow(dead_code)]
use serde_seeded::DeserializeSeeded;

pub struct Seed;

pub struct Seeded<T>(pub T);

impl<'de, T> DeserializeSeeded<'de, Seed> for Seeded<T> {
	fn deserialize_seeded<D>(_seed: &Seed, _deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		unimplemented!()
	}
}

#[derive(DeserializeSeeded)]
#[seeded(de(seed(Seed)))]
pub struct Unit;

#[derive(DeserializeSeeded)]
#[seeded(de(seed(Seed)))]
pub struct Newtype(Seeded<u32>);

#[derive(DeserializeSeeded)]
#[seeded(de(seed(Seed)))]
pub struct Tuple(Seeded<u32>, Seeded<bool>);

#[derive(DeserializeSeeded)]
#[seeded(de(seed(Seed)))]
pub struct Struct {
	foo: Seeded<bool>,
	bar: Seeded<u32>,
}

#[derive(DeserializeSeeded)]
#[seeded(de(seed(Seed)))]
pub enum Bar {
	Unit,
	Newtype(Seeded<u32>),
	Tuple(Seeded<u32>, Seeded<bool>),
	Struct { foo: Seeded<u32>, bar: Seeded<bool> },
}
