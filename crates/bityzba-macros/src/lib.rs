/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![doc = include_str!("../README.md")]

extern crate proc_macro;

mod implementation;

use implementation::ContractMode;
use proc_macro::TokenStream;

/// Pre-conditions are checked before the function body is run.
///
/// ## Example
///
/// ```rust
/// # use bityzba::*;
/// #[requires(elems.len() >= 1)]
/// fn max<T: Ord + Copy>(elems: &[T]) -> T {
///    // ...
/// # unimplemented!()
/// }
/// ```
#[proc_macro_attribute]
pub fn requires(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::requires(ContractMode::Always, attr, toks).into()
}

/// Same as [`requires`], but uses `debug_assert!`.
///
/// [`requires`]: attr.requires.html
#[proc_macro_attribute]
pub fn debug_requires(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::requires(ContractMode::Debug, attr, toks).into()
}

/// Same as [`requires`], but is only enabled in `#[cfg(test)]` environments.
///
/// [`requires`]: attr.requires.html
#[proc_macro_attribute]
pub fn test_requires(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::requires(ContractMode::Test, attr, toks).into()
}

/// Same as [`requires`], but only emits a contract check when the consuming
/// crate enables its `expensive_contracts` feature.
///
/// This compatibility entry point is replaced by first-class expensive contract
/// handling inside `#[contract_trait]` in the bityzba fork; outside traits it
/// expands to a feature-gated regular pre-condition.
#[proc_macro_attribute]
pub fn expensive_requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = item.into();
    implementation::requires(ContractMode::Expensive, attr, toks).into()
}

/// Post-conditions are checked after the function body is run.
///
/// The result of the function call is accessible in conditions using the `ret`
/// identifier.
///
/// A "pseudo-function" named `old` can be used to evaluate expressions in a
/// context *prior* to function execution.
/// This function takes only a single argument and the result of it will be
/// stored in a variable before the function is called. Because of this,
/// handling references might require special care.
///
/// ## Examples
///
/// ```rust
/// # use bityzba::*;
/// #[ensures(ret > x)]
/// fn incr(x: usize) -> usize {
///     x + 1
/// }
/// ```
///
/// ```rust
/// # use bityzba::*;
/// #[ensures(*x == old(*x) + 1, "x is incremented")]
/// fn incr(x: &mut usize) {
///     *x += 1;
/// }
/// ```
#[proc_macro_attribute]
pub fn ensures(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::ensures(ContractMode::Always, attr, toks).into()
}

/// Same as [`ensures`], but uses `debug_assert!`.
///
/// [`ensures`]: attr.ensures.html
#[proc_macro_attribute]
pub fn debug_ensures(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::ensures(ContractMode::Debug, attr, toks).into()
}

/// Same as [`ensures`], but is only enabled in `#[cfg(test)]` environments.
///
/// [`ensures`]: attr.ensures.html
#[proc_macro_attribute]
pub fn test_ensures(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::ensures(ContractMode::Test, attr, toks).into()
}

/// Same as [`ensures`], but only emits a contract check when the consuming
/// crate enables its `expensive_contracts` feature.
#[proc_macro_attribute]
pub fn expensive_ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = item.into();
    implementation::ensures(ContractMode::Expensive, attr, toks).into()
}

/// Invariants are conditions that have to be maintained at the "interface
/// boundaries".
///
/// Invariants can be supplied to functions (and "methods"), as well as on
/// `impl` blocks.
///
/// When applied to an `impl`-block all methods taking `self` (either by value
/// or reference) will be checked for the invariant.
///
/// ## Example
///
/// On a function:
///
/// ```rust
/// # use bityzba::*;
/// /// Update `num` to the next bigger even number.
/// #[invariant(*num % 2 == 0)]
/// fn advance_even(num: &mut usize) {
///     *num += 2;
/// }
/// ```
///
/// On an `impl`-block:
///
/// ```rust
/// # use bityzba::*;
/// struct EvenAdder {
///     count: usize,
/// }
///
/// #[invariant(self.count % 2 == 0)]
/// impl EvenAdder {
///     pub fn tell(&self) -> usize {
///         self.count
///     }
///
///     pub fn advance(&mut self) {
///         self.count += 2;
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn invariant(attr: TokenStream, toks: TokenStream) -> TokenStream {
    // Invariant attributes might apply to `impl` blocks as well, where the same
    // level is simply replicated on all methods.
    // Function expansions will resolve the actual mode themselves, so the
    // actual unreduced mode is passed here
    //
    // TODO: update comment when implemented for traits
    let attr = attr.into();
    let toks = toks.into();
    let mode = ContractMode::Always;
    implementation::invariant(mode, attr, toks).into()
}

