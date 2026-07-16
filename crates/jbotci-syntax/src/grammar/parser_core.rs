//! Parser execution core specialized for the generated syntax grammar.
//!
//! This module is derived from the subset of chumsky 0.13.0 that jbotci used.
//! Portions are Copyright (c) 2021 Joshua Barretto and distributed under the
//! MIT License. The specialized implementation preserves the relevant
//! parser, checkpoint, error-routing, and combinator semantics while removing
//! configurations and parser kinds that the syntax grammar never instantiated.

use std::{
    borrow::Cow,
    cell::{OnceCell, RefCell},
    cmp::Ordering,
    fmt,
    marker::PhantomData,
    ops::{Deref, Range},
    rc::{Rc, Weak},
};

use bityzba::{contract_trait, data, invariant, new, requires};

use super::{ParserCheckpoint, ParserState, Token, parse_error::SyntaxParseError};

/// A rule output shared between the active parse and its packrat memo entry.
///
/// Generated grammar adapters only materialize an owned value when a parent
/// parser needs to consume it. Cloning this wrapper for memo store or replay is
/// therefore constant-time regardless of the output tree's size.
#[invariant(
    Rc::strong_count(value) >= 1,
    "a shared parser output must own a live allocation"
)]
pub(crate) struct SharedSyntaxOutput<O> {
    value: Rc<O>,
}

impl<O> SharedSyntaxOutput<O> {
    #[requires(true)]
    #[ensures(Rc::strong_count(&ret.value) == 1)]
    pub(crate) fn new(value: O) -> Self {
        new!(SharedSyntaxOutput {
            value: Rc::new(value),
        })
    }

    #[requires(true)]
    #[ensures(Rc::as_ptr(&ret.value) == old(Rc::as_ptr(&value)))]
    pub(crate) fn from_shared(value: Rc<O>) -> Self {
        new!(SharedSyntaxOutput { value })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn into_shared(self) -> Rc<O> {
        self.into_data().value
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn into_owned(self) -> O
    where
        O: Clone,
    {
        Rc::try_unwrap(self.into_shared()).unwrap_or_else(|value| (*value).clone())
    }
}

impl<O> Clone for SharedSyntaxOutput<O> {
    #[requires(true)]
    #[ensures(Rc::ptr_eq(&ret.value, &self.value))]
    fn clone(&self) -> Self {
        new!(SharedSyntaxOutput {
            value: Rc::clone(&self.value),
        })
    }
}

impl<O: fmt::Debug> fmt::Debug for SharedSyntaxOutput<O> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(formatter)
    }
}

/// A Chumsky-compatible byte span in the syntax input.
///
/// Empty spans between tokens can have the next token's start offset followed
/// by the preceding token's end offset. Keeping that representation, including
/// the resulting inverted range, is required for diagnostic compatibility.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct SimpleSpan {
    pub start: usize,
    pub end: usize,
}

impl From<Range<usize>> for SimpleSpan {
    #[requires(true)]
    #[ensures(ret.start == value.start)]
    #[ensures(ret.end == value.end)]
    fn from(value: Range<usize>) -> Self {
        SimpleSpan {
            start: value.start,
            end: value.end,
        }
    }
}

/// A value paired with its source span.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Spanned<T, S = SimpleSpan> {
    pub inner: T,
    pub span: S,
}

impl<T, S> Spanned<T, S> {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn into_inner(self) -> T {
        self.inner
    }
}

/// An owned token or a borrowed token used while constructing parser errors.
#[invariant(::Ref(_) => true)]
#[invariant(::Val(_) => true)]
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum MaybeRef<'tokens, T> {
    Ref(&'tokens T),
    Val(T),
}

impl<T> Deref for MaybeRef<'_, T> {
    type Target = T;

    #[requires(true)]
    #[ensures(true)]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Ref(value) => value,
            Self::Val(value) => value,
        }
    }
}

impl<T> fmt::Debug for MaybeRef<'_, T>
where
    T: fmt::Debug,
{
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(self, formatter)
    }
}

impl<T> From<T> for MaybeRef<'_, T> {
    #[requires(true)]
    #[ensures(matches!(ret, MaybeRef::Val(_)))]
    fn from(value: T) -> Self {
        Self::Val(value)
    }
}

impl<'tokens, T> From<&'tokens T> for MaybeRef<'tokens, T> {
    #[requires(true)]
    #[ensures(matches!(ret, MaybeRef::Ref(_)))]
    fn from(value: &'tokens T) -> Self {
        Self::Ref(value)
    }
}

