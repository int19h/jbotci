/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{
    Attribute, Expr, ExprAsync, ExprCall, ExprClosure, Item, Lifetime, ReturnType, TypeImplTrait,
    spanned::Spanned,
    visit::{Visit, visit_return_type},
    visit_mut::{self as visitor, VisitMut, visit_block_mut, visit_expr_mut},
};

use crate::implementation::{Contract, ContractMode, ContractType, FuncWithContracts};

/// Substitution for `old()` expressions.
#[derive(Debug, Clone)]
pub(crate) struct OldExpr {
    /// Binding for the captured value.
    binding: Ident,
    /// Type witness for a disabled expensive capture.
    marker: Ident,
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

                    // The names stay in the macro's own name context, so a user
                    // name cannot collide with them. They point at the `old(...)`
                    // call, so the compiler reports a type error there.
                    let span = Span::mixed_site().located_at(expr.span());
                    let binding = Ident::new(&format!("__contract_old_{id}"), span);
                    let marker = Ident::new(&format!("__contract_old_{id}_marker"), span);

                    *expr = syn::parse_quote!(#binding);
                    self.olds.push(OldExpr {
                        binding,
                        marker,
                        expr: old_arg,
                    });
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
) -> Ident {
    assert!(
        ctype != ContractType::Invariant,
        "expected Invariant to be narrowed down to Pre/Post"
    );
    match mode {
        ContractMode::Always | ContractMode::Expensive => Ident::new("assert", span),
        ContractMode::Test => Ident::new("debug_assert", span),
        ContractMode::Disabled => {
            unreachable!("disabled contracts are removed before code generation")
        }
    }
}

// A statement that binds an `old(...)` value points at the captured
// expression, as before the generated names had their own context. The
// compiler then reports a moved value or a type error at that expression.
fn old_statement_span(old: &OldExpr) -> Span {
    Span::call_site().located_at(old.expr.span())
}

// Keep helper items in their own blocks so their names stay out of user expressions.
fn disabled_old_binding(old: &OldExpr) -> TokenStream {
    let binding = &old.binding;
    let marker = &old.marker;
    quote::quote_spanned! { old_statement_span(old)=>
        let #binding = {
            fn __bityzba_expensive_contract_old_value<T>(
                _: ::core::marker::PhantomData<T>,
            ) -> T {
                ::core::panic!("disabled expensive contract old() value was evaluated")
            }
            __bityzba_expensive_contract_old_value(#marker)
        };
    }
}

fn expensive_old_binding(old: &OldExpr, previous_expensive_olds: &[OldExpr]) -> TokenStream {
    let binding = &old.binding;
    let marker = &old.marker;
    let expr = &old.expr;
    let previous_old_bindings = previous_expensive_olds.iter().map(disabled_old_binding);

    quote::quote_spanned! { old_statement_span(old)=>
        #[cfg(feature = "expensive_contracts")]
        let #binding = #expr;

        #[cfg(not(feature = "expensive_contracts"))]
        #[allow(clippy::redundant_closure)]
        let #marker = if false {
            #(#previous_old_bindings)*
            ({
                fn __bityzba_expensive_contract_old_marker<
                    T,
                    F: ::core::ops::FnOnce() -> T,
                >(
                    _: F,
                ) -> ::core::marker::PhantomData<T> {
                    ::core::marker::PhantomData
                }
                __bityzba_expensive_contract_old_marker
            })(|| #expr)
        } else {
            ::core::marker::PhantomData
        };
    }
}

fn expensive_typecheck(exec_expr: &Expr, old_exprs: &[OldExpr]) -> TokenStream {
    let old_bindings = old_exprs.iter().map(disabled_old_binding);

    quote::quote! {
        #[allow(
            clippy::assertions_on_constants,
            clippy::nonminimal_bool,
            unreachable_code,
            unused_variables
        )]
        {
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

        let assert_macro = get_assert_macro(ctype, mode, span);
        result.extend(quote::quote_spanned! { span=>
            #[allow(clippy::assertions_on_constants, clippy::nonminimal_bool)] {
                #assert_macro!(#exec_expr, "{}", #format_args);
            }
        });

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
                    if mode == ContractMode::Expensive {
                        toks.extend(expensive_old_binding(old, &previous_expensive_olds));
                        previous_expensive_olds.push(old.clone());
                    } else {
                        let name = &old.binding;
                        let expr = &old.expr;

                        toks.extend(quote::quote_spanned! { old_statement_span(old)=>
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

    let result = Ident::new("ret", Span::call_site());
    let return_label = Lifetime::new("'run", Span::mixed_site());
    let body = 'blk: {
        let mut block = func.function.block.clone();
        visit_block_mut(
            &mut ReturnReplacer {
                label: &return_label,
            },
            &mut block,
        );

        let mut impl_detector = ImplDetector { found_impl: false };
        visit_return_type(&mut impl_detector, &func.function.sig.output);

        if !impl_detector.found_impl
            && let ReturnType::Type(.., ref return_type) = func.function.sig.output
        {
            break 'blk quote::quote! {
                let #result: #return_type = #return_label: #block;
            };
        }

        quote::quote! {
            let #result = #return_label: #block;
        }
    };

    //
    // create a new function body containing all assertions
    //

    let inner = quote::quote! {
        #pre
        #olds
        #body
        #post
        #result
    };
    // Keep the source span on the braces so rustc can report unused items.
    // Contract tokens retain their own spans and macro hygiene.
    let mut new_block = proc_macro2::Group::new(proc_macro2::Delimiter::Brace, inner);
    new_block.set_span(func.function.block.brace_token.span.join());

    // insert documentation attributes

    func.function.attrs.extend(docs);

    // replace the old function body with the new one

    *func.function.block = syn::parse_quote!(#new_block);

    func.function.into_token_stream()
}

struct ReturnReplacer<'label> {
    label: &'label Lifetime,
}

impl VisitMut for ReturnReplacer<'_> {
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        if let Expr::Return(ret_expr) = node {
            let ret_expr_expr = ret_expr.expr.clone();
            let label = self.label;
            *node = syn::parse_quote!(break #label #ret_expr_expr);
        }

        visit_expr_mut(self, node);
    }

    fn visit_expr_closure_mut(&mut self, _node: &mut ExprClosure) {
        // Do not replace return statements inside closures.  Skip calling the base visitor.
    }

    fn visit_expr_async_mut(&mut self, _node: &mut ExprAsync) {
        // Async blocks have their own return scope.
    }

    fn visit_item_mut(&mut self, _node: &mut Item) {
        // Function-local items have their own return scopes.
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
