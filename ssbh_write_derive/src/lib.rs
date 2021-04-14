extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use syn::{
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Field, Fields, Generics, Ident,
};

fn parse_usize(attrs: &[Attribute], name: &str) -> Option<usize> {
    for attr in attrs {
        if attr.path.is_ident(name) {
            let lit: syn::LitInt = attr.parse_args().ok()?;
            return lit.base10_parse::<usize>().ok();
        }
    }

    None
}

#[proc_macro_derive(SsbhWrite, attributes(padding, align_after))]
pub fn ssbh_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let padding_size = parse_usize(&input.attrs, "padding").unwrap_or(0);
    let alignment_size = parse_usize(&input.attrs, "align_after").unwrap_or(0) as u64;

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
        padding_size,
        alignment_size,
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
    padding_size: usize,
    alignment_size: u64,
    field_names: &Vec<&Option<Ident>>,
    calculate_enum_size: &TokenStream2,
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // TODO: use writer.position?
    // TODO: Is there a nicer way to handle alignment?
    let alignment = quote! {
        let round_up = |value, n| ((value + n - 1) / n) * n;
        if #alignment_size > 0 {
            // TODO: Is seeking from the end always correct?
            let current_pos = writer.seek(std::io::SeekFrom::End(0))?;
            let aligned_pos = round_up(current_pos, #alignment_size);
            for _ in 0..(aligned_pos - current_pos) {
                writer.write_all(&[0u8])?;
            }
        }
    };

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

                // TODO: Is doesn't make sense to generate both.
                // Types can be either enums or structs but not both.
                #write_fields
                #write_enum

                writer.write_all(&[0u8; #padding_size])?;
                #alignment
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
    enum_data: &Vec<(&proc_macro2::Ident, Vec<&proc_macro2::Ident>)>,
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
