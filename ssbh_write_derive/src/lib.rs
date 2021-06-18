extern crate proc_macro;

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use syn::{
    parse_macro_input, Data, DataStruct, DeriveInput, Fields, Generics, Ident, Index,
};

#[derive(FromDeriveInput)]
#[darling(attributes(ssbhwrite))]
struct WriteOptions {
    #[darling(default)]
    pad_after: Option<usize>,
    #[darling(default)]
    align_after: Option<usize>,
    #[darling(default)]
    alignment: Option<usize>,
}

#[proc_macro_derive(SsbhWrite, attributes(ssbhwrite))]
pub fn ssbh_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let write_options: WriteOptions = FromDeriveInput::from_derive_input(&input).unwrap();

    let pad_after = write_options.pad_after;
    let align_after = write_options.align_after;

    // The alignment for most types will be 8 bytes.
    let alignment_in_bytes = write_options.alignment.unwrap_or(8);

    let name = &input.ident;
    let generics = input.generics;

    // TODO: Support tuples?
    let (write_data, calculate_size) = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let named_fields: Vec<_> = fields.named.iter().map(|field| &field.ident).collect();
            let write_fields = quote! {
                #(
                    self.#named_fields.ssbh_write(writer, data_ptr)?;
                )*
            };
            (
                write_fields,
                generate_size_calculation_named(&named_fields, pad_after),
            )
        }
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(fields),
            ..
        }) => {
            let unnamed_fields: Vec<_> = (0..fields.unnamed.len()).map(syn::Index::from).collect();
            let write_fields = quote! {
                #(
                    self.#unnamed_fields.ssbh_write(writer, data_ptr)?;
                )*
            };
            (
                write_fields,
                generate_size_calculation_unnamed(&unnamed_fields, pad_after),
            )
        }
        Data::Enum(data_enum) => {
            let enum_variants = get_enum_variants(data_enum);
            let write_variants: Vec<_> = enum_variants
                .iter()
                .map(|v| {
                    let name = v.0;
                    quote! {
                        Self::#name(v) => v.ssbh_write(writer, data_ptr)?
                    }
                })
                .collect();
            let write_variants = quote! {
                match self {
                    #(
                        #write_variants,
                    )*
                }
            };
            (
                write_variants,
                generate_size_calculation_enum(&enum_variants, pad_after),
            )
        }
        _ => panic!("Unsupported type"),
    };

    let expanded = generate_ssbh_write(
        &name,
        &generics,
        &write_data,
        &calculate_size,
        pad_after,
        align_after,
        alignment_in_bytes,
    );
    TokenStream::from(expanded)
}

fn get_enum_variants(data_enum: &syn::DataEnum) -> Vec<(&Ident, Vec<&Ident>)> {
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

fn generate_ssbh_write(
    name: &Ident,
    generics: &Generics,
    write_data: &TokenStream2,
    calculate_size: &TokenStream2,
    pad_after: Option<usize>,
    align_after: Option<usize>,
    alignment_in_bytes: usize,
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Skip generating code for unspecified parameters.
    let write_alignment = match align_after {
        Some(num_bytes) => quote! {
            // Check for divide by 0.
            if #num_bytes > 0 {
                let round_up = |value, n| ((value + n - 1) / n) * n;
                // TODO: Is seeking from the end always correct?
                let current_pos = writer.seek(std::io::SeekFrom::End(0))?;
                let aligned_pos = round_up(current_pos, #num_bytes as u64);
                for _ in 0..(aligned_pos - current_pos) {
                    writer.write_all(&[0u8])?;
                }
            }

        },
        None => quote! {},
    };

    let write_padding = match pad_after {
        Some(num_bytes) => quote! { writer.write_all(&[0u8; #num_bytes])?; },
        None => quote! {},
    };

    let expanded = quote! {
        impl #impl_generics crate::SsbhWrite for #name #ty_generics #where_clause {
            fn ssbh_write<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                data_ptr: &mut u64,
            ) -> std::io::Result<()> {
                // The data pointer must point past the containing struct.
                let current_pos = writer.stream_position()?;
                if *data_ptr < current_pos + self.size_in_bytes(){
                    *data_ptr = current_pos + self.size_in_bytes();
                }

                #write_data

                #write_padding
                #write_alignment

                Ok(())
            }

            fn size_in_bytes(&self) -> u64 {
                #calculate_size
            }

            fn alignment_in_bytes(&self) -> u64 {
                #alignment_in_bytes as u64
            }
        }
    };
    expanded
}

fn generate_size_calculation_named(
    named_fields: &[&Option<Ident>],
    pad_after: Option<usize>,
) -> TokenStream2 {
    let add_padding = match pad_after {
        Some(num_bytes) => quote! { size += #num_bytes as u64; },
        None => quote! {},
    };

    quote! {
        let mut size = 0;
        #(
            size += self.#named_fields.size_in_bytes();
        )*
        #add_padding;
        size
    }
}

fn generate_size_calculation_unnamed(
    unnamed_fields: &[Index],
    pad_after: Option<usize>,
) -> TokenStream2 {
    let add_padding = match pad_after {
        Some(num_bytes) => quote! { size += #num_bytes as u64; },
        None => quote! {},
    };

    quote! {
        let mut size = 0;
        #(
            size += self.#unnamed_fields.size_in_bytes();
        )*
        #add_padding;
        size
    }
}

fn generate_size_calculation_enum(
    enum_data: &[(&proc_macro2::Ident, Vec<&proc_macro2::Ident>)],
    pad_after: Option<usize>,
) -> TokenStream2 {
    let add_padding = match pad_after {
        Some(num_bytes) => quote! { size += #num_bytes as u64; },
        None => quote! {},
    };

    let get_variant_size: Vec<_> = enum_data
        .iter()
        .map(|v| {
            let name = v.0;
            quote! {
                Self::#name(v) => v.size_in_bytes()
            }
        })
        .collect();
    if enum_data.is_empty() {
        quote! { 0 }
    } else {
        quote! {
            let mut size = match self {
                #(
                    #get_variant_size,
                )*
            };
            #add_padding
            size
        }
    }
}