impl<T: Clone> MaybeRef<'_, T> {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn into_inner(self) -> T {
        match self {
            Self::Ref(value) => value.clone(),
            Self::Val(value) => value,
        }
    }
}

/// The expected-pattern subset used by syntax errors and parser labels.
#[invariant(::Label(_) => true)]
#[invariant(::EndOfInput => true)]
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum RichPattern<'tokens> {
    Label(Cow<'tokens, str>),
    EndOfInput,
}

impl TryFrom<&'static str> for RichPattern<'_> {
    type Error = ();

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn try_from(label: &'static str) -> Result<Self, Self::Error> {
        Ok(Self::Label(Cow::Borrowed(label)))
    }
}

impl TryFrom<String> for RichPattern<'_> {
    type Error = ();

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn try_from(label: String) -> Result<Self, Self::Error> {
        Ok(Self::Label(Cow::Owned(label)))
    }
}

impl fmt::Debug for RichPattern<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Label(label) => write!(formatter, "{label}"),
            Self::EndOfInput => formatter.write_str("end of input"),
        }
    }
}

impl fmt::Display for RichPattern<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Label(label) => write!(formatter, "{label}"),
            Self::EndOfInput => formatter.write_str("end of input"),
        }
    }
}

/// The error-reason subset consumed by syntax diagnostics.
#[invariant(::ExpectedFound { .. } => true)]
#[invariant(::Custom(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RichReason<'tokens, T, C = String> {
    ExpectedFound {
        expected: Vec<RichPattern<'tokens>>,
        found: Option<MaybeRef<'tokens, T>>,
    },
    Custom(C),
}

/// Error behavior required by parser alternative routing.
#[contract_trait]
pub(crate) trait Error<'tokens>: Sized + LabelError<'tokens, RichPattern<'tokens>> {
    #[requires(true)]
    #[ensures(true)]
    fn merge(self, other: Self) -> Self;
}

/// Error construction and labelling required by parser primitives.
#[contract_trait]
pub(crate) trait LabelError<'tokens, L>: Sized {
    #[requires(true)]
    #[ensures(true)]
    fn expected_found<E: IntoIterator<Item = L>>(
        expected: E,
        found: Option<MaybeRef<'tokens, Token>>,
        span: SimpleSpan,
    ) -> Self;

    #[requires(true)]
    #[ensures(true)]
    fn merge_expected_found<E: IntoIterator<Item = L>>(
        self,
        expected: E,
        found: Option<MaybeRef<'tokens, Token>>,
        span: SimpleSpan,
    ) -> Self
    where
        Self: Error<'tokens>,
    {
        self.merge(Self::expected_found(expected, found, span))
    }

    #[requires(true)]
    #[ensures(true)]
    fn replace_expected_found<E: IntoIterator<Item = L>>(
        self,
        expected: E,
        found: Option<MaybeRef<'tokens, Token>>,
        span: SimpleSpan,
    ) -> Self {
        Self::expected_found(expected, found, span)
    }

    #[requires(true)]
    #[ensures(true)]
    fn label_with(&mut self, _label: L) {}

    #[requires(true)]
    #[ensures(true)]
    fn in_context(&mut self, _label: L, _span: SimpleSpan) {}
}

/// The only input adapter instantiated by the syntax parser.
#[invariant(eoi.start == eoi.end, "mapped syntax input carries a zero-width EOF span")]
#[derive(Clone, Copy)]
pub(crate) struct MappedInput<'tokens> {
    tokens: &'tokens [Spanned<Token>],
    eoi: SimpleSpan,
}

impl MappedInput<'_> {
    #[requires(true)]
    #[ensures(ret == cursor.index)]
    pub(crate) fn cursor_location(cursor: &CursorInner) -> usize {
        cursor.index
    }

    #[requires(start.index <= end.index)]
    #[ensures(true)]
    fn span_between(&self, start: CursorInner, end: CursorInner) -> SimpleSpan {
        match self.tokens.get(start.index) {
            Some(token) => SimpleSpan::from(token.span.start..end.last_end.unwrap_or(self.eoi.end)),
            None => SimpleSpan::from(self.eoi.end..self.eoi.end),
        }
    }
}

/// Converts the sole raw token-slice input into its mapped representation.
#[contract_trait]
pub(crate) trait Input<'tokens> {
    #[requires(eoi.start == eoi.end)]
    #[ensures(true)]
    fn split_spanned(self, eoi: SimpleSpan) -> MappedInput<'tokens>
    where
        Self: Sized;
}