/// Same as [`invariant`], but uses `debug_assert!`.
///
/// [`invariant`]: attr.invariant.html
#[proc_macro_attribute]
pub fn debug_invariant(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = ContractMode::Debug;
    let attr = attr.into();
    let toks = toks.into();
    implementation::invariant(mode, attr, toks).into()
}

/// Same as [`invariant`], but is only enabled in `#[cfg(test)]` environments.
///
/// [`invariant`]: attr.invariant.html
#[proc_macro_attribute]
pub fn test_invariant(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = ContractMode::Test;
    let attr = attr.into();
    let toks = toks.into();
    implementation::invariant(mode, attr, toks).into()
}

/// Same as [`invariant`], but only emits a contract check when the consuming
/// crate enables its `expensive_contracts` feature.
#[proc_macro_attribute]
pub fn expensive_invariant(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = item.into();
    implementation::invariant(ContractMode::Expensive, attr, toks).into()
}

/// A "contract_trait" is a trait which ensures all implementors respect all
/// provided contracts.
///
/// When this attribute is applied to a `trait` definition, the trait gets
/// modified so that all invocations of methods are checked.
///
/// When this attribute is applied to an `impl Trait for Type` item, the
/// implementation gets modified so it matches the trait definition.
///
/// **When the `#[contract_trait]` is not applied to either the trait or an
/// `impl` it will cause compile errors**.
///
/// ## Example
///
/// ```rust
/// # use bityzba::*;
/// #[contract_trait]
/// trait MyRandom {
///     #[requires(min < max)]
///     #[ensures(min <= ret, ret <= max)]
///     fn gen(min: f64, max: f64) -> f64;
/// }
///
/// // Not a very useful random number generator, but a valid one!
/// struct AlwaysMax;
///
/// #[contract_trait]
/// impl MyRandom for AlwaysMax {
///     fn gen(min: f64, max: f64) -> f64 {
///         max
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn contract_trait(attrs: TokenStream, toks: TokenStream) -> TokenStream {
    let attrs: proc_macro2::TokenStream = attrs.into();
    let toks: proc_macro2::TokenStream = toks.into();

    let item: syn::Item = syn::parse_quote!(#toks);

    let tts = match item {
        syn::Item::Trait(trait_) => implementation::contract_trait_item_trait(attrs, trait_),
        syn::Item::Impl(impl_) => {
            assert!(
                impl_.trait_.is_some(),
                "#[contract_trait] can only be applied to `trait` and `impl ... for` items"
            );
            implementation::contract_trait_item_impl(attrs, impl_)
        }
        _ => panic!("#[contract_trait] can only be applied to `trait` and `impl ... for` items"),
    };

    tts.into()
}

/// Data helper for invariant-bearing types generated by type-level
/// [`invariant`].
///
/// In expression position, `data! { name: value }` expands to a builder
/// closure accepted by generated `value.with_data` methods.
/// `data!(Type::Variant { name: value })` expands to the generated data enum
/// variant expression.
///
/// In pattern position, `data!(Type { name, .. })` expands to the generated
/// data type pattern, so code can destructure through `value.as_data()` without
/// naming `TypeData` directly.
#[proc_macro]
pub fn data(input: TokenStream) -> TokenStream {
    implementation::data(input.into()).into()
}

/// Construct an invariant-bearing value from struct or enum data, panicking if
/// the type invariant is violated.
#[proc_macro]
pub fn new(input: TokenStream) -> TokenStream {
    implementation::new_value(input.into()).into()
}

/// Construct an invariant-bearing value from struct or enum data, returning the
/// generated invariant error if validation fails.
#[proc_macro]
pub fn try_new(input: TokenStream) -> TokenStream {
    implementation::try_new_value(input.into()).into()
}
