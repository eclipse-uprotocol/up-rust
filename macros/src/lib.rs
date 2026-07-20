//! Derive macros for up-rust stable payloads: `StablePayload`,
//! `ByteBackedStablePayload`, and `StablePayloadInit`.

/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    parse_macro_input, spanned::Spanned, Data, DeriveInput, Expr, Fields, Ident, Lit, LitStr, Type,
};

struct StablePayloadArgs {
    type_name: String,
}

#[proc_macro_derive(StablePayload, attributes(stable_payload))]
/// Derives `StablePayload`: a fixed-layout type whose bytes are stable across processes.
pub fn derive_stable_payload(input: TokenStream) -> TokenStream {
    expand_stable_payload(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(ByteBackedStablePayload, attributes(stable_payload))]
/// Derives `ByteBackedStablePayload`: marks every byte pattern of the type as valid.
pub fn derive_byte_backed_stable_payload(input: TokenStream) -> TokenStream {
    expand_byte_backed_stable_payload(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(StablePayloadInit, attributes(stable_payload, stable_payload_init))]
/// Derives `StablePayloadInit`: a typestate initializer that writes the type into uninitialized storage field by field.
pub fn derive_stable_payload_init(input: TokenStream) -> TokenStream {
    expand_stable_payload_init(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_stable_payload(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let args = parse_args(&input)?;
    let name = &input.ident;
    let shape = analyze_stable_payload_shape(&input)?;
    let field_checks = shape.field_checks;
    let type_name = args.type_name;

    Ok(quote! {
        const _: () = {
            assert!(
                !::core::mem::needs_drop::<#name>(),
                "StablePayload types must not implement Drop or contain fields that need drop"
            );
        };

        // SAFETY:
        // - The derive requires `#[repr(C)]` or `#[repr(transparent)]`, rejects
        //   process-local fields, and rejects types that need drop glue.
        // - Field checks recursively require every field to satisfy up-rust's
        //   stable payload field proof.
        // - The user-provided `type_name` is the stable identity for this type.
        unsafe impl ::up_rust::payload::stable::StablePayload for #name {
            const TYPE_NAME: &'static str = #type_name;

            fn __stable_payload_field_check(&self) {
                #field_checks
            }
        }

        // SAFETY: The generated `StablePayload` impl above establishes the field
        // proof required when this type is nested in another stable payload.
        unsafe impl ::up_rust::payload::stable::StablePayloadField for #name {}
    })
}

fn expand_byte_backed_stable_payload(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let shape = analyze_stable_payload_shape(&input)?;
    let field_size_sum = shape.field_size_sum;
    let byte_backed_field_checks = shape.byte_backed_field_checks;
    let field_offset_checks = shape.field_offset_checks;

    Ok(quote! {
        const _: () = {
            #field_offset_checks
            #byte_backed_field_checks
            assert!(
                !::core::mem::needs_drop::<#name>(),
                "ByteBackedStablePayload types must not implement Drop or contain fields that need drop"
            );
            assert!(
                ::core::mem::size_of::<#name>() == (#field_size_sum),
                "ByteBackedStablePayload types must not have implicit trailing padding; use explicit initialized padding fields"
            );
        };

        // SAFETY:
        // - The derive checked that fields exactly cover `size_of::<Self>()`, so
        //   there is no implicit inter-field or trailing padding.
        // - Every field is recursively byte-backed and `Self` does not need drop
        //   glue, so safe construction initializes every transported byte.
        unsafe impl ::up_rust::payload::stable::ByteBackedStablePayload for #name {}
    })
}

fn expand_stable_payload_init(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    reject_stable_payload_init_attrs(&input.attrs)?;
    let name = &input.ident;
    let shape = analyze_stable_payload_init_shape(&input)?;
    let init_ident = format_ident!("__UpRustStablePayloadInitFor{}", name);
    let fields = shape.fields;
    let padding_init = shape.padding_init;
    let state_idents = fields
        .iter()
        .map(|field| field.state_ident.clone())
        .collect::<Vec<_>>();
    let initial_states = fields
        .iter()
        .map(|_| quote! { ::up_rust::__derive_support::StablePayloadInitUnset })
        .collect::<Vec<_>>();
    let set_states = fields
        .iter()
        .map(|_| quote! { ::up_rust::__derive_support::StablePayloadInitSet })
        .collect::<Vec<_>>();
    let initial_ty = builder_type(&init_ident, quote! { '__up_rust_payload }, &initial_states);
    let finish_ty = builder_type(&init_ident, quote! { '__up_rust_payload }, &set_states);
    let builder_def_generics = builder_def_generics(&state_idents);
    let builder_state_tuple = quote! { (#(#state_idents,)*) };
    let constructor_impl_generics = builder_def_generics.clone();
    let constructor_ty =
        builder_type_from_idents(&init_ident, quote! { '__up_rust_payload }, &state_idents);
    let setter_impls = fields
        .iter()
        .map(|field| generate_init_field_methods(&init_ident, &fields, field))
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #[doc(hidden)]
        #[allow(private_bounds, private_interfaces)]
        pub struct #init_ident #builder_def_generics {
            __up_rust_slot: ::up_rust::__derive_support::StablePayloadInitSlot<'__up_rust_payload, #name>,
            __up_rust_state: ::core::marker::PhantomData<fn() -> #builder_state_tuple>,
        }

        #[allow(private_bounds, private_interfaces)]
        impl #constructor_impl_generics #constructor_ty {
            #[inline(always)]
            fn __up_rust_from_slot(
                __up_rust_slot: ::up_rust::__derive_support::StablePayloadInitSlot<'__up_rust_payload, #name>,
            ) -> Self {
                Self {
                    __up_rust_slot,
                    __up_rust_state: ::core::marker::PhantomData,
                }
            }
        }

        // SAFETY:
        // - The generated initializer writes every semantic field exactly once
        //   before `finish()` is available.
        // - `__init_from_slot` initializes only implicit/trailing padding gaps;
        //   field setters initialize the field byte ranges with typed valid
        //   values or nested generated initializers.
        // - The generated `finish()` is implemented only for the all-set
        //   typestate, so successful completion proves the transported byte range
        //   contains one initialized `Self`.
        unsafe impl ::up_rust::payload::stable::StablePayloadInit for #name {
            type Init<'__up_rust_payload> = #initial_ty;

            #[inline(always)]
            fn __init_from_slot<'__up_rust_payload>(
                mut __up_rust_slot: ::up_rust::__derive_support::StablePayloadInitSlot<'__up_rust_payload, Self>,
            ) -> Result<Self::Init<'__up_rust_payload>, ::up_rust::payload::UWireError> {
                #padding_init
                Ok(#init_ident::__up_rust_from_slot(__up_rust_slot))
            }
        }

        #[allow(private_bounds, private_interfaces)]
        impl<'__up_rust_payload> #finish_ty {
            #[inline(always)]
            pub fn finish(
                self,
            ) -> Result<
                ::up_rust::payload::stable::InitializedStablePayload<'__up_rust_payload, #name>,
                ::up_rust::payload::UWireError,
            > {
                // SAFETY: This impl exists only for the all-set typestate. The
                // generated constructor initialized padding gaps, and each setter
                // transitions exactly one field from unset to set after writing it.
                let __up_rust_initialized = self.__up_rust_slot.assume_init();
                Ok(__up_rust_initialized)
            }
        }

        #(#setter_impls)*
    })
}

#[derive(Clone)]
struct StablePayloadInitShape {
    fields: Vec<InitField>,
    padding_init: proc_macro2::TokenStream,
}

#[derive(Clone)]
struct InitField {
    method_ident: Ident,
    ty: Type,
    offset: proc_macro2::TokenStream,
    state_ident: Ident,
    kind: InitFieldKind,
    all_field_names: Vec<String>,
}

#[derive(Clone)]
enum InitFieldKind {
    Scalar,
    Nested,
    Array {
        elem: Type,
        len: Expr,
        elem_kind: InitArrayElemKind,
    },
}

#[derive(Clone, Copy)]
enum InitArrayElemKind {
    U8,
    TypedCopy,
    Nested,
}

fn analyze_stable_payload_init_shape(input: &DeriveInput) -> syn::Result<StablePayloadInitShape> {
    ensure_repr_c_or_transparent(input)?;
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "StablePayloadInit derive does not support generic types yet",
        ));
    }

    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let all_field_names = fields
                    .named
                    .iter()
                    .filter_map(|field| field.ident.as_ref())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                let mut init_fields = Vec::new();
                let mut padding_steps = Vec::new();
                for (index, field) in fields.named.iter().enumerate() {
                    reject_stable_payload_init_attrs(&field.attrs)?;
                    validate_stable_field_type(&field.ty)?;
                    let name = &input.ident;
                    let ident = field.ident.as_ref().expect("named fields have identifiers");
                    let ty = field.ty.clone();
                    let offset = quote! { ::core::mem::offset_of!(#name, #ident) };
                    padding_steps.push(padding_step_for_field(input, &offset, &ty, ident.span()));
                    init_fields.push(InitField {
                        method_ident: ident.clone(),
                        ty: ty.clone(),
                        offset,
                        state_ident: format_ident!("__UpRustField{}", index),
                        kind: classify_init_field(&ty),
                        all_field_names: all_field_names.clone(),
                    });
                }
                Ok(StablePayloadInitShape {
                    fields: init_fields,
                    padding_init: padding_init_tokens(input, padding_steps),
                })
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field = fields.unnamed.first().expect("one tuple field");
                reject_stable_payload_init_attrs(&field.attrs)?;
                validate_stable_field_type(&field.ty)?;
                let ty = field.ty.clone();
                let offset = quote! { 0_usize };
                Ok(StablePayloadInitShape {
                    fields: vec![InitField {
                        method_ident: format_ident!("field0"),
                        ty: ty.clone(),
                        offset: offset.clone(),
                        state_ident: format_ident!("__UpRustField0"),
                        kind: classify_init_field(&ty),
                        all_field_names: Vec::new(),
                    }],
                    padding_init: padding_init_tokens(
                        input,
                        vec![padding_step_for_field(input, &offset, &ty, field.span())],
                    ),
                })
            }
            Fields::Unnamed(fields) => {
                for field in &fields.unnamed {
                    reject_stable_payload_init_attrs(&field.attrs)?;
                    validate_stable_field_type(&field.ty)?;
                }
                Err(syn::Error::new_spanned(
                    &data.fields,
                    "StablePayloadInit supports named structs, unit structs, or one-field repr(transparent) structs",
                ))
            }
            Fields::Unit => Ok(StablePayloadInitShape {
                fields: Vec::new(),
                padding_init: padding_init_tokens(input, Vec::new()),
            }),
        },
        Data::Enum(data) => Err(syn::Error::new_spanned(
            data.enum_token,
            "StablePayloadInit derive does not support enums",
        )),
        Data::Union(data) => Err(syn::Error::new_spanned(
            data.union_token,
            "StablePayloadInit derive does not support unions",
        )),
    }
}

