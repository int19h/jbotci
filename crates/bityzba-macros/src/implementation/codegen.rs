/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{
    Attribute, Expr, ExprCall, ExprClosure, ReturnType, TypeImplTrait,
    spanned::Spanned,
    visit::{Visit, visit_return_type},
    visit_mut::{self as visitor, VisitMut, visit_block_mut, visit_expr_mut},
};

use crate::implementation::{Contract, ContractMode, ContractType, FuncWithContracts};

/// Substitution for `old()` expressions.
#[derive(Debug, Clone)]
pub(crate) struct OldExpr {
    /// Name of the variable binder.
    pub(crate) name: String,
    /// Expression to be evaluated.
    pub(crate) expr: Expr,
}

/// Extract calls to the pseudo-function `old()` in post-conditions,
/// which evaluates an expression in a context *before* the
/// to-be-checked-function is executed.
pub(crate) fn extract_old_calls(contracts: &mut [Contract]) {
    struct OldExtractor {
        last_id: usize,
        olds: Vec<OldExpr>,
    }

    // if the call is a call to old() then the argument will be
    // returned.
    fn get_old_data(call: &ExprCall) -> Option<Expr> {
        // must have only one argument
        if call.args.len() != 1 {
            return None;
        }

        if let Expr::Path(path) = &*call.func {
            if path.path.is_ident("old") {
                Some(call.args[0].clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    impl visitor::VisitMut for OldExtractor {
        fn visit_expr_mut(&mut self, expr: &mut Expr) {
            if let Expr::Call(call) = expr {
                if let Some(mut old_arg) = get_old_data(call) {
                    // if it's a call to old() then add to list of
                    // old expressions and continue to check the
                    // argument.

                    self.visit_expr_mut(&mut old_arg);

                    let id = self.last_id;
                    self.last_id += 1;

                    let old_var_name = format!("__contract_old_{}", id);

                    let old_expr = OldExpr {
                        name: old_var_name.clone(),
                        expr: old_arg,
                    };

                    self.olds.push(old_expr);

                    // override the original expression with the new variable
                    // identifier
                    *expr = {
                        let span = expr.span();

                        let ident = syn::Ident::new(&old_var_name, span);

                        let toks = quote::quote_spanned! { span=> #ident };

                        syn::parse(toks.into()).unwrap()
                    };
                } else {
                    // otherwise continue visiting the expression call
                    visitor::visit_expr_call_mut(self, call);
                }
            } else {
                visitor::visit_expr_mut(self, expr);
            }
        }
    }

    let mut last_id = 0;

    for contract in contracts {
        if contract.ty != ContractType::Ensures {
            continue;
        }

        for (assertion, old_exprs) in contract
            .assertions
            .iter_mut()
            .zip(contract.old_assertions.iter_mut())
        {
            let mut extractor = OldExtractor {
                last_id,
                olds: vec![],
            };
            extractor.visit_expr_mut(assertion);
            last_id = extractor.last_id;
            *old_exprs = extractor.olds;
        }
    }
}

fn get_assert_macro(
    ctype: ContractType, // only Pre/Post allowed.
    mode: ContractMode,
    span: Span,
) -> Option<Ident> {
    if cfg!(feature = "mirai_assertions") {
        match (ctype, mode) {
            (ContractType::Requires, ContractMode::Always) => {
                Some(Ident::new("checked_precondition", span))
            }
            (ContractType::Requires, ContractMode::Debug) => {
                Some(Ident::new("debug_checked_precondition", span))
            }
            (ContractType::Requires, ContractMode::Expensive) => {
                Some(Ident::new("checked_precondition", span))
            }
            (ContractType::Requires, ContractMode::Test) => {
                Some(Ident::new("debug_checked_precondition", span))
            }
            (ContractType::Requires, ContractMode::Disabled) => {
                Some(Ident::new("precondition", span))
            }
            (ContractType::Requires, ContractMode::LogOnly) => {
                Some(Ident::new("precondition", span))
            }
            (ContractType::Ensures, ContractMode::Always) => {
                Some(Ident::new("checked_postcondition", span))
            }
            (ContractType::Ensures, ContractMode::Debug) => {
                Some(Ident::new("debug_checked_postcondition", span))
            }
            (ContractType::Ensures, ContractMode::Expensive) => {
                Some(Ident::new("checked_postcondition", span))
            }
            (ContractType::Ensures, ContractMode::Test) => {
                Some(Ident::new("debug_checked_postcondition", span))
            }
            (ContractType::Ensures, ContractMode::Disabled) => {
                Some(Ident::new("postcondition", span))
            }
            (ContractType::Ensures, ContractMode::LogOnly) => {
                Some(Ident::new("postcondition", span))
            }
            (ContractType::Invariant, _) => {
                panic!("expected Invariant to be narrowed down to Pre/Post")
            }
        }
    } else {
        match mode {
            ContractMode::Always => Some(Ident::new("assert", span)),
            ContractMode::Debug => Some(Ident::new("debug_assert", span)),
            ContractMode::Expensive => Some(Ident::new("assert", span)),
            ContractMode::Test => Some(Ident::new("debug_assert", span)),
            ContractMode::Disabled => None,
            ContractMode::LogOnly => None,
        }
    }
}

fn old_ident(old: &OldExpr) -> Ident {
    Ident::new(&old.name, old.expr.span())
}

fn old_marker_ident(old: &OldExpr) -> Ident {
    Ident::new(&format!("{}_marker", old.name), old.expr.span())
}

fn expensive_old_binding(old: &OldExpr, previous_expensive_olds: &[OldExpr]) -> TokenStream {
    let span = old.expr.span();
    let old_name = old_ident(old);
    let marker_name = old_marker_ident(old);
    let expr = &old.expr;

    let previous_old_bindings = previous_expensive_olds.iter().map(|previous| {
        let previous_name = old_ident(previous);
        let previous_marker_name = old_marker_ident(previous);
        let previous_span = previous.expr.span();

        quote::quote_spanned! { previous_span=>
            let #previous_name =
                __bityzba_expensive_contract_old_value(#previous_marker_name);
        }
    });

    quote::quote_spanned! { span=>
        #[cfg(feature = "expensive_contracts")]
        let #old_name = #expr;

        #[cfg(not(feature = "expensive_contracts"))]
        let #marker_name = {
            fn __bityzba_expensive_contract_old_marker<
                T,
                F: ::core::ops::FnOnce() -> T,
            >(
                _: F,
            ) -> ::core::marker::PhantomData<T> {
                ::core::marker::PhantomData
            }

            fn __bityzba_expensive_contract_old_value<T>(
                _: ::core::marker::PhantomData<T>,
            ) -> T {
                loop {}
            }

            let marker;
            if false {
                #(#previous_old_bindings)*
                marker = __bityzba_expensive_contract_old_marker(|| #expr);
            } else {
                marker = ::core::marker::PhantomData;
            }
            marker
        };
    }
}

fn expensive_typecheck(exec_expr: &Expr, old_exprs: &[OldExpr]) -> TokenStream {
    let span = exec_expr.span();

    let old_value_fn = if old_exprs.is_empty() {
        TokenStream::new()
    } else {
        quote::quote_spanned! { span=>
            fn __bityzba_expensive_contract_old_value<T>(
                _: ::core::marker::PhantomData<T>,
            ) -> T {
                loop {}
            }
        }
    };

    let old_bindings = old_exprs.iter().map(|old| {
        let name = old_ident(old);
        let marker_name = old_marker_ident(old);
        let span = old.expr.span();

        quote::quote_spanned! { span=>
            let #name = __bityzba_expensive_contract_old_value(#marker_name);
        }
    });

    quote::quote_spanned! { span=>
        #[allow(
            clippy::assertions_on_constants,
            clippy::nonminimal_bool,
            unreachable_code,
            unused_variables
        )]
        {
            #old_value_fn
            if false {
                #(#old_bindings)*
                let _: bool = #exec_expr;
            }
        }
    }
}

/// Generate the resulting code for this function by inserting assertions.
pub(crate) fn generate(mut func: FuncWithContracts, docs: Vec<Attribute>) -> TokenStream {
    let func_name = func.function.sig.ident.to_string();

    // creates an assertion appropriate for the current mode
    let make_assertion = |mode: ContractMode,
                          ctype: ContractType,
                          display: TokenStream,
                          exec_expr: &Expr,
                          old_exprs: &[OldExpr],
                          desc: &str| {
        let span = display.span();
        let mut result = TokenStream::new();

        let format_args = quote::quote_spanned! { span=>
            concat!(concat!(#desc, ": "), stringify!(#display))
        };

        if mode == ContractMode::LogOnly {
            result.extend(quote::quote_spanned! { span=>
                #[allow(clippy::assertions_on_constants, clippy::nonminimal_bool)]
                {
                    if !(#exec_expr) {
                        log::error!("{}", #format_args);
                    }
                }
            });
        }

        if let Some(assert_macro) = get_assert_macro(ctype, mode, span) {
            result.extend(quote::quote_spanned! { span=>
                #[allow(clippy::assertions_on_constants, clippy::nonminimal_bool)] {
                    #assert_macro!(#exec_expr, "{}", #format_args);
                }
            });
        }

        if mode == ContractMode::Test {
            quote::quote_spanned! { span=>
              #[cfg(test)] {
                #result
              }
            }
        } else if mode == ContractMode::Expensive {
            let typecheck = expensive_typecheck(exec_expr, old_exprs);

            quote::quote_spanned! { span=>
              #[cfg(feature = "expensive_contracts")] {
                #result
              }

              #[cfg(not(feature = "expensive_contracts"))] {
                #typecheck
              }
            }
        } else {
            result
        }
    };

    //
    // generate assertion code for pre-conditions
    //

    let pre = func
        .contracts
        .iter()
        .filter(|c| c.ty == ContractType::Requires || c.ty == ContractType::Invariant)
        .flat_map(|c| {
            let contract_type_name = if c.ty == ContractType::Invariant {
                format!("{} (as pre-condition)", c.ty.message_name())
            } else {
                c.ty.message_name().to_string()
            };

            let desc = if let Some(desc) = c.desc.as_ref() {
                format!("{} of {} violated: {}", contract_type_name, func_name, desc)
            } else {
                format!("{} of {} violated", contract_type_name, func_name)
            };

            c.assertions
                .iter()
                .zip(c.streams.iter())
                .map(move |(expr, display)| {
                    let mode = c.mode.final_mode();

                    make_assertion(
                        mode,
                        ContractType::Requires,
                        display.clone(),
                        expr,
                        &[],
                        &desc.clone(),
                    )
                })
        })
        .collect::<TokenStream>();

    //
    // generate assertion code for post-conditions
    //

    let post = func
        .contracts
        .iter()
        .filter(|c| c.ty == ContractType::Ensures || c.ty == ContractType::Invariant)
        .flat_map(|c| {
            let contract_type_name = if c.ty == ContractType::Invariant {
                format!("{} (as post-condition)", c.ty.message_name())
            } else {
                c.ty.message_name().to_string()
            };

            let desc = if let Some(desc) = c.desc.as_ref() {
                format!("{} of {} violated: {}", contract_type_name, func_name, desc)
            } else {
                format!("{} of {} violated", contract_type_name, func_name)
            };

            c.assertions
                .iter()
                .zip(c.streams.iter())
                .zip(c.old_assertions.iter())
                .map(move |((expr, display), old_exprs)| {
                    let mode = c.mode.final_mode();

                    make_assertion(
                        mode,
                        ContractType::Ensures,
                        display.clone(),
                        expr,
                        old_exprs,
                        &desc.clone(),
                    )
                })
        })
        .collect::<TokenStream>();

    //
    // bind "old()" expressions
    //

    let olds = {
        let mut toks = TokenStream::new();
        let mut previous_expensive_olds = Vec::new();

        for contract in &func.contracts {
            let mode = contract.mode.final_mode();

            for old_exprs in &contract.old_assertions {
                for old in old_exprs {
                    let span = old.expr.span();

                    if mode == ContractMode::Expensive {
                        toks.extend(expensive_old_binding(old, &previous_expensive_olds));
                        previous_expensive_olds.push(old.clone());
                    } else {
                        let name = syn::Ident::new(&old.name, span);
                        let expr = &old.expr;

                        toks.extend(quote::quote_spanned! { span=>
                            let #name = #expr;
                        });
                    }
                }
            }
        }

        toks
    };

    //
    // wrap the function body in a block so that we can use its return value
    //

    let body = 'blk: {
        let mut block = func.function.block.clone();
        visit_block_mut(&mut ReturnReplacer, &mut block);

        let mut impl_detector = ImplDetector { found_impl: false };
        visit_return_type(&mut impl_detector, &func.function.sig.output);

        if !impl_detector.found_impl
            && let ReturnType::Type(.., ref return_type) = func.function.sig.output
        {
            break 'blk quote::quote! {
                let ret: #return_type = 'run: #block;
            };
        }

        quote::quote! {
            let ret = 'run: #block;
        }
    };

    //
    // create a new function body containing all assertions
    //

    let new_block = quote::quote! {

        {
            #pre

            #olds

            #body

            #post

            ret
        }

    };

    // insert documentation attributes

    func.function.attrs.extend(docs);

    // replace the old function body with the new one

    *func.function.block = syn::parse_quote!(#new_block);

    func.function.into_token_stream()
}

struct ReturnReplacer;

impl VisitMut for ReturnReplacer {
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        if let Expr::Return(ret_expr) = node {
            let ret_expr_expr = ret_expr.expr.clone();
            *node = syn::parse_quote!(break 'run #ret_expr_expr);
        }

        visit_expr_mut(self, node);
    }

    fn visit_expr_closure_mut(&mut self, _node: &mut ExprClosure) {
        // Do not replace return statements inside closures.  Skip calling the base visitor.
    }
}

struct ImplDetector {
    found_impl: bool,
}

impl<'a> Visit<'a> for ImplDetector {
    fn visit_type_impl_trait(&mut self, _node: &'a TypeImplTrait) {
        self.found_impl = true;
    }
}
