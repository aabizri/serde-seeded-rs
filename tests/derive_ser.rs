#![no_std]
#![allow(dead_code)]
use serde_seeded::SerializeSeeded;

pub struct Seed;

pub struct Seeded<T>(pub T);

impl<'de, T> SerializeSeeded<Seed> for Seeded<T> {
	fn serialize_seeded<S>(&self, _seed: &Seed, _serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		unimplemented!()
	}
}

#[derive(SerializeSeeded)]
#[seeded(ser(seed(Seed)))]
pub struct Unit;

#[derive(SerializeSeeded)]
#[seeded(ser(seed(Seed)))]
pub struct Newtype(Seeded<u32>);

#[derive(SerializeSeeded)]
#[seeded(ser(seed(Seed)))]
pub struct Tuple(Seeded<u32>, Seeded<bool>);

#[derive(SerializeSeeded)]
#[seeded(ser(seed(Seed)))]
pub struct Struct {
	foo: Seeded<bool>,
	bar: Seeded<u32>,
}

#[derive(SerializeSeeded)]
#[seeded(ser(seed(Seed)))]
pub enum Bar {
	Unit,
	Newtype(Seeded<u32>),
	Tuple(Seeded<u32>, Seeded<bool>),
	Struct {
		foo: Seeded<u32>,
		#[seeded(skip_serializing_if(Option::is_none))]
		bar: Option<Seeded<bool>>,
	},
}
