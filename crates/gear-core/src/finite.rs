//! **Every number a report holds, read for one that is not finite** — the
//! path of each not-a-number and infinity in anything serialisable, which is
//! everything that crosses the boundary. A report is a function of admitted
//! input, and a number past the doubles in one is a figure the model could
//! not form; the laws over the input table (`crate::input`) ask this of the
//! report at every row's extremes.

use serde::ser::{self, Serialize};

/// The path of every number in `t` that is not finite, `/`-joined field
/// names and list indices from the root. Empty where every one is.
#[must_use]
pub fn non_finite<T: Serialize + ?Sized>(t: &T) -> Vec<String> {
    let mut w = Walk {
        path: Vec::new(),
        found: Vec::new(),
    };
    // The walk refuses nothing, so the result is always `Ok`.
    let _ = t.serialize(&mut w);
    w.found
}

/// The walk: where it is, and what it found.
struct Walk {
    path: Vec<String>,
    found: Vec<String>,
}

impl Walk {
    fn number(&mut self, v: f64) {
        if !v.is_finite() {
            self.found.push(self.path.join("/"));
        }
    }

    fn at<T: Serialize + ?Sized>(&mut self, name: String, v: &T) -> Result<(), Never> {
        self.path.push(name);
        let r = v.serialize(&mut *self);
        self.path.pop();
        r
    }
}

/// The walk's error, which it never raises.
#[derive(Debug)]
pub struct Never;

impl std::fmt::Display for Never {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("never")
    }
}

impl std::error::Error for Never {}

impl ser::Error for Never {
    fn custom<M: std::fmt::Display>(_: M) -> Self {
        Self
    }
}

/// A compound value — a list, a map, a struct — its elements numbered or
/// named as they come.
struct Compound<'a> {
    walk: &'a mut Walk,
    index: usize,
    key: Option<String>,
}

impl<'a> ser::Serializer for &'a mut Walk {
    type Ok = ();
    type Error = Never;
    type SerializeSeq = Compound<'a>;
    type SerializeTuple = Compound<'a>;
    type SerializeTupleStruct = Compound<'a>;
    type SerializeTupleVariant = Compound<'a>;
    type SerializeMap = Compound<'a>;
    type SerializeStruct = Compound<'a>;
    type SerializeStructVariant = Compound<'a>;

    fn serialize_bool(self, _: bool) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_i8(self, _: i8) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_i16(self, _: i16) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_i32(self, _: i32) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_i64(self, _: i64) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_u8(self, _: u8) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_u16(self, _: u16) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_u32(self, _: u32) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_u64(self, _: u64) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<(), Never> {
        self.number(f64::from(v));
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> Result<(), Never> {
        self.number(v);
        Ok(())
    }
    fn serialize_char(self, _: char) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_str(self, _: &str) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_none(self) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<(), Never> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        v: &T,
    ) -> Result<(), Never> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        v: &T,
    ) -> Result<(), Never> {
        self.at(variant.to_owned(), v)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Compound<'a>, Never> {
        Ok(Compound {
            walk: self,
            index: 0,
            key: None,
        })
    }
    fn serialize_tuple(self, _: usize) -> Result<Compound<'a>, Never> {
        self.serialize_seq(None)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Compound<'a>, Never> {
        self.serialize_seq(None)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Compound<'a>, Never> {
        self.path.push(variant.to_owned());
        self.serialize_seq(None)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Compound<'a>, Never> {
        self.serialize_seq(None)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Compound<'a>, Never> {
        self.serialize_seq(None)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Compound<'a>, Never> {
        self.path.push(variant.to_owned());
        self.serialize_seq(None)
    }
}

impl Compound<'_> {
    fn element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        let name = self.index.to_string();
        self.index += 1;
        self.walk.at(name, v)
    }
}

impl ser::SerializeSeq for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        self.element(v)
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

impl ser::SerializeTuple for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        self.element(v)
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        self.element(v)
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

impl ser::SerializeTupleVariant for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        self.element(v)
    }
    fn end(self) -> Result<(), Never> {
        self.walk.path.pop();
        Ok(())
    }
}

impl ser::SerializeMap for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, k: &T) -> Result<(), Never> {
        // A map's key names its value where it is text; otherwise its place.
        self.key = serde_plain_key(k);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        match self.key.take() {
            Some(name) => self.walk.at(name, v),
            None => self.element(v),
        }
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

impl ser::SerializeStruct for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        name: &'static str,
        v: &T,
    ) -> Result<(), Never> {
        self.walk.at(name.to_owned(), v)
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

impl ser::SerializeStructVariant for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        name: &'static str,
        v: &T,
    ) -> Result<(), Never> {
        self.walk.at(name.to_owned(), v)
    }
    fn end(self) -> Result<(), Never> {
        self.walk.path.pop();
        Ok(())
    }
}

