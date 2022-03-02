extern crate proc_macro;

use core::panic;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, FieldsNamed,
    FieldsUnnamed, Generics, Ident, LitByteStr, MetaNameValue,
};

#[derive(Default)]
struct WriteOptions {
    pad_after: Option<usize>,
    align_after: Option<usize>,
    alignment: Option<usize>,
    repr: Option<Ident>,
    magic: Option<LitByteStr>,
}

struct TypeRepr {
    value: Ident,
}

mod kw {
    syn::custom_keyword!(repr);
}

impl Parse for TypeRepr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let _ident: kw::repr = input.parse()?;
        let content;
        parenthesized!(content in input);
        let value = content.parse()?;

        Ok(Self { value })
    }
}

fn get_repr(attr: &Attribute) -> Option<Ident> {
    match attr.parse_args::<TypeRepr>() {
        Ok(type_repr) => Some(type_repr.value),
        Err(_) => None,
    }
}

fn get_usize_arg(m: &MetaNameValue) -> Option<usize> {
    if let syn::Lit::Int(value) = &m.lit {
        Some(value.base10_parse().unwrap())
    } else {
        None
    }
}

fn get_byte_string_arg(m: &MetaNameValue) -> Option<LitByteStr> {
    if let syn::Lit::ByteStr(value) = &m.lit {
        Some(value.clone())
    } else {
        None
    }
}

#[proc_macro_derive(SsbhWrite, attributes(ssbhwrite))]
pub fn ssbh_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // TODO: Clean this up.
    let write_options = get_write_options(&input.attrs);

    let name = &input.ident;
    let generics = input.generics;

    // TODO: Support tuples?
    // Specifying a repr type overrides most of the generated code.
    // TODO: This is kind of messy.
    // TODO: The repr doesn't really make sense for structs.
    // TODO: This only makes sense for primitive types?
    let (write_data, calculate_size) = match &write_options.repr {
        Some(repr) => (
            quote! {
                (*self as #repr).ssbh_write(writer, data_ptr)?;
            },
            quote! {
                (*self as #repr).size_in_bytes()
            },
        ),
        None => match &input.data {
            Data::Struct(DataStruct {
                fields: Fields::Named(fields),
                ..
            }) => write_data_calculate_size_named(fields, &write_options),
            Data::Struct(DataStruct {
                fields: Fields::Unnamed(fields),
                ..
            }) => write_data_calculate_size_unnamed(fields, &write_options),
            Data::Enum(data_enum) => write_data_calculate_size_enum(data_enum, &write_options),
            _ => panic!("Unsupported type"),
        },
    };

    let expanded = generate_ssbh_write(
        name,
        &generics,
        &write_data,
        &calculate_size,
        &write_options,
    );
    TokenStream::from(expanded)
}

fn write_data_calculate_size_enum(
    data_enum: &syn::DataEnum,
    write_options: &WriteOptions,
) -> (TokenStream2, TokenStream2) {
    let write_variants: Vec<_> = data_enum
        .variants
        .iter()
        .map(|variant| {
            let name = &variant.ident;

            // TODO: Which options should be allowed at the variant level?
            let variant_options = get_write_options(&variant.attrs);
            let write_pad_after = write_pad_after(&variant_options);
            let write_align_after = write_aligned_after(&variant_options);

            match &variant.fields {
                Fields::Unnamed(_fields) => {
                    // TODO: Support multiple unnamed fields.
                    quote! {
                        Self::#name(v) => {
                            v.ssbh_write(writer, data_ptr)?;
                            #write_pad_after
                            #write_align_after
                        }
                    }
                }
                Fields::Named(fields) => {
                    let field_names = field_names(fields);
                    let write_fields = write_named_fields(fields, false);
                    quote! {
                        Self::#name { #(#field_names),* } => {
                            #(#write_fields)*
                            #write_pad_after
                            #write_align_after
                        }
                    }
                }
                Fields::Unit => panic!("expected an enum with fields"),
            }
        })
        .collect();
    let write_variants = quote! {
        match self {
            #(
                #write_variants
            ),*
        }
    };

    let add_variants: Vec<_> = data_enum
        .variants
        .iter()
        .map(|variant| {
            let name = &variant.ident;

            match &variant.fields {
                Fields::Unnamed(_fields) => {
                    // TODO: Support multiple unnamed fields.
                    quote! {
                        Self::#name(v) => v.size_in_bytes()
                    }
                }
                Fields::Named(fields) => {
                    // TODO: Reuse code for structs?
                    let field_names = field_names(fields);
                    quote! {
                        Self::#name { #(#field_names),* } => { #(#field_names.size_in_bytes())+* }
                    }
                }
                Fields::Unit => panic!("expected an enum with fields"),
            }
        })
        .collect();

    let add_variants = quote! {
        size += match self {
            #(
                #add_variants
            ),*
        };
    };

    (
        write_variants,
        generate_size_calculation(
            add_variants,
            write_options.pad_after,
            write_options.magic.clone(),
        ),
    )
}

fn write_data_calculate_size_unnamed(
    fields: &syn::FieldsUnnamed,
    write_options: &WriteOptions,
) -> (TokenStream2, TokenStream2) {
    let unnamed_fields: Vec<_> = (0..fields.unnamed.len()).map(syn::Index::from).collect();
    let write_fields = write_unnamed_fields(fields);
    (
        write_fields,
        generate_size_calculation(
            quote! {#(
                size += self.#unnamed_fields.size_in_bytes();
            )*},
            write_options.pad_after,
            write_options.magic.clone(),
        ),
    )
}

