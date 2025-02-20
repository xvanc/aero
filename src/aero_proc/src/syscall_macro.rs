/*
 * Copyright (C) 2021-2022 The Aero Project Developers.
 *
 * This file is part of The Aero Project.
 *
 * Aero is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Aero is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Aero. If not, see <https://www.gnu.org/licenses/>.
 */

use proc_macro::{Diagnostic, Level, TokenStream};
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{punctuated::Punctuated, spanned::Spanned, Expr, FnArg, Pat, Type};

enum ArgType {
    Array(bool),     // mutable?
    Slice(bool),     // mutable?
    Pointer(bool),   // mutable?
    Reference(bool), // mutable?
    String,
    Path,
}

pub fn parse(_: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_fn = syn::parse_macro_input!(item as syn::ItemFn);

    if let Some(constness) = parsed_fn.sig.constness {
        Diagnostic::spanned(
            constness.span().unwrap(),
            Level::Error,
            "syscall functions cannot be const",
        )
        .emit();
    }

    if let Some(asyncness) = parsed_fn.sig.asyncness {
        Diagnostic::spanned(
            asyncness.span().unwrap(),
            Level::Error,
            "syscall functions cannot be async",
        )
        .emit();
    }

    if let Some(unsafety) = parsed_fn.sig.unsafety {
        Diagnostic::spanned(
            unsafety.span().unwrap(),
            Level::Error,
            "syscall functions cannot be unsafe",
        )
        .emit();
    }

    if let Some(_) = parsed_fn.sig.generics.lt_token {
        let lt_span = parsed_fn.sig.generics.lt_token.span().unwrap();
        let gt_span = parsed_fn.sig.generics.gt_token.span().unwrap();

        Diagnostic::spanned(
            lt_span.join(gt_span).unwrap(),
            Level::Error,
            "syscall functions cannot have generic parameters",
        )
        .emit();
    }

    let attrs = &parsed_fn.attrs;
    let vis = &parsed_fn.vis;
    let name = &parsed_fn.sig.ident;
    let orig_args = &parsed_fn.sig.inputs;
    let processed_args = process_args(orig_args);
    let call_args = process_call_args(orig_args);
    let ret = &parsed_fn.sig.output;
    let body = &parsed_fn.block;

    let result = quote! {
        #(#attrs)* #vis fn #name(#(#processed_args),*) #ret {
            #(#attrs)* fn inner_syscall(#orig_args) #ret {
                #body
            }

            inner_syscall(#(#call_args),*)
        }
    };

    result.into()
}

fn determine_arg_type(typ: &Type) -> Option<ArgType> {
    match typ {
        Type::Reference(typ) => match typ.elem.as_ref() {
            Type::Array(_) => Some(ArgType::Array(typ.mutability.is_some())),
            Type::Slice(_) => Some(ArgType::Slice(typ.mutability.is_some())),
            Type::Path(path) => {
                if path.path.segments.last().unwrap().ident == "str" {
                    Some(ArgType::String)
                } else if path.path.segments.last().unwrap().ident == "Path" {
                    // NOTE: This will match to any type that has the name "Path"
                    Some(ArgType::Path)
                } else {
                    Some(ArgType::Reference(typ.mutability.is_some()))
                }
            }
            _ => None,
        },
        Type::Ptr(typ) => Some(ArgType::Pointer(typ.mutability.is_some())),
        _ => None,
    }
}