#[contract_trait]
impl<'tokens> Input<'tokens> for &'tokens [Spanned<Token>] {
    fn split_spanned(self, eoi: SimpleSpan) -> MappedInput<'tokens> {
        new!(MappedInput { tokens: self, eoi })
    }
}

/// The mapped-input cursor state needed to reproduce Chumsky span construction.
#[invariant(last_end.is_none() -> *index == 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CursorInner {
    index: usize,
    last_end: Option<usize>,
}

/// A parse location tied to one invocation of the parser driver.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cursor<'tokens, 'parse> {
    inner: CursorInner,
    marker: PhantomData<(&'tokens (), &'parse mut &'parse ())>,
}

impl Cursor<'_, '_> {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn inner(&self) -> &CursorInner {
        &self.inner
    }
}

/// A rewind point for input, inspector state, and emitted errors.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct Checkpoint<'tokens, 'parse> {
    cursor: Cursor<'tokens, 'parse>,
    inspector: ParserCheckpoint,
}

impl<'tokens, 'parse> Checkpoint<'tokens, 'parse> {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn cursor(&self) -> &Cursor<'tokens, 'parse> {
        &self.cursor
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn inspector(&self) -> &ParserCheckpoint {
        &self.inspector
    }
}

/// Parser-state hooks used when tokens are consumed and checkpoints rewind.
#[contract_trait]
pub(crate) trait Inspector<'tokens> {
    #[requires(true)]
    #[ensures(true)]
    fn on_token(&mut self, token: &Token);

    #[requires(true)]
    #[ensures(true)]
    fn on_save<'parse>(&self, cursor: &Cursor<'tokens, 'parse>) -> ParserCheckpoint;

    #[requires(true)]
    #[ensures(true)]
    fn on_rewind<'parse>(&mut self, checkpoint: &Checkpoint<'tokens, 'parse>);
}

#[invariant(true)]
struct LocatedError<'tokens> {
    position: CursorInner,
    error: SyntaxParseError<'tokens>,
}

#[invariant(true)]
#[derive(Default)]
struct Errors<'tokens> {
    alternative: Option<LocatedError<'tokens>>,
}

/// Mutable access to one parser invocation.
#[invariant(true)]
pub(crate) struct InputRef<'tokens, 'parse> {
    input: MappedInput<'tokens>,
    cursor: CursorInner,
    errors: &'parse mut Errors<'tokens>,
    state: &'parse mut ParserState<'tokens>,
}

impl<'tokens, 'parse> InputRef<'tokens, 'parse> {
    #[requires(true)]
    #[ensures(true)]
    #[inline(always)]
    pub(crate) fn cursor(&self) -> Cursor<'tokens, 'parse> {
        Cursor {
            inner: self.cursor,
            marker: PhantomData,
        }
    }

    #[requires(true)]
    #[ensures(ret.cursor().inner().index == self.cursor.index)]
    #[inline(always)]
    pub(crate) fn save(&self) -> Checkpoint<'tokens, 'parse> {
        let cursor = self.cursor();
        Checkpoint {
            cursor,
            inspector: self.state.on_save(&cursor),
        }
    }

    #[requires(true)]
    #[ensures(self.cursor.index == checkpoint.cursor.inner.index)]
    #[inline(always)]
    pub(crate) fn rewind(&mut self, checkpoint: Checkpoint<'tokens, 'parse>) {
        self.state.on_rewind(&checkpoint);
        self.cursor = checkpoint.cursor.inner;
    }

