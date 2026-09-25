/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, LitInt, LitStr, Type};

#[proc_macro_derive(StablePayload, attributes(stable_payload))]
pub fn derive_stable_payload(input: TokenStream) -> TokenStream {
    derive_stable_payload_impl(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(StablePayloadInit)]
pub fn derive_stable_payload_init(input: TokenStream) -> TokenStream {
    derive_stable_payload_init_impl(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn derive_stable_payload_init_impl(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "StablePayloadInit does not support generic payload structs",
        ));
    }
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "StablePayloadInit can only be derived for structs",
            ));
        }
    };
    if fields.is_empty() {
        return Err(syn::Error::new_spanned(
            name,
            "StablePayloadInit requires at least one field",
        ));
    }

    let builder = format_ident!("__UpRustStablePayloadInitFor{name}");
    let states: Vec<_> = (0..fields.len())
        .map(|index| format_ident!("__UpRustField{index}"))
        .collect();
    let unset = quote!(::up_rust::payload::stable::StablePayloadInitUnset);
    let set = quote!(::up_rust::payload::stable::StablePayloadInitSet);
    let initial_states = states.iter().map(|_| unset.clone());
    let final_states = states.iter().map(|_| set.clone());

    let mut setters = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let method = field
            .ident
            .clone()
            .unwrap_or_else(|| format_ident!("field_{index}"));
        let member = field
            .ident
            .as_ref()
            .map(|ident| quote!(#ident))
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(#index)
            });
        let ty = &field.ty;
        let impl_states: Vec<_> = states
            .iter()
            .enumerate()
            .map(|(state_index, state)| {
                if state_index == index {
                    unset.clone()
                } else {
                    quote!(#state)
                }
            })
            .collect();
        let output_states: Vec<_> = states
            .iter()
            .enumerate()
            .map(|(state_index, state)| {
                if state_index == index {
                    set.clone()
                } else {
                    quote!(#state)
                }
            })
            .collect();
        let other_states: Vec<_> = states
            .iter()
            .enumerate()
            .filter_map(|(state_index, state)| (state_index != index).then_some(state))
            .collect();

        let body = if let Type::Array(array) = ty {
            let elem = &array.elem;
            let len = &array.len;
            let from_array = format_ident!("{method}_from_array");
            let from_slice = format_ident!("{method}_from_slice");
            let fill = format_ident!("{method}_fill");
            let fill_with = format_ident!("{method}_fill_with");
            quote! {
                pub fn #from_array(
                    self,
                    values: &[#elem; #len],
                ) -> #builder<'__up_rust_payload, #(#output_states),*>
                where
                    #elem: Clone,
                {
                    unsafe {
                        ::up_rust::payload::stable::write_stable_payload_array::<#name, #elem, #len, _>(
                            self.payload,
                            ::core::mem::offset_of!(#name, #member),
                            |index| values.get(index).expect("array initializer index").clone(),
                        );
                    }
                    #builder { payload: self.payload, marker: ::core::marker::PhantomData }
                }

                pub fn #from_slice(
                    self,
                    values: &[#elem],
                ) -> Result<#builder<'__up_rust_payload, #(#output_states),*>, ::up_rust::UWireError>
                where
                    #elem: Clone,
                {
                    if values.len() != #len {
                        return Err(::up_rust::UWireError::invalid_payload_length(#len, values.len()));
                    }
                    Ok(self.#from_array(values.try_into().expect("checked array length")))
                }

                pub fn #fill(
                    self,
                    value: #elem,
                ) -> #builder<'__up_rust_payload, #(#output_states),*>
                where
                    #elem: Clone,
                {
                    self.#fill_with(|_| value.clone())
                }

                pub fn #fill_with<F>(
                    self,
                    element: F,
                ) -> #builder<'__up_rust_payload, #(#output_states),*>
                where
                    F: FnMut(usize) -> #elem,
                {
                    unsafe {
                        ::up_rust::payload::stable::write_stable_payload_array::<#name, #elem, #len, F>(
                            self.payload,
                            ::core::mem::offset_of!(#name, #member),
                            element,
                        );
                    }
                    #builder { payload: self.payload, marker: ::core::marker::PhantomData }
                }
            }
        } else {
            quote! {
                pub fn #method(
                    self,
                    value: #ty,
                ) -> #builder<'__up_rust_payload, #(#output_states),*> {
                    unsafe {
                        ::up_rust::payload::stable::write_stable_payload_field::<#name, #ty>(
                            self.payload,
                            ::core::mem::offset_of!(#name, #member),
                            value,
                        );
                    }
                    #builder { payload: self.payload, marker: ::core::marker::PhantomData }
                }
            }
        };
        setters.push(quote! {
            impl<'__up_rust_payload, #(#other_states),*>
                #builder<'__up_rust_payload, #(#impl_states),*>
            {
                #body
            }
        });
    }

    Ok(quote! {
        #[doc(hidden)]
        pub struct #builder<'__up_rust_payload, #(#states),*> {
            payload: *mut #name,
            marker: ::core::marker::PhantomData<(&'__up_rust_payload mut #name, #(#states),*)>,
        }

        #(#setters)*

        impl<'__up_rust_payload>
            #builder<'__up_rust_payload, #(#final_states),*>
        {
            pub fn finish(self) -> ::up_rust::payload::stable::InitializedStablePayload<'__up_rust_payload, #name> {
                unsafe { ::up_rust::payload::stable::finish_stable_payload_init(self.payload) }
            }
        }

        impl ::up_rust::payload::stable::StablePayloadInit for #name {
            type Initializer<'__up_rust_payload> =
                #builder<'__up_rust_payload, #(#initial_states),*>;

            fn init(
                storage: &mut [::core::mem::MaybeUninit<u8>],
            ) -> Result<Self::Initializer<'_>, ::up_rust::UWireError> {
                let payload = ::up_rust::payload::stable::prepare_stable_payload_storage::<Self>(storage)?;
                Ok(#builder { payload, marker: ::core::marker::PhantomData })
            }
        }
    })
}