fn process_args(args: &Punctuated<FnArg, syn::Token![,]>) -> Vec<FnArg> {
    let mut result = Vec::new();

    for arg in args {
        match arg {
            FnArg::Typed(arg) => match arg.pat.as_ref() {
                Pat::Ident(pat) => {
                    let attrs = &arg.attrs;
                    let typ = &arg.ty;
                    let ident = &pat.ident;

                    match determine_arg_type(typ) {
                        Some(ArgType::Slice(_) | ArgType::String | ArgType::Path) => {
                            let data = Ident::new(&format!("{}_data", ident), Span::call_site());
                            let len = Ident::new(&format!("{}_len", ident), Span::call_site());

                            result.push(syn::parse_quote!(#data: usize));
                            result.push(syn::parse_quote!(#len: usize));
                        }
                        Some(ArgType::Array(_)) => {
                            let data = Ident::new(&format!("{}_data", ident), Span::call_site());

                            result.push(syn::parse_quote!(#data: usize));
                        }
                        Some(ArgType::Pointer(_)) | Some(ArgType::Reference(_)) => {
                            result.push(syn::parse_quote!(#(#attrs)* #ident: usize));
                        }
                        None => {
                            result.push(syn::parse_quote!(#(#attrs)* #ident: #typ));
                        }
                    };
                }
                _ => {
                    Diagnostic::spanned(
                        arg.span().unwrap(),
                        Level::Error,
                        "syscall function arguments cannot have non-ident patterns",
                    )
                    .emit();
                }
            },
            FnArg::Receiver(_) => {
                Diagnostic::spanned(
                    arg.span().unwrap(),
                    Level::Error,
                    "syscall functions cannot have receiver arguments",
                )
                .emit();
            }
        }
    }

    result
}

fn process_call_args(args: &Punctuated<FnArg, syn::Token![,]>) -> Vec<Expr> {
    let mut result = Vec::new();

    for arg in args {
        match arg {
            FnArg::Typed(arg) => match arg.pat.as_ref() {
                Pat::Ident(pat) => {
                    let ty = &arg.ty;
                    let ident = &pat.ident;

                    if let Some(arg_type) = determine_arg_type(ty) {
                        let data_ident = Ident::new(&format!("{}_data", ident), Span::call_site());
                        let len_ident = Ident::new(&format!("{}_len", ident), Span::call_site());

                        match arg_type {
                            ArgType::Slice(is_mut) => {
                                let slice_expr: Expr = if is_mut {
                                    syn::parse_quote! {
                                        crate::utils::validate_slice_mut(#data_ident as *mut _, #len_ident).ok_or(AeroSyscallError::EINVAL)?
                                    }
                                } else {
                                    syn::parse_quote! {
                                        crate::utils::validate_slice(#data_ident as *const _, #len_ident).ok_or(AeroSyscallError::EINVAL)?
                                    }
                                };

                                result.push(slice_expr);
                            }
                            ArgType::Array(is_mut) => {
                                let array_expr: Expr = if is_mut {
                                    syn::parse_quote! {
                                        crate::utils::validate_array_mut(#data_ident as *mut _).ok_or(AeroSyscallError::EINVAL)?
                                    }
                                } else {
                                    unimplemented!()
                                };

                                result.push(array_expr);
                            }
                            ArgType::Pointer(is_mut) => {
                                let ptr_expr: Expr = if is_mut {
                                    syn::parse_quote!(#ident as *mut _)
                                } else {
                                    syn::parse_quote!(#ident as *const _)
                                };

                                result.push(ptr_expr);
                            }
                            ArgType::Reference(is_mut) => {
                                let ref_expr: Expr = if is_mut {
                                    syn::parse_quote!({
                                        crate::utils::validate_mut_ptr(#ident as *mut _).ok_or(AeroSyscallError::EINVAL)?
                                    })
                                } else {
                                    syn::parse_quote!({
                                        crate::utils::validate_ptr(#ident as *const _).ok_or(AeroSyscallError::EINVAL)?
                                    })
                                };

                                result.push(ref_expr);
                            }
                            ArgType::String => result.push(syn::parse_quote! {
                                crate::utils::validate_str(#data_ident as *const _, #len_ident).ok_or(AeroSyscallError::EINVAL)?
                            }),
                            ArgType::Path => result.push(syn::parse_quote! {
                                {
                                    let string = crate::utils::validate_str(#data_ident as *const _, #len_ident).ok_or(AeroSyscallError::EINVAL)?;
                                    let path = Path::new(string);
                                    path
                                }
                            }),
                        }
                    } else {
                        result.push(syn::parse_quote!(#ident));
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    result
}