    #[requires(true)]
    #[ensures(true)]
    #[inline(always)]
    pub(crate) fn state(&mut self) -> &mut ParserState<'tokens> {
        self.state
    }

    #[requires(true)]
    #[ensures(ret.is_some() -> self.cursor.index == old(self.cursor.index) + 1)]
    #[ensures(ret.is_none() -> self.cursor.index == old(self.cursor.index))]
    #[inline(always)]
    pub(crate) fn next(&mut self) -> Option<Token> {
        let token = self.input.tokens.get(self.cursor.index)?.clone();
        self.cursor = self.cursor.with_data(data! {
            index: self.cursor.index + 1,
            last_end: Some(token.span.end),
        });
        self.state.on_token(&token.inner);
        Some(token.into_inner())
    }

    /// Reports whether the next token is the unmatchable completion sentinel.
    /// Terminal parsers reject it, while the root EOF parser sees the token and
    /// therefore forces a parse failure at the completion cut.
    #[requires(true)]
    #[ensures(ret == self.state.is_continuation_sentinel_location(self.cursor.index))]
    #[inline(always)]
    pub(crate) fn next_is_continuation_sentinel(&self) -> bool {
        self.state
            .is_continuation_sentinel_location(self.cursor.index)
    }

    #[requires(true)]
    #[ensures(self.cursor.index >= old(self.cursor.index))]
    #[inline(always)]
    pub(crate) fn skip(&mut self) {
        let _ = self.next();
    }

    #[requires(before.inner.index <= self.cursor.index)]
    #[ensures(true)]
    #[inline(always)]
    pub(crate) fn span_since(&self, before: &Cursor<'tokens, 'parse>) -> SimpleSpan {
        self.input.span_between(before.inner, self.cursor)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn parse<O, P>(&mut self, parser: P) -> Result<O, SyntaxParseError<'tokens>>
    where
        P: Parser<'tokens, O>,
    {
        match parser.drive_emit(self) {
            Ok(output) => Ok(output),
            Err(()) => Err(self
                .take_alternative()
                .expect("failed parsers register a primary error")
                .error),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_expected<L, E>(
        &mut self,
        expected: E,
        found: Option<MaybeRef<'tokens, Token>>,
        span: SimpleSpan,
    ) where
        E: IntoIterator<Item = L>,
        SyntaxParseError<'tokens>: LabelError<'tokens, L>,
    {
        let position = self.cursor;
        self.errors.alternative = Some(match self.errors.alternative.take() {
            Some(alternative) => match alternative.position.index.cmp(&position.index) {
                Ordering::Equal => LocatedError {
                    position: alternative.position,
                    error: alternative
                        .error
                        .merge_expected_found(expected, found, span),
                },
                Ordering::Greater => alternative,
                Ordering::Less => LocatedError {
                    position,
                    error: alternative
                        .error
                        .replace_expected_found(expected, found, span),
                },
            },
            None => LocatedError {
                position,
                error: <SyntaxParseError<'tokens> as LabelError<'tokens, L>>::expected_found(
                    expected, found, span,
                ),
            },
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_alternative_error(&mut self, position: CursorInner, error: SyntaxParseError<'tokens>) {
        self.errors.alternative = Some(match self.errors.alternative.take() {
            Some(alternative) => match alternative.position.index.cmp(&position.index) {
                Ordering::Equal => LocatedError {
                    position: alternative.position,
                    error: alternative.error.merge(error),
                },
                Ordering::Greater => alternative,
                Ordering::Less => LocatedError { position, error },
            },
            None => LocatedError { position, error },
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn take_alternative(&mut self) -> Option<LocatedError<'tokens>> {
        self.errors.alternative.take()
    }
}

/// State access supplied to `map_with` callbacks.
#[invariant(true)]
pub(crate) struct MapExtra<'tokens, 'parse> {
    state: &'parse mut ParserState<'tokens>,
}

impl<'tokens> MapExtra<'tokens, '_> {
    #[requires(true)]
    #[ensures(true)]
    #[inline(always)]
    pub(crate) fn state(&mut self) -> &mut ParserState<'tokens> {
        self.state
    }
}

/// The result surface used by syntax parser entry points.
#[invariant(output.is_some() || !errors.is_empty())]
pub(crate) struct ParseResult<'tokens, O> {
    output: Option<O>,
    errors: Vec<SyntaxParseError<'tokens>>,
}

impl<'tokens, O> ParseResult<'tokens, O> {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.as_ref().err().is_some_and(|errors| !errors.is_empty()))]
    pub(crate) fn into_result(self) -> Result<O, Vec<SyntaxParseError<'tokens>>> {
        let data!(ParseResult { output, errors }) = self.into_data();
        if errors.is_empty() {
            Ok(output.expect("successful parser results contain output"))
        } else {
            Err(errors)
        }
    }
}

/// A parser from the syntax token stream to `O`.
#[contract_trait]
pub(super) trait Parser<'tokens, O> {
    #[requires(true)]
    #[ensures(true)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<O, ()>;

    #[requires(true)]
    #[ensures(true)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()>;

    #[requires(true)]
    #[ensures(true)]
    fn parse_with_state(
        &self,
        input: MappedInput<'tokens>,
        state: &mut ParserState<'tokens>,
    ) -> ParseResult<'tokens, O>
    where
        Self: Sized,
    {
        let mut errors = Errors::default();
        let result = {
            let mut input = InputRef {
                input,
                cursor: new!(CursorInner {
                    index: 0,
                    last_end: None,
                }),
                errors: &mut errors,
                state,
            };
            let output = self.drive_emit(&mut input);
            let output = match output {
                Ok(output) if end().drive_check(&mut input).is_ok() => Some(output),
                Ok(_) | Err(()) => None,
            };
            if output.is_none() && input.errors.alternative.is_none() {
                let cursor = input.cursor();
                let span = input.span_since(&cursor);
                input.add_expected(std::iter::empty::<RichPattern<'tokens>>(), None, span);
            }
            output
        };
        let parser_errors = match (result.is_none(), errors.alternative) {
            (true, Some(alternative)) => vec![alternative.error],
            (true, None) => unreachable!("failed parsers register a primary error"),
            (false, _) => Vec::new(),
        };
        new!(ParseResult {
            output: result,
            errors: parser_errors,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn map<U, F>(self, mapper: F) -> Map<Self, O, F>
    where
        Self: Sized,
        F: Fn(O) -> U,
    {
        Map {
            parser: self,
            mapper,
            output: PhantomData,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn map_with<U, F>(self, mapper: F) -> MapWith<Self, O, F>
    where
        Self: Sized,
        F: Fn(O, &mut MapExtra<'tokens, '_>) -> U,
    {
        MapWith {
            parser: self,
            mapper,
            output: PhantomData,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn map_err_with_state<F>(self, mapper: F) -> MapErrWithState<Self, F>
    where
        Self: Sized,
        F: Fn(
            SyntaxParseError<'tokens>,
            SimpleSpan,
            &mut ParserState<'tokens>,
        ) -> SyntaxParseError<'tokens>,
    {
        MapErrWithState {
            parser: self,
            mapper,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn then<U, P>(self, other: P) -> Then<Self, P, O, U>
    where
        Self: Sized,
        P: Parser<'tokens, U>,
    {
        Then {
            first: self,
            second: other,
            outputs: PhantomData,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn ignore_then<U, P>(self, other: P) -> IgnoreThen<Self, P, O>
    where
        Self: Sized,
        P: Parser<'tokens, U>,
    {
        IgnoreThen {
            first: self,
            second: other,
            ignored: PhantomData,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn or<P>(self, other: P) -> Or<Self, P>
    where
        Self: Sized,
        P: Parser<'tokens, O>,
    {
        Or {
            first: self,
            second: other,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn labelled<L>(self, label: L) -> Labelled<Self, L>
    where
        Self: Sized,
        L: Clone,
    {
        Labelled {
            parser: self,
            label,
            is_context: false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn boxed(self) -> Boxed<'tokens, O>
    where
        Self: Sized + 'tokens,
        O: 'tokens,
    {
        Boxed::new(self)
    }
}

#[contract_trait]
impl<'tokens, O, P> Parser<'tokens, O> for &P
where
    P: Parser<'tokens, O>,
{
    #[inline(always)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<O, ()> {
        P::drive_emit(*self, input)
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        P::drive_check(*self, input)
    }
}

/// A parser backed by shared type-erased storage.
#[invariant(true)]
pub(crate) struct Boxed<'tokens, O> {
    inner: Rc<dyn Parser<'tokens, O> + 'tokens>,
}

impl<O> Clone for Boxed<'_, O> {
    #[requires(true)]
    #[ensures(Rc::ptr_eq(&ret.inner, &self.inner))]
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl<'tokens, O> Boxed<'tokens, O> {
    #[requires(true)]
    #[ensures(true)]
    fn new<P>(parser: P) -> Self
    where
        P: Parser<'tokens, O> + 'tokens,
    {
        Self {
            inner: Rc::new(parser),
        }
    }
}

#[contract_trait]
impl<'tokens, O> Parser<'tokens, O> for Boxed<'tokens, O> {
    #[inline(always)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<O, ()> {
        self.inner.drive_emit(input)
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        self.inner.drive_check(input)
    }

    fn boxed(self) -> Boxed<'tokens, O>
    where
        Self: Sized + 'tokens,
        O: 'tokens,
    {
        self
    }
}

/// A custom imperative parser.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct Custom<F> {
    parser: F,
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn custom<'tokens, F, O>(parser: F) -> Custom<F>
where
    F: Fn(&mut InputRef<'tokens, '_>) -> Result<O, SyntaxParseError<'tokens>>,
{
    Custom { parser }
}

#[contract_trait]
impl<'tokens, O, F> Parser<'tokens, O> for Custom<F>
where
    F: Fn(&mut InputRef<'tokens, '_>) -> Result<O, SyntaxParseError<'tokens>>,
{
    #[inline]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<O, ()> {
        let before = input.cursor;
        match (self.parser)(input) {
            Ok(output) => Ok(output),
            Err(error) => {
                input.add_alternative_error(before, error);
                Err(())
            }
        }
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        self.drive_emit(input).map(|_| ())
    }
}

/// A parser that succeeds without consuming input.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct Empty;

#[requires(true)]
#[ensures(true)]
pub(crate) fn empty() -> Empty {
    Empty
}

#[contract_trait]
impl<'tokens> Parser<'tokens, ()> for Empty {
    #[inline(always)]
    fn drive_emit(&self, _input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        Ok(())
    }

    #[inline(always)]
    fn drive_check(&self, _input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        Ok(())
    }
}

/// A parser that accepts only the end of input.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct End;

#[requires(true)]
#[ensures(true)]
pub(crate) fn end() -> End {
    End
}

#[contract_trait]
impl<'tokens> Parser<'tokens, ()> for End {
    #[inline]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        let before = input.save();
        match input.next() {
            None => Ok(()),
            Some(token) => {
                let span = input.span_since(before.cursor());
                input.rewind(before);
                input.add_expected([RichPattern::EndOfInput], Some(MaybeRef::Val(token)), span);
                Err(())
            }
        }
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        self.drive_emit(input)
    }
}

/// Output mapping.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct Map<P, O, F> {
    parser: P,
    mapper: F,
    output: PhantomData<fn(O)>,
}

#[contract_trait]
impl<'tokens, O, U, P, F> Parser<'tokens, U> for Map<P, O, F>
where
    P: Parser<'tokens, O>,
    F: Fn(O) -> U,
{
    #[inline(always)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<U, ()> {
        self.parser.drive_emit(input).map(&self.mapper)
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        self.parser.drive_check(input)
    }
}

/// Output mapping with parser-state access.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct MapWith<P, O, F> {
    parser: P,
    mapper: F,
    output: PhantomData<fn(O)>,
}

#[contract_trait]
impl<'tokens, O, U, P, F> Parser<'tokens, U> for MapWith<P, O, F>
where
    P: Parser<'tokens, O>,
    F: Fn(O, &mut MapExtra<'tokens, '_>) -> U,
{
    #[inline(always)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<U, ()> {
        let output = self.parser.drive_emit(input)?;
        Ok((self.mapper)(output, &mut MapExtra { state: input.state }))
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        self.parser.drive_check(input)
    }
}

/// Primary-error mapping with parser-state access.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct MapErrWithState<P, F> {
    parser: P,
    mapper: F,
}

impl<P, F> MapErrWithState<P, F> {
    #[requires(true)]
    #[ensures(true)]
    #[inline(always)]
    fn drive<'tokens, O, R>(
        &self,
        input: &mut InputRef<'tokens, '_>,
        parser: impl FnOnce(&P, &mut InputRef<'tokens, '_>) -> Result<R, ()>,
    ) -> Result<R, ()>
    where
        P: Parser<'tokens, O>,
        F: Fn(
            SyntaxParseError<'tokens>,
            SimpleSpan,
            &mut ParserState<'tokens>,
        ) -> SyntaxParseError<'tokens>,
    {
        let start = input.cursor();
        let old_alternative = input.take_alternative();
        let result = parser(&self.parser, input);
        let new_alternative = input.take_alternative();
        input.errors.alternative = old_alternative;
        if result.is_ok() {
            if let Some(alternative) = new_alternative {
                input.add_alternative_error(alternative.position, alternative.error);
            }
        } else {
            let mut alternative = new_alternative.expect("failed parsers register a primary error");
            let span = input.span_since(&start);
            alternative.error = (self.mapper)(alternative.error, span, input.state());
            input.add_alternative_error(alternative.position, alternative.error);
        }
        result
    }
}

#[contract_trait]
impl<'tokens, O, P, F> Parser<'tokens, O> for MapErrWithState<P, F>
where
    P: Parser<'tokens, O>,
    F: Fn(
        SyntaxParseError<'tokens>,
        SimpleSpan,
        &mut ParserState<'tokens>,
    ) -> SyntaxParseError<'tokens>,
{
    #[inline(always)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<O, ()> {
        self.drive(input, Parser::drive_emit)
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        self.drive(input, Parser::drive_check)
    }
}

/// Sequential parser composition retaining both outputs.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct Then<A, B, OA, OB> {
    first: A,
    second: B,
    outputs: PhantomData<fn() -> (OA, OB)>,
}

#[contract_trait]
impl<'tokens, A, B, OA, OB> Parser<'tokens, (OA, OB)> for Then<A, B, OA, OB>
where
    A: Parser<'tokens, OA>,
    B: Parser<'tokens, OB>,
{
    #[inline(always)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<(OA, OB), ()> {
        let first = self.first.drive_emit(input)?;
        let second = self.second.drive_emit(input)?;
        Ok((first, second))
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        self.first.drive_check(input)?;
        self.second.drive_check(input)
    }
}

/// Sequential parser composition discarding the first output.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct IgnoreThen<A, B, OA> {
    first: A,
    second: B,
    ignored: PhantomData<fn(OA)>,
}

#[contract_trait]
impl<'tokens, A, B, OA, OB> Parser<'tokens, OB> for IgnoreThen<A, B, OA>
where
    A: Parser<'tokens, OA>,
    B: Parser<'tokens, OB>,
{
    #[inline(always)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<OB, ()> {
        self.first.drive_check(input)?;
        self.second.drive_emit(input)
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        self.first.drive_check(input)?;
        self.second.drive_check(input)
    }
}

/// Ordered parser choice with checkpoint rewind between alternatives.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct Or<A, B> {
    first: A,
    second: B,
}

