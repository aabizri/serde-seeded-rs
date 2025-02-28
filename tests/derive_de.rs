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
static_assertions::assert_impl_all!(Unit: DeserializeSeeded<'static, Seed>);

#[derive(DeserializeSeeded)]
#[seeded(de(seed(Seed)))]
pub struct Newtype(Seeded<u32>);
static_assertions::assert_impl_all!(Newtype: DeserializeSeeded<'static, Seed>);

#[derive(DeserializeSeeded)]
#[seeded(de(seed(Seed)))]
pub struct Tuple(Seeded<u32>, Seeded<bool>);
static_assertions::assert_impl_all!(Tuple: DeserializeSeeded<'static, Seed>);

#[derive(DeserializeSeeded)]
#[seeded(de(seed(Seed)))]
pub struct Struct {
	foo: Seeded<bool>,
	bar: Seeded<u32>,
}
static_assertions::assert_impl_all!(Struct: DeserializeSeeded<'static, Seed>);

#[derive(DeserializeSeeded)]
#[seeded(de(seed(Seed)))]
pub enum Bar {
	Unit,
	Newtype(Seeded<u32>),
	Tuple(Seeded<u32>, Seeded<bool>),
	Struct { foo: Seeded<u32>, bar: Seeded<bool> },
}
static_assertions::assert_impl_all!(Bar: DeserializeSeeded<'static, Seed>);

struct Foo;

impl<'de> serde::Deserialize<'de> for Foo {
	fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		unimplemented!()
	}
}

#[derive(DeserializeSeeded)]
#[seeded(de(seed(Seed)))]
pub struct HybridStruct {
	foo: Seeded<bool>,
	bar: Seeded<u32>,
	text: Foo,
}
static_assertions::assert_impl_all!(HybridStruct: DeserializeSeeded<'static, Seed>);