/// A map key as text, where it is a string; `None` otherwise.
fn serde_plain_key<T: Serialize + ?Sized>(k: &T) -> Option<String> {
    struct Key(Option<String>);
    impl ser::Serializer for &mut Key {
        type Ok = ();
        type Error = Never;
        type SerializeSeq = ser::Impossible<(), Never>;
        type SerializeTuple = ser::Impossible<(), Never>;
        type SerializeTupleStruct = ser::Impossible<(), Never>;
        type SerializeTupleVariant = ser::Impossible<(), Never>;
        type SerializeMap = ser::Impossible<(), Never>;
        type SerializeStruct = ser::Impossible<(), Never>;
        type SerializeStructVariant = ser::Impossible<(), Never>;
        fn serialize_str(self, v: &str) -> Result<(), Never> {
            self.0 = Some(v.to_owned());
            Ok(())
        }
        fn serialize_bool(self, _: bool) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_i8(self, _: i8) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_i16(self, _: i16) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_i32(self, _: i32) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_i64(self, _: i64) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_u8(self, _: u8) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_u16(self, _: u16) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_u32(self, _: u32) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_u64(self, _: u64) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_f32(self, _: f32) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_f64(self, _: f64) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_char(self, _: char) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_bytes(self, _: &[u8]) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_none(self) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<(), Never> {
            v.serialize(self)
        }
        fn serialize_unit(self) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_unit_struct(self, _: &'static str) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_unit_variant(
            self,
            _: &'static str,
            _: u32,
            variant: &'static str,
        ) -> Result<(), Never> {
            self.0 = Some(variant.to_owned());
            Ok(())
        }
        fn serialize_newtype_struct<T: Serialize + ?Sized>(
            self,
            _: &'static str,
            v: &T,
        ) -> Result<(), Never> {
            v.serialize(self)
        }
        fn serialize_newtype_variant<T: Serialize + ?Sized>(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: &T,
        ) -> Result<(), Never> {
            Ok(())
        }
        fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Never> {
            Err(Never)
        }
        fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Never> {
            Err(Never)
        }
        fn serialize_tuple_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleStruct, Never> {
            Err(Never)
        }
        fn serialize_tuple_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleVariant, Never> {
            Err(Never)
        }
        fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Never> {
            Err(Never)
        }
        fn serialize_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStruct, Never> {
            Err(Never)
        }
        fn serialize_struct_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStructVariant, Never> {
            Err(Never)
        }
    }
    let mut key = Key(None);
    let _ = k.serialize(&mut key);
    key.0
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[derive(serde::Serialize)]
    struct Leaf {
        a: f64,
        b: Option<f64>,
        c: Vec<[f64; 2]>,
    }

    /// **Every not-finite number is found, by its path, and nothing else**:
    /// in a field, an option, a list of pairs, a map's value and an enum's
    /// variant — and a report whose every number is finite has none.
    #[test]
    fn every_number_past_the_doubles_is_found_by_its_path() {
        #[derive(serde::Serialize)]
        enum Kind {
            Line { at: f64 },
        }
        #[derive(serde::Serialize)]
        struct Report {
            leaf: Leaf,
            kinds: Vec<Kind>,
            map: std::collections::BTreeMap<String, f64>,
        }
        let mut map = std::collections::BTreeMap::new();
        map.insert("x".to_owned(), f64::NEG_INFINITY);
        map.insert("y".to_owned(), 1.0);
        let r = Report {
            leaf: Leaf {
                a: f64::NAN,
                b: Some(f64::INFINITY),
                c: vec![[1.0, 2.0], [3.0, f64::NAN]],
            },
            kinds: vec![Kind::Line { at: 1.0 }, Kind::Line { at: f64::NAN }],
            map,
        };
        assert_eq!(
            non_finite(&r),
            ["leaf/a", "leaf/b", "leaf/c/1/1", "kinds/1/Line/at", "map/x"]
        );
        let fine = Leaf {
            a: 0.0,
            b: None,
            c: vec![[f64::MAX, -f64::MIN_POSITIVE]],
        };
        assert!(non_finite(&fine).is_empty());
    }
}