fn padding_step_for_field(
    input: &DeriveInput,
    offset: &proc_macro2::TokenStream,
    ty: &Type,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    let name = &input.ident;
    quote_spanned! {span=>
        let __up_rust_field_offset = #offset;
        if __up_rust_padding_cursor < __up_rust_field_offset {
            // SAFETY: The generated offset calculation names only the implicit
            // padding gap before this field.
                            __up_rust_slot.write_padding(
                    __up_rust_padding_cursor,
                    __up_rust_field_offset - __up_rust_padding_cursor,
                );
        }
        __up_rust_padding_cursor = __up_rust_field_offset + ::core::mem::size_of::<#ty>();
        let _ = ::core::mem::size_of::<#name>();
    }
}

fn padding_init_tokens(
    input: &DeriveInput,
    padding_steps: Vec<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let name = &input.ident;
    quote! {
        let mut __up_rust_padding_cursor = 0_usize;
        #(#padding_steps)*
        let __up_rust_payload_size = ::core::mem::size_of::<#name>();
        if __up_rust_padding_cursor < __up_rust_payload_size {
            // SAFETY: The generated cursor has advanced past every semantic
            // field; the remaining range is trailing padding.
                            __up_rust_slot.write_padding(
                    __up_rust_padding_cursor,
                    __up_rust_payload_size - __up_rust_padding_cursor,
                );
        }
    }
}