#[contract_trait]
impl<'tokens, O, A, B> Parser<'tokens, O> for Or<A, B>
where
    A: Parser<'tokens, O>,
    B: Parser<'tokens, O>,
{
    #[inline]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<O, ()> {
        let before = input.save();
        match self.first.drive_emit(input) {
            Ok(output) => Ok(output),
            Err(()) => {
                input.rewind(before);
                self.second.drive_emit(input)
            }
        }
    }

    #[inline]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        let before = input.save();
        match self.first.drive_check(input) {
            Ok(()) => Ok(()),
            Err(()) => {
                input.rewind(before);
                self.second.drive_check(input)
            }
        }
    }
}

/// Error labelling at the beginning of a parser.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct Labelled<P, L> {
    parser: P,
    label: L,
    is_context: bool,
}

impl<P, L> Labelled<P, L> {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn as_terminal(self) -> Self {
        self
    }

    #[requires(true)]
    #[ensures(true)]
    #[inline]
    fn drive<'tokens, R>(
        &self,
        input: &mut InputRef<'tokens, '_>,
        parser: impl FnOnce(&P, &mut InputRef<'tokens, '_>) -> Result<R, ()>,
    ) -> Result<R, ()>
    where
        L: Clone,
        SyntaxParseError<'tokens>: LabelError<'tokens, L>,
    {
        let old_alternative = input.take_alternative();
        let before = input.save();
        let result = parser(&self.parser, input);
        let new_alternative = input.take_alternative();
        input.errors.alternative = old_alternative;
        if let Some(mut alternative) = new_alternative {
            match alternative.position.index.cmp(&before.cursor.inner.index) {
                Ordering::Equal => alternative.error.label_with(self.label.clone()),
                Ordering::Greater if self.is_context => {
                    let span = input
                        .input
                        .span_between(before.cursor.inner, alternative.position);
                    alternative.error.in_context(self.label.clone(), span);
                }
                Ordering::Greater | Ordering::Less => {}
            }
            input.add_alternative_error(alternative.position, alternative.error);
        }
        result
    }
}

