extern crate proc_macro;

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields, Generics, Ident};

// TODO: How to also support enums?
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

    let write_enum = generate_write_enum(&enum_data);
    let calculate_enum_size = generate_calculate_enum_size(enum_data);

    let expanded = generate_write_ssbh(
        &name,
        &generics,
        &write_fields,
        &write_enum,
        pad_after,
        align_after,
        alignment_in_bytes,
        &field_names,
        &calculate_enum_size,
    );
    TokenStream::from(expanded)
}

fn generate_write_ssbh(
    name: &Ident,
    generics: &Generics,
    write_fields: &TokenStream2,
    write_enum: &TokenStream2,
    pad_after: Option<usize>,
    align_after: Option<usize>,
    alignment_in_bytes: usize,
    field_names: &[&Option<Ident>],
    calculate_enum_size: &TokenStream2,
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

    let add_padding = match pad_after {
        Some(num_bytes) => quote! { size += #num_bytes as u64; },
        None => quote! {},
    };

    let calculate_size = quote! {
        let mut size = 0;
        #(
            size += self.#field_names.size_in_bytes();
        )*
        #add_padding;
        // TODO: Having this default to 0 for structs is confusing.
        size += #calculate_enum_size;
        size
    };

    let expanded = quote! {
        impl #impl_generics crate::SsbhWrite for #name #ty_generics #where_clause {
            fn write_ssbh<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                data_ptr: &mut u64,
            ) -> std::io::Result<()> {
                // The data pointer must point past the containing struct.
                let current_pos = writer.stream_position()?;
                if *data_ptr < current_pos + self.size_in_bytes(){
                    *data_ptr = current_pos + self.size_in_bytes();
                }

                // TODO: Is doesn't make sense to generate both.
                // Types can be either enums or structs but not both.
                #write_fields
                #write_enum

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

fn generate_calculate_enum_size(
    enum_data: Vec<(&proc_macro2::Ident, Vec<&proc_macro2::Ident>)>,
) -> TokenStream2 {
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
            match self {
                #(
                    #get_variant_size,
                )*
            }
        }
    }
}

fn generate_write_enum(
    enum_data: &[(&proc_macro2::Ident, Vec<&proc_macro2::Ident>)],
) -> TokenStream2 {
    let write_variants: Vec<_> = enum_data
        .iter()
        .map(|v| {
            let name = v.0;
            quote! {
                Self::#name(v) => v.write_ssbh(writer, data_ptr)?
            }
        })
        .collect();
    if enum_data.is_empty() {
        quote! {}
    } else {
        quote! {
            match self {
                #(
                    #write_variants,
                )*
            }
        }
    }
}
