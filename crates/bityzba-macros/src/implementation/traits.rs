/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{FnArg, ImplItem, ItemImpl, ItemTrait, Pat, TraitItem, TraitItemFn};

use crate::implementation::ContractType;

/// Name used for the "re-routed" method.
fn contract_method_impl_name(name: &str) -> String {
    format!("__contracts_impl_{}", name)
}

/// Modifies a trait item in a way that it includes contracts.
pub(crate) fn contract_trait_item_trait(_attrs: TokenStream, mut trait_: ItemTrait) -> TokenStream {
    /// Just rename the method to have an internal, generated name.
    fn create_method_rename(method: &TraitItemFn) -> TraitItemFn {
        let mut m = method.clone();

        // rename method and modify attributes
        {
            // remove any contracts attributes and rename
            let name = m.sig.ident.to_string();

            let new_name = contract_method_impl_name(&name);

            let mut new_attrs = vec![];
            new_attrs.push(syn::parse_quote!(#[doc(hidden)]));
            new_attrs.push(syn::parse_quote!(#[doc = "This is an internal function that is not meant to be used directly!"]));
            new_attrs.push(syn::parse_quote!(#[doc = "See the documentation of the `#[contract_trait]` attribute."]));

            // add all existing non-contract attributes
            new_attrs.extend(
                m.attrs
                    .iter()
                    .filter(|attr| {
                        let name = attr.path().segments.last().unwrap().ident.to_string();

                        ContractType::contract_type_and_mode(&name).is_none()
                    })
                    .cloned(),
            );

            m.attrs = new_attrs;

            m.sig.ident = syn::Ident::new(&new_name, m.sig.ident.span());
        }

        m
    }

    /// Create a wrapper function which has a default implementation and
    /// includes contracts.
    ///
    /// This new function forwards the call to the actual implementation.
    fn create_method_wrapper(method: &TraitItemFn) -> syn::Result<TraitItemFn> {
        struct ArgInfo {
            call_toks: proc_macro2::TokenStream,
        }

        // Calculate name and pattern tokens
        fn arg_pat_info(pat: &mut Pat, next_synthetic: &mut usize) -> syn::Result<ArgInfo> {
            match pat {
                Pat::Ident(ident) => {
                    if ident.subpat.is_some() {
                        return Err(syn::Error::new_spanned(
                            pat,
                            "#[contract_trait] does not support `@` parameter patterns",
                        ));
                    }
                    let ident = ident.ident.clone();
                    *pat = syn::parse_quote!(#ident);
                    let toks = quote::quote! {
                        #ident
                    };
                    Ok(ArgInfo { call_toks: toks })
                }
                Pat::Wild(wild) => {
                    let index = *next_synthetic;
                    *next_synthetic += 1;
                    let ident = syn::Ident::new(
                        &format!("__bityzba_contract_arg_{index}"),
                        wild.underscore_token.span,
                    );
                    *pat = syn::parse_quote!(#ident);
                    Ok(ArgInfo {
                        call_toks: quote::quote!(#ident),
                    })
                }
                Pat::Tuple(tup) => {
                    let infos = tup
                        .elems
                        .iter_mut()
                        .map(|elem| arg_pat_info(elem, next_synthetic))
                        .collect::<syn::Result<Vec<_>>>()?;

                    let toks = {
                        let mut toks = proc_macro2::TokenStream::new();

                        for info in infos {
                            toks.extend(info.call_toks);
                            toks.extend(quote::quote!(,));
                        }

                        toks
                    };

                    Ok(ArgInfo {
                        call_toks: quote::quote!((#toks)),
                    })
                }
                _ => Err(syn::Error::new_spanned(
                    pat,
                    "#[contract_trait] supports identifier, wildcard, and tuple parameter patterns",
                )),
            }
        }

        let mut m = method.clone();
        let mut next_synthetic = 0usize;

        let argument_data = m
            .sig
            .inputs
            .iter_mut()
            .map(|t: &mut FnArg| match t {
                FnArg::Receiver(_) => Ok(quote::quote!(self)),
                FnArg::Typed(p) => {
                    arg_pat_info(&mut p.pat, &mut next_synthetic).map(|info| info.call_toks)
                }
            })
            .collect::<syn::Result<Vec<_>>>()?;

        let arguments = {
            let mut toks = proc_macro2::TokenStream::new();

            for arg in argument_data {
                toks.extend(arg);
                toks.extend(quote::quote!(,));
            }

            toks
        };

        let body: TokenStream = {
            let name = contract_method_impl_name(&m.sig.ident.to_string());
            let name = syn::Ident::new(&name, m.sig.ident.span());

            quote::quote! {
                {
                    Self::#name(#arguments)
                }
            }
        };

        let mut attrs = vec![];

        // keep the documentation and contracts of the original method
        attrs.extend(
            m.attrs
                .iter()
                .filter(|a| {
                    let name = a.path().segments.last().unwrap().ident.to_string();
                    // is doc?
                    if name == "doc" {
                        return true;
                    }

                    // is contract?
                    ContractType::contract_type_and_mode(&name).is_some()
                })
                .cloned(),
        );
        // always inline
        attrs.push(syn::parse_quote!(#[inline(always)]));

        m.attrs = attrs;

        {
            let block: syn::Block = syn::parse2(body).unwrap();
            m.default = Some(block);
            m.semi_token = None;
        }

        Ok(m)
    }

    // create method wrappers and renamed items
    let funcs = match trait_
        .items
        .iter()
        .filter_map(|item| {
            if let TraitItem::Fn(m) = item {
                Some(create_method_wrapper(m).map(|wrapper| {
                    vec![
                        TraitItem::Fn(create_method_rename(m)),
                        TraitItem::Fn(wrapper),
                    ]
                }))
            } else {
                None
            }
        })
        .collect::<syn::Result<Vec<_>>>()
    {
        Ok(funcs) => funcs.into_iter().flatten().collect::<Vec<_>>(),
        Err(error) => return error.to_compile_error(),
    };

    // remove all previous methods
    trait_
        .items
        .retain(|item| !matches!(item, TraitItem::Fn(_)));

    // add back new methods
    trait_.items.extend(funcs);

    trait_.into_token_stream()
}

/// Rename all methods inside an `impl` to use the "internal implementation"
/// name.
pub(crate) fn contract_trait_item_impl(_attrs: TokenStream, impl_: ItemImpl) -> TokenStream {
    let new_impl = {
        let mut impl_: ItemImpl = impl_;

        impl_.items.iter_mut().for_each(|it| {
            if let ImplItem::Fn(method) = it {
                let new_name = contract_method_impl_name(&method.sig.ident.to_string());
                let new_ident = syn::Ident::new(&new_name, method.sig.ident.span());

                method.sig.ident = new_ident;
            }
        });

        impl_
    };

    new_impl.to_token_stream()
}

#[cfg(test)]
mod tests {

    #[test]
    fn attributes_stay_on_trait_def() {
        // attributes on functions should apply to the outer "wrapping" function
        // only, the "internal" function should be hidden and be inlined.

        let code = syn::parse_quote! {
            trait Random {
                /// Test!
                #[aaa]
                #[ensures((min..max).contains(ret))]
                fn random_number(min: u8, max: u8) -> u8;
            }
        };

        let expected = quote::quote! {
            trait Random {
                #[doc(hidden)]
                #[doc = "This is an internal function that is not meant to be used directly!"]
                #[doc = "See the documentation of the `#[contract_trait]` attribute."]
                /// Test!
                #[aaa]
                fn __contracts_impl_random_number(min: u8, max: u8) -> u8;

                /// Test!
                #[ensures((min..max).contains(ret))]
                #[inline(always)]
                fn random_number(min: u8, max: u8) -> u8 {
                    Self::__contracts_impl_random_number(min, max,)
                }
            }
        };

        let generated = super::contract_trait_item_trait(Default::default(), code);

        assert_eq!(generated.to_string(), expected.to_string());
    }

    #[test]
    fn attributes_stay_on_trait_impl() {
        // attributes on functions should apply to the outer "wrapping" function
        // only, the "internal" function should be hidden and be inlined.

        let code = syn::parse_quote! {
            impl Random for AlwaysMin {
                /// docs for this function!
                #[no_panic]
                fn random_number(min: u8, max: u8) -> u8 {
                    min
                }
            }
        };

        let expected = quote::quote! {
            impl Random for AlwaysMin {
                /// docs for this function!
                #[no_panic]
                fn __contracts_impl_random_number(min: u8, max: u8) -> u8 {
                    min
                }
            }
        };

        let generated = super::contract_trait_item_impl(Default::default(), code);

        assert_eq!(generated.to_string(), expected.to_string());
    }
}
