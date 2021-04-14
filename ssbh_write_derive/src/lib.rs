extern crate proc_macro;

use crate::proc_macro::TokenStream;

use quote::quote;

use syn::{parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Field, Fields};

fn get_padding_size(attrs: &[Attribute]) -> usize {
    for attr in attrs {
        if attr.path.is_ident("padding") {
            let lit: syn::LitInt = attr.parse_args().unwrap();
            return lit.base10_parse::<usize>().unwrap();
        }
    }

    0
}

fn get_alignment(attrs: &[Attribute]) -> u64 {
    for attr in attrs {
        if attr.path.is_ident("align_after") {
            let lit: syn::LitInt = attr.parse_args().unwrap();
            return lit.base10_parse::<u64>().unwrap();
        }
    }

    0
}

#[proc_macro_derive(SsbhWrite, attributes(padding, align_after))]
pub fn ssbh_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let padding_size = get_padding_size(&input.attrs);
    let alignment_size = get_alignment(&input.attrs);

    let name = &input.ident;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // TODO: Support tuples.
    let fields: Vec<&Field> = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => fields.named.iter().collect(),
        _ => Vec::new(),
    };
    let field_names: Vec<_> = fields.iter().map(|field| &field.ident).collect();

    let write_fields = quote! {
        #(
            self.#field_names.write_ssbh(writer, data_ptr)?;
        )*
    };

    // TODO: This is probably not a good way to handle enums.
    let enum_data = match &input.data {
        Data::Enum(data_enum) => {
            let enum_variants: Vec<_> = data_enum
                .variants
                .iter()
                .map(|v| {
                    let fields: Vec<_> = match &v.fields {
                        Fields::Unnamed(unnamed_fields) => unnamed_fields,
                        _ => panic!("expected an enum with unnamed fields"),
                    }
                    .unnamed
                    .iter()
                    .filter_map(|f| f.ident.as_ref())
                    .collect();
                    (&v.ident, fields)
                })
                .collect();
            enum_variants
        }
        _ => Vec::new(),
    };

    // TODO: Don't assume a single field for each variant.
    let write_variants: Vec<_> = enum_data
        .iter()
        .map(|v| {
            let name = v.0;
            quote! {
                Self::#name(v) => v.write_ssbh(writer, data_ptr)?
            }
        })
        .collect();

    // Most types won't be enums, so just generate empty code if there are no variants.
    let write_enum = if enum_data.is_empty() {
        quote! {}
    } else {
        quote! {
            match self {
                #(
                    #write_variants,
                )*
            }
        }
    };

    // TODO: Find a way to clean this up.
    let get_variant_size: Vec<_> = enum_data
        .iter()
        .map(|v| {
            let name = v.0;
            quote! {
                Self::#name(v) => v.size_in_bytes()
            }
        })
        .collect();

    let calculate_enum_size = if enum_data.is_empty() {
        quote! { 0 }
    } else {
        quote! {
            match self {
                #(
                    #get_variant_size,
                )*
            }
        }
    };

    // Create the trait implementation.
    let expanded = quote! {
        impl #impl_generics crate::SsbhWrite for #name #ty_generics #where_clause {
            fn write_ssbh<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                data_ptr: &mut u64,
            ) -> std::io::Result<()> {
                // The data pointer must point past the containing struct.
                let current_pos = writer.seek(std::io::SeekFrom::Current(0))?;
                if *data_ptr < current_pos + self.size_in_bytes(){
                    *data_ptr = current_pos + self.size_in_bytes();
                }

                #write_fields
                #write_enum

                writer.write_all(&[0u8; #padding_size])?;

                // TODO: Is there a nicer way to handle alignment.
                let round_up = |value, n| ((value + n - 1) / n) * n;
                if #alignment_size > 0 {
                    // TODO: Is seeking from the end always correct?
                    let current_pos = writer.seek(std::io::SeekFrom::End(0))?;
                    let aligned_pos = round_up(current_pos, #alignment_size);
                    for _ in 0..(aligned_pos - current_pos) {
                        writer.write_all(&[0u8])?;
                    }
                }
                Ok(())
            }

            fn size_in_bytes(&self) -> u64 {
                let mut size = 0;
                #(
                    size += self.#field_names.size_in_bytes();
                )*
                size += #padding_size as u64;
                size += #calculate_enum_size;
                size
            }

            fn alignment_in_bytes(&self) -> u64 {
                8
            }
        }
    };

    TokenStream::from(expanded)
}