fn derive_stable_payload_impl(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let mut has_stable_repr = false;
    for attr in &input.attrs {
        if attr.path().is_ident("repr") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("C") || meta.path.is_ident("transparent") {
                    has_stable_repr = true;
                } else if meta.path.is_ident("packed") {
                    return Err(meta.error("StablePayload does not support packed fields"));
                } else if meta.path.is_ident("align") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let _: LitInt = content.parse()?;
                }
                Ok(())
            })?;
        }
    }
    if !has_stable_repr {
        return Err(syn::Error::new_spanned(
            name,
            "StablePayload requires #[repr(C)] or #[repr(transparent)]",
        ));
    }

    let mut type_name = None::<LitStr>;
    let mut variant = None::<LitInt>;
    for attr in &input.attrs {
        if !attr.path().is_ident("stable_payload") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("type_name") {
                type_name = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("variant") {
                variant = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("payload_encoding_id") {
                return Err(meta.error("encoding IDs belong to NativeProfile, not a payload type"));
            } else {
                return Err(meta.error("unsupported stable_payload option"));
            }
            Ok(())
        })?;
    }
    let type_name = type_name.ok_or_else(|| {
        syn::Error::new_spanned(
            name,
            "StablePayload requires #[stable_payload(type_name = \"...\")]",
        )
    })?;
    let variant = variant
        .map(|value| quote!(#value))
        .unwrap_or_else(|| quote!(0_u32));

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "StablePayload can only be derived for structs",
            ));
        }
    };

    let mut generics = input.generics.clone();
    let mut validators = Vec::new();
    let mut always_valid = Vec::new();
    let mut representations = Vec::new();
    let mut field_sizes = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let ty = &field.ty;
        field_sizes.push(quote!(::core::mem::size_of::<#ty>()));
        generics
            .make_where_clause()
            .predicates
            .push(syn::parse_quote!(#ty: ::up_rust::payload::stable::StablePayloadField));
        always_valid.push(quote!(
            <#ty as ::up_rust::payload::stable::StablePayloadField>::FIELD_BITS_ALWAYS_VALID
        ));
        let member = field
            .ident
            .as_ref()
            .map(|ident| quote!(#ident))
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(#index)
            });
        let local = format_ident!("field_bytes_{index}");
        let field_name = field
            .ident
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| index.to_string());
        representations.push(quote! {
            ::up_rust::NativeField::new(
                #field_name, ::core::mem::offset_of!(Self, #member) as u64,
                <#ty as ::up_rust::payload::stable::StablePayloadField>::field_representation(),
            )
        });
        validators.push(quote! {
            let field_start = ::core::mem::offset_of!(Self, #member);
            let Some(field_end) = field_start.checked_add(::core::mem::size_of::<#ty>()) else {
                return false;
            };
            let Some(#local) = bytes.get(field_start..field_end) else {
                return false;
            };
            if !<#ty as ::up_rust::payload::stable::StablePayloadField>::validate_field_bytes(#local) {
                return false;
            }
        });
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let always_valid_expr = if always_valid.is_empty() {
        quote!(true)
    } else {
        quote!(true #(&& #always_valid)*)
    };

    let _ = fields;
    let layout_assert = if input.generics.params.is_empty() {
        quote! {
            const _: () = {
                let _ = <#name as ::up_rust::payload::stable::StablePayloadField>::FIELD_BITS_ALWAYS_VALID;
            };
        }
    } else {
        quote!()
    };
    Ok(quote! {
        #layout_assert
        unsafe impl #impl_generics ::up_rust::payload::stable::StablePayloadField
            for #name #ty_generics #where_clause
        {
            const FIELD_BITS_ALWAYS_VALID: bool = {
                assert!(!::core::mem::needs_drop::<Self>(), "StablePayload cannot have drop glue");
                assert!(::core::mem::size_of::<Self>() == (0_usize #(+ #field_sizes)*),
                    "StablePayload requires explicit padding fields");
                #always_valid_expr
            };

            fn field_representation() -> ::up_rust::NativeRepresentation {
                let _ = Self::FIELD_BITS_ALWAYS_VALID;
                ::up_rust::NativeRepresentation::new(
                    #type_name, #variant,
                    ::core::mem::size_of::<Self>() as u64,
                    ::core::mem::align_of::<Self>() as u64,
                    ::up_rust::NativeByteOrder::native(),
                    vec![#(#representations),*],
                ).expect("compiler-proven stable representation")
            }

            fn validate_field_bytes(bytes: &[u8]) -> bool {
                let _ = Self::FIELD_BITS_ALWAYS_VALID;
                if bytes.len() != ::core::mem::size_of::<Self>() {
                    return false;
                }
                #(#validators)*
                true
            }
        }

        unsafe impl #impl_generics ::up_rust::payload::stable::StablePayload
            for #name #ty_generics #where_clause
        {
            const TYPE_NAME: &'static str = #type_name;
            const VARIANT: u32 = #variant;
        }
    })
}
