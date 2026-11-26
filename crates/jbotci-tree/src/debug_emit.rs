//! One buffer for a hand-written `Debug` walk.
//!
//! Compact mode writes each byte once. Pretty mode writes each output byte
//! once. The pad count is the pretty nesting depth. A newline sets every
//! open pad, which is how `Debug` indents a nested value.
//!
//! A field value keeps its pad. `end_field`, `end_tuple_field`, and `end_item`
//! write `",\n"` and then drop that pad. `core::fmt` builders do the same.

use std::fmt::Debug;

use bityzba::{invariant, new, requires};

#[invariant(true)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameKind {
    Struct,
    Tuple,
    List,
}

/// One open `Debug` frame.
///
/// Every kind and every field flag is a legal open frame. The caller order
/// is a precondition of `Emit`, not a constraint on this pair.
#[invariant(true)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Frame {
    kind: FrameKind,
    has_fields: bool,
}

#[invariant(*true_suffix <= *pad_len)]
#[invariant(*pretty || *pad_len == 0)]
#[invariant(*pad_len <= frames.len())]
#[derive(Debug)]
pub struct Emit {
    buf: String,
    pretty: bool,
    pad_len: usize,
    true_suffix: usize,
    frames: Vec<Frame>,
}

impl Emit {
    #[requires(true)]
    #[ensures(!ret.pretty || ret.pad_len == 0)]
    #[ensures(ret.frames.is_empty())]
    pub fn new(pretty: bool) -> Self {
        new!(Self {
            buf: String::new(),
            pretty: pretty,
            pad_len: 0,
            true_suffix: 0,
            frames: Vec::new(),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn finish(self) -> String {
        self.into_data().buf
    }

    #[requires(!name.is_empty())]
    #[ensures(self.frames.len() == old(self.frames.len()) + 1)]
    pub fn begin_struct(&mut self, name: &str) {
        self.write(name);
        self.push_frame(FrameKind::Struct);
    }

    #[requires(!name.is_empty())]
    #[ensures(self.frames.len() == old(self.frames.len()) + 1)]
    pub fn begin_tuple(&mut self, name: &str) {
        self.write(name);
        self.push_frame(FrameKind::Tuple);
    }

    #[requires(true)]
    #[ensures(self.frames.len() == old(self.frames.len()) + 1)]
    pub fn begin_list(&mut self) {
        self.write("[");
        self.push_frame(FrameKind::List);
    }

    #[requires(!name.is_empty())]
    #[requires(self.frames.last().is_some_and(|frame| frame.kind == FrameKind::Struct))]
    #[requires(self.pad_len < self.frames.len())]
    #[ensures(self.frames.last().is_some_and(|frame| frame.has_fields))]
    pub fn begin_field(&mut self, name: &str) {
        let has_fields = self
            .frames
            .last()
            .expect("a field belongs to a struct")
            .has_fields;
        if self.pretty {
            if !has_fields {
                self.write(" {\n");
            }
        } else if has_fields {
            self.write(", ");
        } else {
            self.write(" { ");
        }
        self.mark_fields();
        if self.pretty {
            self.push_pad();
        }
        self.write(name);
        self.write(": ");
    }

    #[requires(self.frames.last().is_some_and(|frame| frame.kind == FrameKind::Struct && frame.has_fields))]
    #[requires(!self.pretty || (self.pad_len > 0 && self.pad_len == self.frames.len()))]
    #[ensures(self.pad_len + usize::from(self.pretty) == old(self.pad_len))]
    pub fn end_field(&mut self) {
        // The comma stays in the field pad. `PadAdapter` writes it before the
        // adapter drops, and a fresh adapter starts on a newline.
        if self.pretty {
            self.write(",\n");
            self.pop_pad();
        }
    }

    #[requires(self.frames.last().is_some_and(|frame| frame.kind == FrameKind::Tuple))]
    #[requires(self.pad_len < self.frames.len())]
    #[ensures(self.frames.last().is_some_and(|frame| frame.has_fields))]
    pub fn begin_tuple_field(&mut self) {
        let has_fields = self
            .frames
            .last()
            .expect("a tuple field belongs to a tuple")
            .has_fields;
        if self.pretty {
            if !has_fields {
                self.write("(\n");
            }
        } else if has_fields {
            self.write(", ");
        } else {
            self.write("(");
        }
        self.mark_fields();
        if self.pretty {
            self.push_pad();
        }
    }

    #[requires(self.frames.last().is_some_and(|frame| frame.kind == FrameKind::Tuple && frame.has_fields))]
    #[requires(!self.pretty || (self.pad_len > 0 && self.pad_len == self.frames.len()))]
    #[ensures(self.pad_len + usize::from(self.pretty) == old(self.pad_len))]
    pub fn end_tuple_field(&mut self) {
        // The comma stays in the field pad. `PadAdapter` writes it before the
        // adapter drops, and a fresh adapter starts on a newline.
        if self.pretty {
            self.write(",\n");
            self.pop_pad();
        }
    }

    #[requires(self.frames.last().is_some_and(|frame| frame.kind == FrameKind::List))]
    #[requires(self.pad_len < self.frames.len())]
    #[ensures(self.frames.last().is_some_and(|frame| frame.has_fields))]
    pub fn begin_item(&mut self) {
        let has_fields = self
            .frames
            .last()
            .expect("a list item belongs to a list")
            .has_fields;
        if self.pretty {
            if !has_fields {
                self.write("\n");
            }
        } else if has_fields {
            self.write(", ");
        }
        self.mark_fields();
        if self.pretty {
            self.push_pad();
        }
    }

    #[requires(self.frames.last().is_some_and(|frame| frame.kind == FrameKind::List && frame.has_fields))]
    #[requires(!self.pretty || (self.pad_len > 0 && self.pad_len == self.frames.len()))]
    #[ensures(self.pad_len + usize::from(self.pretty) == old(self.pad_len))]
    pub fn end_item(&mut self) {
        // The comma stays in the field pad. `PadAdapter` writes it before the
        // adapter drops, and a fresh adapter starts on a newline.
        if self.pretty {
            self.write(",\n");
            self.pop_pad();
        }
    }

    #[requires(self.frames.last().is_some_and(|frame| frame.kind == FrameKind::Struct))]
    #[requires(self.pad_len < self.frames.len())]
    #[ensures(self.frames.len() + 1 == old(self.frames.len()))]
    pub fn end_struct(&mut self) {
        let frame = self.pop_frame(FrameKind::Struct);
        if frame.has_fields {
            if self.pretty {
                self.write("}");
            } else {
                self.write(" }");
            }
        }
    }

    #[requires(self.frames.last().is_some_and(|frame| frame.kind == FrameKind::Tuple))]
    #[requires(self.pad_len < self.frames.len())]
    #[ensures(self.frames.len() + 1 == old(self.frames.len()))]
    pub fn end_tuple(&mut self) {
        let frame = self.pop_frame(FrameKind::Tuple);
        if frame.has_fields {
            self.write(")");
        }
    }

    #[requires(self.frames.last().is_some_and(|frame| frame.kind == FrameKind::List))]
    #[requires(self.pad_len < self.frames.len())]
    #[ensures(self.frames.len() + 1 == old(self.frames.len()))]
    pub fn end_list(&mut self) {
        let _frame = self.pop_frame(FrameKind::List);
        self.write("]");
    }

    #[requires(true)]
    #[ensures(self.pad_len == old(self.pad_len))]
    #[ensures(self.frames.len() == old(self.frames.len()))]
    pub fn debug_value(&mut self, value: &(impl Debug + ?Sized)) {
        let text = if self.pretty {
            format!("{value:#?}")
        } else {
            format!("{value:?}")
        };
        self.write(&text);
    }

    #[requires(self.pretty)]
    #[requires(self.pad_len < self.frames.len())]
    #[ensures(self.pad_len == old(self.pad_len) + 1)]
    #[ensures(self.true_suffix == old(self.true_suffix) + 1)]
    fn push_pad(&mut self) {
        let mut data = take(self);
        data.pad_len += 1;
        data.true_suffix += 1;
        *self = Self::from_data(data);
    }

    #[requires(self.pad_len > 0)]
    #[ensures(self.pad_len + 1 == old(self.pad_len))]
    fn pop_pad(&mut self) {
        let mut data = take(self);
        if data.true_suffix > 0 {
            data.true_suffix -= 1;
        }
        data.pad_len -= 1;
        *self = Self::from_data(data);
    }

    #[requires(true)]
    #[ensures(self.pad_len == old(self.pad_len))]
    #[ensures(self.frames.len() == old(self.frames.len()))]
    fn write(&mut self, text: &str) {
        let mut data = take(self);
        if text.is_empty() || data.pad_len == 0 {
            data.buf.push_str(text);
            *self = Self::from_data(data);
            return;
        }
        for chunk in text.split_inclusive('\n') {
            for _ in 0..data.true_suffix {
                data.buf.push_str("    ");
            }
            data.buf.push_str(chunk);
            data.true_suffix = if chunk.ends_with('\n') {
                data.pad_len
            } else {
                0
            };
        }
        *self = Self::from_data(data);
    }

    #[requires(self.pad_len <= self.frames.len())]
    #[ensures(self.frames.len() == old(self.frames.len()) + 1)]
    fn push_frame(&mut self, kind: FrameKind) {
        let mut data = take(self);
        data.frames.push(Frame {
            kind,
            has_fields: false,
        });
        *self = Self::from_data(data);
    }

    #[requires(self.frames.last().is_some())]
    #[ensures(self.frames.last().is_some_and(|frame| frame.has_fields))]
    fn mark_fields(&mut self) {
        let mut data = take(self);
        data.frames
            .last_mut()
            .expect("a field belongs to an open frame")
            .has_fields = true;
        *self = Self::from_data(data);
    }

    #[requires(self.frames.last().is_some_and(|frame| frame.kind == kind))]
    #[requires(self.pad_len < self.frames.len())]
    #[ensures(self.frames.len() + 1 == old(self.frames.len()))]
    fn pop_frame(&mut self, kind: FrameKind) -> Frame {
        let mut data = take(self);
        let frame = data.frames.pop().expect("a frame is open");
        assert!(
            frame.kind == kind,
            "the closed frame has the requested kind"
        );
        *self = Self::from_data(data);
        frame
    }
}

#[requires(true)]
#[ensures(ret.buf.is_empty() && !ret.pretty && ret.pad_len == 0 && ret.frames.is_empty())]
fn blank_data() -> EmitData {
    EmitData {
        buf: String::new(),
        pretty: false,
        pad_len: 0,
        true_suffix: 0,
        frames: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn take(emit: &mut Emit) -> EmitData {
    std::mem::replace(emit, Emit::from_data(blank_data())).into_data()
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use bityzba::{invariant, requires};

    use super::Emit;

    /// The fixture has one text field. Any name is valid test data.
    #[invariant(true)]
    #[derive(Debug)]
    struct Sample {
        name: &'static str,
    }

    /// The fixture has two numbers. Any pair is valid test data.
    #[invariant(true)]
    #[derive(Debug)]
    struct Pair(u8, u8);

    /// The test value writes no text. The struct holds no data.
    #[invariant(true)]
    struct EmptyLeaf;

    impl fmt::Debug for EmptyLeaf {
        #[requires(true)]
        #[ensures(true)]
        fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            Ok(())
        }
    }

    /// The test value writes one fixed line. The struct holds no data.
    #[invariant(true)]
    struct TrailingNewlineLeaf;

    impl fmt::Debug for TrailingNewlineLeaf {
        #[requires(true)]
        #[ensures(true)]
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("CustomLeaf\n")
        }
    }

    /// One empty field. The wrapper exists so derived `Debug` builds a tuple.
    #[invariant(true)]
    #[derive(Debug)]
    struct HeldEmpty(EmptyLeaf);

    /// One field that ends in a newline. Derived `Debug` builds the tuple.
    #[invariant(true)]
    #[derive(Debug)]
    struct HeldNewline(TrailingNewlineLeaf);

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn emit_matches_derived_debug() {
        for pretty in [false, true] {
            let sample = Sample { name: "mi" };
            let mut emit = Emit::new(pretty);
            emit.begin_struct("Sample");
            emit.begin_field("name");
            emit.debug_value(&sample.name);
            emit.end_field();
            emit.end_struct();
            assert_same("struct", pretty, emit, &sample);

            let pair = Pair(1, 2);
            let mut emit = Emit::new(pretty);
            emit.begin_tuple("Pair");
            emit.begin_tuple_field();
            emit.debug_value(&pair.0);
            emit.end_tuple_field();
            emit.begin_tuple_field();
            emit.debug_value(&pair.1);
            emit.end_tuple_field();
            emit.end_tuple();
            assert_same("tuple", pretty, emit, &pair);

            let values = vec![1u8, 2];
            let mut emit = Emit::new(pretty);
            emit.begin_list();
            for value in &values {
                emit.begin_item();
                emit.debug_value(value);
                emit.end_item();
            }
            emit.end_list();
            assert_same("list", pretty, emit, &values);

            let empty = HeldEmpty(EmptyLeaf);
            let mut emit = Emit::new(pretty);
            emit.begin_tuple("HeldEmpty");
            emit.begin_tuple_field();
            emit.debug_value(&empty.0);
            emit.end_tuple_field();
            emit.end_tuple();
            assert_same("empty leaf", pretty, emit, &empty);

            let newline = HeldNewline(TrailingNewlineLeaf);
            let mut emit = Emit::new(pretty);
            emit.begin_tuple("HeldNewline");
            emit.begin_tuple_field();
            emit.debug_value(&newline.0);
            emit.end_tuple_field();
            emit.end_tuple();
            assert_same("newline leaf", pretty, emit, &newline);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_same(label: &str, pretty: bool, emit: Emit, derived: &impl fmt::Debug) {
        let expect = if pretty {
            format!("{derived:#?}")
        } else {
            format!("{derived:?}")
        };
        assert_eq!(emit.finish(), expect, "{label} pretty={pretty}");
    }
}