#[contract_trait]
impl<'tokens, O, P, L> Parser<'tokens, O> for Labelled<P, L>
where
    P: Parser<'tokens, O>,
    L: Clone,
    SyntaxParseError<'tokens>: LabelError<'tokens, L>,
{
    #[inline(always)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<O, ()> {
        self.drive(input, Parser::drive_emit)
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        self.drive(input, Parser::drive_check)
    }
}

#[contract_trait]
trait ErasedRecursiveNode {}

#[contract_trait]
impl<T> ErasedRecursiveNode for T {}

/// Strong owner for every node in one mutually-recursive parser family.
#[invariant(true)]
struct RecursiveFamilyStorage<'tokens> {
    nodes: RefCell<Vec<Rc<dyn ErasedRecursiveNode + 'tokens>>>,
}

/// Builder and lifetime owner for one generated recursive parser family.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct RecursiveFamily<'tokens> {
    storage: Rc<RecursiveFamilyStorage<'tokens>>,
}

impl<'tokens> RecursiveFamily<'tokens> {
    #[requires(true)]
    #[ensures(ret.storage.nodes.borrow().is_empty())]
    pub(crate) fn new() -> Self {
        Self {
            storage: Rc::new(RecursiveFamilyStorage {
                nodes: RefCell::new(Vec::new()),
            }),
        }
    }

