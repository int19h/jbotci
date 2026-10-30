//! Enforce raw text and wire byte limits during the same serialization pass.
use crate::{ByteLimit, OutputBudget, OutputLimitError};
use bityzba::{invariant, requires};
use serde::ser::*;
use serde::{Serialize, Serializer};
use std::sync::atomic::{AtomicBool, Ordering};

#[invariant(true)]
struct TextBudget {
    output: OutputBudget,
    exhausted: AtomicBool,
}
impl TextBudget {
    #[requires(true)]
    #[ensures(true)]
    fn charge(&self, bytes: usize) -> Result<(), OutputLimitError> {
        self.output.charge(bytes).inspect_err(|_| {
            self.exhausted.store(true, Ordering::Release);
        })
    }
}
/// A borrowed value carries the budget through every nested serializer call.
#[invariant(true)]
struct TextValue<'a, T: ?Sized> {
    value: &'a T,
    budget: &'a TextBudget,
}
impl<T: ?Sized + Serialize> Serialize for TextValue<'_, T> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Match serde_stacker's stack growth without nesting serializer adapters.
        stacker::maybe_grow(64 * 1024, 2 * 1024 * 1024, || {
            self.value.serialize(TextSerializer {
                inner: serializer,
                budget: self.budget,
            })
        })
    }
}
#[invariant(true)]
struct TextSerializer<'a, S> {
    inner: S,
    budget: &'a TextBudget,
}
#[invariant(true)]
struct TextCompound<'a, C> {
    inner: C,
    budget: &'a TextBudget,
}