fn classify_init_field(ty: &Type) -> InitFieldKind {
    match ty {
        Type::Array(array) => {
            let elem = (*array.elem).clone();
            let elem_kind = if is_u8_type(&elem) {
                InitArrayElemKind::U8
            } else if is_typed_copy_init_type(&elem) {
                InitArrayElemKind::TypedCopy
            } else {
                InitArrayElemKind::Nested
            };
            InitFieldKind::Array {
                elem,
                len: array.len.clone(),
                elem_kind,
            }
        }
        _ if is_typed_copy_init_type(ty) => InitFieldKind::Scalar,
        _ => InitFieldKind::Nested,
    }
}

fn is_typed_copy_init_type(ty: &Type) -> bool {
    match ty {
        Type::Path(value) => value
            .path
            .segments
            .last()
            .is_some_and(|segment| is_primitive_segment(&segment.ident)),
        Type::Tuple(value) if value.elems.is_empty() => true,
        Type::Array(value) => is_typed_copy_init_type(&value.elem),
        _ => false,
    }
}

fn is_u8_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(value) if path_last_segment_is(value, "u8"))
}

fn is_primitive_segment(ident: &Ident) -> bool {
    matches!(
        ident.to_string().as_str(),
        "bool"
            | "char"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
    )
}

fn builder_def_generics(state_idents: &[Ident]) -> proc_macro2::TokenStream {
    if state_idents.is_empty() {
        quote! { <'__up_rust_payload> }
    } else {
        quote! { <'__up_rust_payload, #(#state_idents),*> }
    }
}

fn builder_type_from_idents(
    init_ident: &Ident,
    lifetime: proc_macro2::TokenStream,
    states: &[Ident],
) -> proc_macro2::TokenStream {
    let states = states
        .iter()
        .map(|ident| quote! { #ident })
        .collect::<Vec<_>>();
    builder_type(init_ident, lifetime, &states)
}

fn builder_type(
    init_ident: &Ident,
    lifetime: proc_macro2::TokenStream,
    states: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    if states.is_empty() {
        quote! { #init_ident<#lifetime> }
    } else {
        quote! { #init_ident<#lifetime, #(#states),*> }
    }
}

fn generate_init_field_methods(
    init_ident: &Ident,
    fields: &[InitField],
    field: &InitField,
) -> syn::Result<proc_macro2::TokenStream> {
    let ty = &field.ty;
    let method = &field.method_ident;
    let offset = &field.offset;
    let (impl_generics, self_ty, next_ty) = setter_types(init_ident, fields, field);

    let methods = match &field.kind {
        InitFieldKind::Scalar => quote_spanned! {method.span()=>
            #[inline(always)]
            pub fn #method(self, value: #ty) -> #next_ty {
                let mut __up_rust_slot = self.__up_rust_slot;
                __up_rust_slot.write_field::<#ty>(#offset, value);
                #init_ident::__up_rust_from_slot(__up_rust_slot)
            }
        },
        InitFieldKind::Nested => {
            nested_field_methods(init_ident, ty, method, offset, &next_ty, field)
        }
        InitFieldKind::Array {
            elem,
            len,
            elem_kind,
        } => array_field_methods(
            init_ident, ty, elem, len, *elem_kind, method, offset, &next_ty, field,
        ),
    };

    Ok(quote! {
        #[allow(private_bounds, private_interfaces)]
        impl #impl_generics #self_ty {
            #methods
        }
    })
}

fn setter_types(
    init_ident: &Ident,
    fields: &[InitField],
    field: &InitField,
) -> (
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
) {
    let mut impl_state_params = Vec::new();
    let mut self_states = Vec::new();
    let mut next_states = Vec::new();
    for existing in fields {
        if existing.state_ident == field.state_ident {
            self_states.push(quote! { ::up_rust::__derive_support::StablePayloadInitUnset });
            next_states.push(quote! { ::up_rust::__derive_support::StablePayloadInitSet });
        } else {
            let ident = &existing.state_ident;
            impl_state_params.push(ident.clone());
            self_states.push(quote! { #ident });
            next_states.push(quote! { #ident });
        }
    }

    let impl_generics = if impl_state_params.is_empty() {
        quote! { <'__up_rust_payload> }
    } else {
        quote! { <'__up_rust_payload, #(#impl_state_params),*> }
    };
    let self_ty = builder_type(init_ident, quote! { '__up_rust_payload }, &self_states);
    let next_ty = builder_type(init_ident, quote! { '__up_rust_payload }, &next_states);
    (impl_generics, self_ty, next_ty)
}

fn nested_field_methods(
    init_ident: &Ident,
    ty: &Type,
    method: &Ident,
    offset: &proc_macro2::TokenStream,
    next_ty: &proc_macro2::TokenStream,
    field: &InitField,
) -> proc_macro2::TokenStream {
    let value_method = helper_ident(method, "value", field);
    let value_method_tokens = value_method.map(|value_method| {
        quote_spanned! {value_method.span()=>
            #[inline(always)]
            pub fn #value_method<__UpRustValue>(self, value: __UpRustValue) -> #next_ty
            where
                __UpRustValue: ::up_rust::payload::stable::StablePayloadInitCompleteValue<#ty>,
            {
                let mut __up_rust_slot = self.__up_rust_slot;
                __up_rust_slot.write_field::<#ty>(
                    #offset,
                    ::up_rust::payload::stable::StablePayloadInitCompleteValue::into_complete_value(value),
                );
                #init_ident::__up_rust_from_slot(__up_rust_slot)
            }
        }
    });

    quote_spanned! {method.span()=>
        #[inline(always)]
        pub fn #method(
            self,
            init: impl for<'__up_rust_slot> FnOnce(
                ::up_rust::payload::stable::StablePayloadInitContext<'__up_rust_slot, #ty>,
            ) -> Result<
                ::up_rust::payload::stable::InitializedStablePayload<'__up_rust_slot, #ty>,
                ::up_rust::payload::UWireError,
            >,
        ) -> Result<#next_ty, ::up_rust::payload::UWireError>
        where
            #ty: ::up_rust::payload::stable::StablePayloadInit,
        {
            let mut __up_rust_slot = self.__up_rust_slot;
            let __up_rust_nested_slot = __up_rust_slot.field_slot::<#ty>(#offset);
            let __up_rust_context =
                <#ty as ::up_rust::payload::stable::StablePayloadInit>::__init_context_from_slot(
                    __up_rust_nested_slot,
                )?;
            let _ = init(__up_rust_context)?;
            Ok(#init_ident::__up_rust_from_slot(__up_rust_slot))
        }

        #value_method_tokens
    }
}

#[allow(clippy::too_many_arguments)]
fn array_field_methods(
    init_ident: &Ident,
    ty: &Type,
    elem: &Type,
    len: &Expr,
    elem_kind: InitArrayElemKind,
    method: &Ident,
    offset: &proc_macro2::TokenStream,
    next_ty: &proc_macro2::TokenStream,
    field: &InitField,
) -> proc_macro2::TokenStream {
    match elem_kind {
        InitArrayElemKind::U8 => {
            u8_array_methods(init_ident, ty, len, method, offset, next_ty, field)
        }
        InitArrayElemKind::TypedCopy => {
            typed_array_methods(init_ident, ty, elem, len, method, offset, next_ty, field)
        }
        InitArrayElemKind::Nested => {
            nested_array_methods(init_ident, ty, elem, len, method, offset, next_ty, field)
        }
    }
}

fn u8_array_methods(
    init_ident: &Ident,
    ty: &Type,
    len: &Expr,
    method: &Ident,
    offset: &proc_macro2::TokenStream,
    next_ty: &proc_macro2::TokenStream,
    field: &InitField,
) -> proc_macro2::TokenStream {
    let from_array = helper_ident(method, "from_array", field);
    let from_slice = helper_ident(method, "from_slice", field);
    let fill = helper_ident(method, "fill", field);
    let fill_with = helper_ident(method, "fill_with", field);
    let from_array_tokens = from_array.map(|ident| {
        quote_spanned! {ident.span()=>
            #[inline(always)]
            pub fn #ident(self, value: &[u8; #len]) -> #next_ty {
                let mut __up_rust_slot = self.__up_rust_slot;
                __up_rust_slot.write_bytes(#offset, value);
                #init_ident::__up_rust_from_slot(__up_rust_slot)
            }
        }
    });
    let from_slice_tokens = from_slice.map(|ident| {
        quote_spanned! {ident.span()=>
            #[inline(always)]
            pub fn #ident(self, value: &[u8]) -> Result<#next_ty, ::up_rust::payload::UWireError> {
                let __up_rust_expected = #len;
                if value.len() != __up_rust_expected {
                    return Err(::up_rust::payload::UWireError::invalid_payload_length(
                        __up_rust_expected,
                        value.len(),
                    ));
                }
                let mut __up_rust_slot = self.__up_rust_slot;
                __up_rust_slot.write_bytes(#offset, value);
                Ok(#init_ident::__up_rust_from_slot(__up_rust_slot))
            }
        }
    });
    let fill_tokens = fill.map(|ident| {
        quote_spanned! {ident.span()=>
            #[inline(always)]
            pub fn #ident(self, value: u8) -> #next_ty {
                let mut __up_rust_slot = self.__up_rust_slot;
                __up_rust_slot.fill_bytes(#offset, #len, value);
                #init_ident::__up_rust_from_slot(__up_rust_slot)
            }
        }
    });
    let fill_with_tokens = fill_with.map(|ident| {
        quote_spanned! {ident.span()=>
            #[inline(always)]
            pub fn #ident(self, value: impl FnMut(usize) -> u8) -> #next_ty {
                let mut __up_rust_slot = self.__up_rust_slot;
                __up_rust_slot.fill_bytes_with(#offset, #len, value);
                #init_ident::__up_rust_from_slot(__up_rust_slot)
            }
        }
    });

    quote_spanned! {method.span()=>
        #[inline(always)]
        pub fn #method(self, value: #ty) -> #next_ty {
            let mut __up_rust_slot = self.__up_rust_slot;
            __up_rust_slot.write_field::<#ty>(#offset, value);
            #init_ident::__up_rust_from_slot(__up_rust_slot)
        }

        #from_array_tokens
        #from_slice_tokens
        #fill_tokens
        #fill_with_tokens
    }
}

fn typed_array_methods(
    init_ident: &Ident,
    ty: &Type,
    elem: &Type,
    len: &Expr,
    method: &Ident,
    offset: &proc_macro2::TokenStream,
    next_ty: &proc_macro2::TokenStream,
    field: &InitField,
) -> proc_macro2::TokenStream {
    let from_array = helper_ident(method, "from_array", field);
    let from_slice = helper_ident(method, "from_slice", field);
    let fill = helper_ident(method, "fill", field);
    let from_array_tokens = from_array.map(|ident| {
        quote_spanned! {ident.span()=>
            #[inline(always)]
            pub fn #ident(self, value: &[#elem; #len]) -> #next_ty
            where
                #elem: Copy,
            {
                let mut __up_rust_slot = self.__up_rust_slot;
                __up_rust_slot
                    .copy_array_from_slice::<#elem>(#offset, value, #len)
                    .expect("array reference length matches generated stable payload field length");
                #init_ident::__up_rust_from_slot(__up_rust_slot)
            }
        }
    });
    let from_slice_tokens = from_slice.map(|ident| {
        quote_spanned! {ident.span()=>
            #[inline(always)]
            pub fn #ident(self, value: &[#elem]) -> Result<#next_ty, ::up_rust::payload::UWireError>
            where
                #elem: Copy,
            {
                let mut __up_rust_slot = self.__up_rust_slot;
                __up_rust_slot.copy_array_from_slice::<#elem>(#offset, value, #len)?;
                Ok(#init_ident::__up_rust_from_slot(__up_rust_slot))
            }
        }
    });
    let fill_tokens = fill.map(|ident| {
        quote_spanned! {ident.span()=>
            #[inline(always)]
            pub fn #ident(self, value: #elem) -> #next_ty
            where
                #elem: Copy,
            {
                let mut __up_rust_slot = self.__up_rust_slot;
                __up_rust_slot.fill_array::<#elem>(#offset, #len, value);
                #init_ident::__up_rust_from_slot(__up_rust_slot)
            }
        }
    });

    quote_spanned! {method.span()=>
        #[inline(always)]
        pub fn #method(self, value: #ty) -> #next_ty {
            let mut __up_rust_slot = self.__up_rust_slot;
            __up_rust_slot.write_field::<#ty>(#offset, value);
            #init_ident::__up_rust_from_slot(__up_rust_slot)
        }

        #from_array_tokens
        #from_slice_tokens
        #fill_tokens
    }
}

fn nested_array_methods(
    init_ident: &Ident,
    ty: &Type,
    elem: &Type,
    len: &Expr,
    method: &Ident,
    offset: &proc_macro2::TokenStream,
    next_ty: &proc_macro2::TokenStream,
    field: &InitField,
) -> proc_macro2::TokenStream {
    let value_method = helper_ident(method, "value", field);
    let value_method_tokens = value_method.map(|value_method| {
        quote_spanned! {value_method.span()=>
            #[inline(always)]
            pub fn #value_method<__UpRustValue>(self, value: __UpRustValue) -> #next_ty
            where
                __UpRustValue: ::up_rust::payload::stable::StablePayloadInitCompleteValue<#ty>,
            {
                let mut __up_rust_slot = self.__up_rust_slot;
                __up_rust_slot.write_field::<#ty>(
                    #offset,
                    ::up_rust::payload::stable::StablePayloadInitCompleteValue::into_complete_value(value),
                );
                #init_ident::__up_rust_from_slot(__up_rust_slot)
            }
        }
    });

    quote_spanned! {method.span()=>
        #[inline(always)]
        pub fn #method(
            self,
            mut init: impl for<'__up_rust_slot> FnMut(
                usize,
                ::up_rust::payload::stable::StablePayloadInitContext<'__up_rust_slot, #elem>,
            ) -> Result<
                ::up_rust::payload::stable::InitializedStablePayload<'__up_rust_slot, #elem>,
                ::up_rust::payload::UWireError,
            >,
        ) -> Result<#next_ty, ::up_rust::payload::UWireError>
        where
            #elem: ::up_rust::payload::stable::StablePayloadInit,
        {
            let mut __up_rust_slot = self.__up_rust_slot;
            let __up_rust_len = #len;
            for __up_rust_index in 0..__up_rust_len {
                let __up_rust_element_slot = __up_rust_slot
                    .array_element_slot::<#elem>(#offset, __up_rust_index);
                let __up_rust_context =
                    <#elem as ::up_rust::payload::stable::StablePayloadInit>::__init_context_from_slot(
                        __up_rust_element_slot,
                    )?;
                let _ = init(__up_rust_index, __up_rust_context)?;
            }
            Ok(#init_ident::__up_rust_from_slot(__up_rust_slot))
        }

        #value_method_tokens
    }
}

fn helper_ident(base: &Ident, suffix: &str, field: &InitField) -> Option<Ident> {
    let helper_name = format!("{}_{}", base, suffix);
    if field
        .all_field_names
        .iter()
        .any(|name| name == &helper_name)
    {
        None
    } else {
        Some(format_ident!("{}", helper_name, span = base.span()))
    }
}

fn reject_stable_payload_init_attrs(attrs: &[syn::Attribute]) -> syn::Result<()> {
    for attr in attrs {
        if !attr.path().is_ident("stable_payload_init") {
            continue;
        }
        return Err(syn::Error::new_spanned(
            attr,
            "unsupported stable_payload_init attribute; no stable_payload_init field or type attributes are currently defined",
        ));
    }
    Ok(())
}

struct StablePayloadShape {
    field_checks: proc_macro2::TokenStream,
    field_size_sum: proc_macro2::TokenStream,
    byte_backed_field_checks: proc_macro2::TokenStream,
    field_offset_checks: proc_macro2::TokenStream,
}

fn analyze_stable_payload_shape(input: &DeriveInput) -> syn::Result<StablePayloadShape> {
    ensure_repr_c_or_transparent(input)?;
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "StablePayload derive does not support generic types yet",
        ));
    }

    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let name = &input.ident;
                let mut checks = Vec::new();
                let mut field_types = Vec::new();
                let mut preceding_sizes = Vec::new();
                let mut field_offset_checks = Vec::new();
                let mut byte_backed_field_checks = Vec::new();

                for field in &fields.named {
                    let ident = field.ident.as_ref().expect("named fields have identifiers");
                    validate_stable_field_type(&field.ty)?;
                    let ty = &field.ty;
                    checks.push(quote_spanned! {ty.span()=>
                        <#ty as ::up_rust::payload::stable::StablePayloadField>::__stable_payload_field_check(&self.#ident);
                    });

                    let expected_offset =
                        quote! { 0_usize #(+ ::core::mem::size_of::<#preceding_sizes>())* };
                    let padding_message = LitStr::new(
                        &format!(
                            "ByteBackedStablePayload field `{ident}` has implicit padding before it; add explicit initialized padding fields"
                        ),
                        ident.span(),
                    );
                    field_offset_checks.push(quote_spanned! {ident.span()=>
                        assert!(
                            ::core::mem::offset_of!(#name, #ident) == (#expected_offset),
                            #padding_message
                        );
                    });

                    let byte_backed_message = LitStr::new(
                        &format!(
                            "ByteBackedStablePayload field `{ident}` must be recursively byte-backed"
                        ),
                        ident.span(),
                    );
                    byte_backed_field_checks.push(quote_spanned! {ty.span()=>
                        assert!(
                            <#ty as ::up_rust::payload::stable::ByteBackedStablePayloadField>::SUPPORTS_BYTE_BACKED_STABLE_FIELD,
                            #byte_backed_message
                        );
                    });
                    preceding_sizes.push(ty);
                    field_types.push(ty);
                }

                Ok(StablePayloadShape {
                    field_checks: quote! { #(#checks)* },
                    field_size_sum: quote! { 0_usize #(+ ::core::mem::size_of::<#field_types>())* },
                    byte_backed_field_checks: quote! { #(#byte_backed_field_checks)* },
                    field_offset_checks: quote! { #(#field_offset_checks)* },
                })
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let mut checks = Vec::new();
                let mut field_types = Vec::new();
                let mut byte_backed_field_checks = Vec::new();

                for (field_index, field) in fields.unnamed.iter().enumerate() {
                    validate_stable_field_type(&field.ty)?;
                    let index = syn::Index::from(field_index);
                    let ty = &field.ty;
                    checks.push(quote_spanned! {ty.span()=>
                        <#ty as ::up_rust::payload::stable::StablePayloadField>::__stable_payload_field_check(&self.#index);
                    });

                    let byte_backed_message = LitStr::new(
                        &format!(
                            "ByteBackedStablePayload tuple field `{field_index}` must be recursively byte-backed"
                        ),
                        field.span(),
                    );
                    byte_backed_field_checks.push(quote_spanned! {ty.span()=>
                        assert!(
                            <#ty as ::up_rust::payload::stable::ByteBackedStablePayloadField>::SUPPORTS_BYTE_BACKED_STABLE_FIELD,
                            #byte_backed_message
                        );
                    });
                    field_types.push(ty);
                }

                Ok(StablePayloadShape {
                    field_checks: quote! { #(#checks)* },
                    field_size_sum: quote! { 0_usize #(+ ::core::mem::size_of::<#field_types>())* },
                    byte_backed_field_checks: quote! { #(#byte_backed_field_checks)* },
                    field_offset_checks: quote! {},
                })
            }
            Fields::Unnamed(fields) => {
                for field in &fields.unnamed {
                    validate_stable_field_type(&field.ty)?;
                }
                Err(syn::Error::new_spanned(
                    &data.fields,
                    "StablePayload supports named structs or one-field repr(transparent) structs",
                ))
            }
            Fields::Unit => Ok(StablePayloadShape {
                field_checks: quote! {},
                field_size_sum: quote! { 0_usize },
                byte_backed_field_checks: quote! {},
                field_offset_checks: quote! {},
            }),
        },
        Data::Enum(data) => Err(syn::Error::new_spanned(
            data.enum_token,
            "StablePayload derive does not support enums",
        )),
        Data::Union(data) => Err(syn::Error::new_spanned(
            data.union_token,
            "StablePayload derive does not support unions",
        )),
    }
}

fn parse_args(input: &DeriveInput) -> syn::Result<StablePayloadArgs> {
    let mut type_name = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("stable_payload") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("type_name") {
                type_name = Some(meta.value()?.parse::<Lit>()?);
                Ok(())
            } else {
                Err(meta.error("unsupported stable_payload attribute key; expected type_name"))
            }
        })?;
    }

    let type_name = match type_name {
        Some(Lit::Str(value)) => value.value(),
        Some(other) => {
            return Err(syn::Error::new_spanned(
                other,
                "stable_payload type_name must be a string literal",
            ));
        }
        None => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "StablePayload derive requires #[stable_payload(type_name = \"...\")]",
            ));
        }
    };
    Ok(StablePayloadArgs { type_name })
}

fn ensure_repr_c_or_transparent(input: &DeriveInput) -> syn::Result<()> {
    let mut has_supported_repr = false;
    for attr in &input.attrs {
        if !attr.path().is_ident("repr") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("packed") {
                return Err(meta.error("StablePayload does not support repr(packed)"));
            }
            if meta.path.is_ident("C") || meta.path.is_ident("transparent") {
                has_supported_repr = true;
            }
            Ok(())
        })?;
    }

    if !has_supported_repr {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "StablePayload requires #[repr(C)] or #[repr(transparent)]",
        ));
    }
    Ok(())
}

fn validate_stable_field_type(ty: &Type) -> syn::Result<()> {
    match ty {
        Type::Reference(value) => Err(syn::Error::new_spanned(
            value,
            "StablePayload fields must not be references",
        )),
        Type::Ptr(value) => Err(syn::Error::new_spanned(
            value,
            "StablePayload fields must not be raw pointers",
        )),
        Type::BareFn(value) => Err(syn::Error::new_spanned(
            value,
            "StablePayload fields must not be function pointers",
        )),
        Type::Path(value) if path_last_segment_is(value, "String") => Err(syn::Error::new_spanned(
            value,
            "StablePayload fields must not own heap allocations",
        )),
        Type::Path(value)
            if ["Vec", "Box", "Arc"]
                .into_iter()
                .any(|ident| path_last_segment_is(value, ident)) =>
        {
            Err(syn::Error::new_spanned(
                value,
                "StablePayload fields must not own heap allocations",
            ))
        }
        Type::Array(value) => validate_stable_field_type(&value.elem),
        _ => Ok(()),
    }
}

fn path_last_segment_is(value: &syn::TypePath, ident: &str) -> bool {
    value
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == ident)
}