    #[requires(true)]
    #[ensures(self.storage.nodes.borrow().len() == old(self.storage.nodes.borrow().len()) + 1)]
    pub(crate) fn declare<O: 'tokens>(&self) -> Recursive<'tokens, O> {
        let node = Rc::new(RecursiveNode {
            parser: OnceCell::new(),
        });
        self.storage.nodes.borrow_mut().push(node.clone());
        Recursive {
            node: Rc::downgrade(&node),
        }
    }

    #[requires(true)]
    #[ensures(Rc::ptr_eq(&ret.owner, &self.storage))]
    pub(crate) fn own<P>(&self, parser: P) -> OwnedRecursiveRoot<'tokens, P> {
        OwnedRecursiveRoot {
            parser,
            owner: Rc::clone(&self.storage),
        }
    }
}

/// Type-specific recursive rule node stored in a heterogeneous family owner.
#[invariant(true)]
struct RecursiveNode<'tokens, O> {
    parser: OnceCell<Box<dyn Parser<'tokens, O> + 'tokens>>,
}

/// A weak, non-owning recursive backedge.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct Recursive<'tokens, O> {
    node: Weak<RecursiveNode<'tokens, O>>,
}

impl<'tokens, O: 'tokens> Recursive<'tokens, O> {
    #[requires(true)]
    #[ensures(self.node.upgrade().is_some_and(|node| node.parser.get().is_some()))]
    #[track_caller]
    pub(crate) fn define<P>(&mut self, parser: P)
    where
        P: Parser<'tokens, O> + 'tokens,
    {
        self.node
            .upgrade()
            .expect("recursive parser family dropped before definition")
            .parser
            .set(Box::new(parser))
            .unwrap_or_else(|_| panic!("recursive parsers can only be defined once"));
    }
}