fn write_data_calculate_size_named(
    fields: &syn::FieldsNamed,
    write_options: &WriteOptions,
) -> (TokenStream2, TokenStream2) {
    let named_fields: Vec<_> = fields.named.iter().map(|field| &field.ident).collect();

    let write_fields = write_named_fields(fields, true);

    // TODO: This is shared with enums, unnamed fields, etc?
    let write_magic = if let Some(magic) = &write_options.magic {
        quote! { #magic.ssbh_write(writer, data_ptr)?; }
    } else {
        quote! {}
    };

    let write_fields = quote! {
        #write_magic
        #(#write_fields)*;
    };

    (
        write_fields,
        generate_size_calculation(
            quote! {#(
                size += self.#named_fields.size_in_bytes();
            )*},
            write_options.pad_after,
            write_options.magic.clone(),
        ),
    )
}

fn get_write_options(attrs: &[Attribute]) -> WriteOptions {
    let mut write_options = WriteOptions::default();

    for attr in attrs {
        if attr.path.is_ident("ssbhwrite") {
            if let Some(repr) = get_repr(attr) {
                // This uses a different syntax than named values.
                // ex: #[ssbhwrite(repr(u32)]
                write_options.repr = Some(repr);
            } else if let Ok(syn::Meta::List(l)) = attr.parse_meta() {
                for nested in l.nested {
                    // ex: #[ssbhwrite(pad_after = 16, align_after = 8)]
                    if let syn::NestedMeta::Meta(syn::Meta::NameValue(v)) = nested {
                        match v.path.get_ident().unwrap().to_string().as_str() {
                            "pad_after" => write_options.pad_after = get_usize_arg(&v),
                            "align_after" => write_options.align_after = get_usize_arg(&v),
                            "alignment" => write_options.alignment = get_usize_arg(&v),
                            "magic" => write_options.magic = get_byte_string_arg(&v),
                            _ => panic!("Unrecognized attribute"),
                        }
                    }
                }
            }
        }
    }
    write_options
}

fn generate_ssbh_write(
    name: &Ident,
    generics: &Generics,
    write_data: &TokenStream2,
    calculate_size: &TokenStream2,
    write_options: &WriteOptions,
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Skip generating code for unspecified parameters.
    let write_align_after = write_aligned_after(write_options);

    let write_pad_after = write_pad_after(write_options);

    // Alignment can be user specified or determined by the type.
    let calculate_alignment = match write_options.alignment {
        Some(alignment) => quote! { #alignment as u64 },
        None => match &write_options.repr {
            Some(repr) => quote! { std::mem::align_of::<#repr>() as u64 },
            None => quote! { std::mem::align_of::<Self>() as u64 },
        },
    };

    let expanded = quote! {
        impl #impl_generics ssbh_write::SsbhWrite for #name #ty_generics #where_clause {
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

                #write_pad_after
                #write_align_after

                Ok(())
            }

            fn size_in_bytes(&self) -> u64 {
                #calculate_size
            }

            fn alignment_in_bytes() -> u64 {
                #calculate_alignment
            }
        }
    };
    expanded
}

fn write_pad_after(write_options: &WriteOptions) -> TokenStream2 {
    match write_options.pad_after {
        Some(num_bytes) => quote! { writer.write_all(&[0u8; #num_bytes])?; },
        None => quote! {},
    }
}

fn write_aligned_after(write_options: &WriteOptions) -> TokenStream2 {
    let write_align_after = match write_options.align_after {
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
    write_align_after
}

fn generate_size_calculation(
    add_fields: TokenStream2,
    pad_after: Option<usize>,
    magic: Option<LitByteStr>,
) -> TokenStream2 {
    let add_padding = match pad_after {
        Some(num_bytes) => quote! { size += #num_bytes as u64; },
        None => quote! {},
    };

    let add_magic = match magic {
        Some(magic) => quote! {
            size += #magic.len() as u64;
        },
        None => quote! {},
    };

    quote! {
        let mut size = 0;
        #add_fields;
        #add_padding;
        #add_magic;
        size
    }
}

fn field_names(fields: &FieldsNamed) -> Vec<Ident> {
    fields
        .named
        .iter()
        .filter_map(|f| f.ident.clone())
        .collect()
}

fn write_named_fields(fields: &FieldsNamed, include_self: bool) -> Vec<TokenStream2> {
    fields
        .named
        .iter()
        .map(|field| {
            let name = &field.ident;
            let field_options = get_write_options(&field.attrs);
            let write_pad_after = write_pad_after(&field_options);
            let write_align_after = write_aligned_after(&field_options);

            if include_self {
                quote! {
                    self.#name.ssbh_write(writer, data_ptr)?;
                    #write_pad_after
                    #write_align_after
                }
            } else {
                quote! {
                    #name.ssbh_write(writer, data_ptr)?;
                    #write_pad_after
                    #write_align_after
                }
            }
        })
        .collect()
}

fn write_unnamed_fields(fields: &FieldsUnnamed) -> TokenStream2 {
    let fields: Vec<_> = (0..fields.unnamed.len()).map(syn::Index::from).collect();
    quote! {
        #(
            self.#fields.ssbh_write(writer, data_ptr)?;
        )*
    }
}