/// Count UTF-8 string values. Field names and scalar values are protocol structure.
#[requires(true)]
#[ensures(true)]
pub fn check_json_text_budget<T: Serialize>(
    value: &T,
    limit: ByteLimit,
) -> Result<(), serde_json::Error> {
    let budget = TextBudget {
        output: OutputBudget::new(limit),
        exhausted: AtomicBool::new(false),
    };
    let mut serializer = serde_json::Serializer::new(std::io::sink());
    finish_text_serialization(
        TextValue {
            value,
            budget: &budget,
        }
        .serialize(&mut serializer),
        &budget,
    )
}
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bytes| bytes.len() <= wire.get()) || ret.is_err())]
pub fn bounded_json_with_text_budget<T: Serialize>(
    value: &T,
    text: ByteLimit,
    wire: ByteLimit,
) -> Result<Vec<u8>, serde_json::Error> {
    let budget = TextBudget {
        output: OutputBudget::new(text),
        exhausted: AtomicBool::new(false),
    };
    let mut writer = OutputBudget::new(wire).writer();
    let mut serializer = serde_json::Serializer::new(&mut writer);
    finish_text_serialization(
        TextValue {
            value,
            budget: &budget,
        }
        .serialize(&mut serializer),
        &budget,
    )?;
    Ok(writer.into_bytes())
}
#[requires(true)]
#[ensures(true)]
fn finish_text_serialization(
    result: Result<(), serde_json::Error>,
    budget: &TextBudget,
) -> Result<(), serde_json::Error> {
    if budget.exhausted.load(Ordering::Acquire) {
        Err(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::FileTooLarge,
            OutputLimitError {
                limit: budget.output.limit(),
            },
        )))
    } else {
        result
    }
}
impl<'a, S: Serializer> Serializer for TextSerializer<'a, S> {
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = TextCompound<'a, S::SerializeSeq>;
    type SerializeTuple = TextCompound<'a, S::SerializeTuple>;
    type SerializeTupleStruct = TextCompound<'a, S::SerializeTupleStruct>;
    type SerializeTupleVariant = TextCompound<'a, S::SerializeTupleVariant>;
    type SerializeMap = TextCompound<'a, S::SerializeMap>;
    type SerializeStruct = TextCompound<'a, S::SerializeStruct>;
    type SerializeStructVariant = TextCompound<'a, S::SerializeStructVariant>;
    #[requires(true)]
    #[ensures(true)]
    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_bool(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_i8(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_i16(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_i32(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_i64(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_i128(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_u8(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_u16(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_u32(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_u64(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_u128(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_f32(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_f64(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_bytes(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.budget.charge(value.len()).map_err(S::Error::custom)?;
        self.inner.serialize_str(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        self.budget
            .charge(value.len_utf8())
            .map_err(S::Error::custom)?;
        self.inner.serialize_char(value)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_none()
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_unit()
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_unit_struct(name)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_unit_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_unit_variant(name, index, variant)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_some(&TextValue {
            value,
            budget: self.budget,
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_newtype_struct(
            name,
            &TextValue {
                value,
                budget: self.budget,
            },
        )
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_newtype_variant(
            name,
            index,
            variant,
            &TextValue {
                value,
                budget: self.budget,
            },
        )
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.inner.serialize_seq(len).map(|inner| TextCompound {
            inner,
            budget: self.budget,
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.inner.serialize_tuple(len).map(|inner| TextCompound {
            inner,
            budget: self.budget,
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.inner
            .serialize_tuple_struct(name, len)
            .map(|inner| TextCompound {
                inner,
                budget: self.budget,
            })
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.inner
            .serialize_tuple_variant(name, index, variant, len)
            .map(|inner| TextCompound {
                inner,
                budget: self.budget,
            })
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.inner.serialize_map(len).map(|inner| TextCompound {
            inner,
            budget: self.budget,
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.inner
            .serialize_struct(name, len)
            .map(|inner| TextCompound {
                inner,
                budget: self.budget,
            })
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.inner
            .serialize_struct_variant(name, index, variant, len)
            .map(|inner| TextCompound {
                inner,
                budget: self.budget,
            })
    }
    #[requires(true)]
    #[ensures(true)]
    fn collect_str<T: ?Sized + std::fmt::Display>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let mut writer = self.budget.output.writer();
        if let Err(error) = std::io::Write::write_fmt(&mut writer, format_args!("{value}")) {
            if error.kind() == std::io::ErrorKind::FileTooLarge {
                self.budget.exhausted.store(true, Ordering::Release);
            }
            return Err(S::Error::custom(error));
        }
        let rendered = writer.into_string().expect("formatting writes UTF-8");
        self.inner.serialize_str(&rendered)
    }
    #[requires(true)]
    #[ensures(true)]
    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}
impl<C: SerializeSeq> SerializeSeq for TextCompound<'_, C> {
    type Ok = C::Ok;
    type Error = C::Error;
    #[requires(true)]
    #[ensures(true)]
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.inner.serialize_element(&TextValue {
            value,
            budget: self.budget,
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}
impl<C: SerializeTuple> SerializeTuple for TextCompound<'_, C> {
    type Ok = C::Ok;
    type Error = C::Error;
    #[requires(true)]
    #[ensures(true)]
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.inner.serialize_element(&TextValue {
            value,
            budget: self.budget,
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}
impl<C: SerializeTupleStruct> SerializeTupleStruct for TextCompound<'_, C> {
    type Ok = C::Ok;
    type Error = C::Error;
    #[requires(true)]
    #[ensures(true)]
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.inner.serialize_field(&TextValue {
            value,
            budget: self.budget,
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}
impl<C: SerializeTupleVariant> SerializeTupleVariant for TextCompound<'_, C> {
    type Ok = C::Ok;
    type Error = C::Error;
    #[requires(true)]
    #[ensures(true)]
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.inner.serialize_field(&TextValue {
            value,
            budget: self.budget,
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}
impl<C: SerializeStruct> SerializeStruct for TextCompound<'_, C> {
    type Ok = C::Ok;
    type Error = C::Error;
    #[requires(true)]
    #[ensures(true)]
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.inner.serialize_field(
            key,
            &TextValue {
                value,
                budget: self.budget,
            },
        )
    }
    #[requires(true)]
    #[ensures(true)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}
impl<C: SerializeStructVariant> SerializeStructVariant for TextCompound<'_, C> {
    type Ok = C::Ok;
    type Error = C::Error;
    #[requires(true)]
    #[ensures(true)]
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.inner.serialize_field(
            key,
            &TextValue {
                value,
                budget: self.budget,
            },
        )
    }
    #[requires(true)]
    #[ensures(true)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}
impl<C: SerializeMap> SerializeMap for TextCompound<'_, C> {
    type Ok = C::Ok;
    type Error = C::Error;
    #[requires(true)]
    #[ensures(true)]
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.inner.serialize_key(key)
    }
    #[requires(true)]
    #[ensures(true)]
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.inner.serialize_value(&TextValue {
            value,
            budget: self.budget,
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[invariant(true)]
    struct OnceText(std::cell::Cell<usize>);
    impl Serialize for OnceText {
        #[requires(true)]
        #[ensures(true)]
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let calls = self.0.get() + 1;
            self.0.set(calls);
            assert_eq!(calls, 1, "a response must use one serialization pass");
            serializer.collect_str(&format_args!("{}{}", "😀", "\"\n"))
        }
    }
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn formatted_fields_charge_raw_utf8_during_the_only_serialization_pass() {
        let value = OnceText(std::cell::Cell::new(0));
        let bytes = bounded_json_with_text_budget(
            &value,
            ByteLimit::new(6).unwrap(),
            ByteLimit::new(32).unwrap(),
        )
        .unwrap();
        assert_eq!(serde_json::from_slice::<String>(&bytes).unwrap(), "😀\"\n");
        let value = OnceText(std::cell::Cell::new(0));
        assert_eq!(
            bounded_json_with_text_budget(
                &value,
                ByteLimit::new(5).unwrap(),
                ByteLimit::new(32).unwrap()
            )
            .unwrap_err()
            .io_error_kind(),
            Some(std::io::ErrorKind::FileTooLarge)
        );
    }
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn text_and_wire_limits_count_different_representations() {
        let value = serde_json::json!({ "first": "😀", "second": "\n\"" });
        let expected = serde_json::to_vec(&value).unwrap();
        let text = ByteLimit::new(6).unwrap();
        let wire = ByteLimit::new(expected.len()).unwrap();
        assert_eq!(
            bounded_json_with_text_budget(&value, text, wire).unwrap(),
            expected
        );
        for result in [
            bounded_json_with_text_budget(&value, ByteLimit::new(5).unwrap(), wire),
            bounded_json_with_text_budget(
                &value,
                text,
                ByteLimit::new(expected.len() - 1).unwrap(),
            ),
        ] {
            assert_eq!(
                result.unwrap_err().io_error_kind(),
                Some(std::io::ErrorKind::FileTooLarge)
            );
        }
    }
}