#[contract_trait]
impl<'tokens, O> Parser<'tokens, O> for Recursive<'tokens, O> {
    #[inline(always)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<O, ()> {
        stacker::maybe_grow(64 * 1024, 1024 * 1024, || {
            self.node
                .upgrade()
                .expect("recursive parser family owner is not retained")
                .parser
                .get()
                .expect("recursive parser used before definition")
                .drive_emit(input)
        })
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        stacker::maybe_grow(64 * 1024, 1024 * 1024, || {
            self.node
                .upgrade()
                .expect("recursive parser family owner is not retained")
                .parser
                .get()
                .expect("recursive parser used before definition")
                .drive_check(input)
        })
    }
}

/// An exported root parser that keeps its entire recursive family alive.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct OwnedRecursiveRoot<'tokens, P> {
    parser: P,
    owner: Rc<RecursiveFamilyStorage<'tokens>>,
}

#[contract_trait]
impl<'tokens, O, P> Parser<'tokens, O> for OwnedRecursiveRoot<'tokens, P>
where
    P: Parser<'tokens, O>,
{
    #[inline(always)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<O, ()> {
        let _owner = &self.owner;
        self.parser.drive_emit(input)
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        let _owner = &self.owner;
        self.parser.drive_check(input)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use bityzba::{invariant, requires};

    use super::{Parser, Recursive, RecursiveFamily, SharedSyntaxOutput};

    #[invariant(true)]
    struct DropProbe;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_syntax_output_clones_share_until_owned_consumption() {
        let stored = SharedSyntaxOutput::new(vec!["memo payload".to_owned()]);
        let replayed = stored.clone();

        assert!(Rc::ptr_eq(&stored.value, &replayed.value));
        assert_eq!(replayed.into_owned(), ["memo payload"]);
        assert_eq!(stored.into_owned(), ["memo payload"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recursive_family_releases_definitions_with_last_root() {
        let retained_by_definition = Rc::new(DropProbe);
        let weak_probe = Rc::downgrade(&retained_by_definition);
        let root = {
            let family = RecursiveFamily::new();
            let mut first: Recursive<'_, ()> = family.declare();
            let mut second: Recursive<'_, ()> = family.declare();
            first.define(second.clone().map({
                let retained_by_definition = Rc::clone(&retained_by_definition);
                move |()| {
                    let _retained = &retained_by_definition;
                }
            }));
            second.define(first.clone());
            family.own(first).boxed()
        };
        drop(retained_by_definition);
        assert!(weak_probe.upgrade().is_some());
        drop(root);
        assert!(weak_probe.upgrade().is_none());
    }
}
