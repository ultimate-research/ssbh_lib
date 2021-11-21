extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, Generics, Ident, Index,
    LitByteStr, MetaNameValue,
};

#[derive(Default)]
struct WriteOptions {
    pad_after: Option<usize>,
    align_after: Option<usize>,
    alignment: Option<usize>,
    repr: Option<Ident>,
    magic: Option<LitByteStr>,
}

// TODO: This is misleading since it won't always be a TypeRepr.
struct TypeRepr {
    ident: kw::repr,
    value: Ident,
}

mod kw {
    syn::custom_keyword!(repr);
}

impl Parse for TypeRepr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident = input.parse()?;
        let content;
        parenthesized!(content in input);
        let value = content.parse()?;

        Ok(Self { ident, value })
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

    // darling doesn't support the repr(type) syntax, so do everything manually.
    // TODO: Clean this up.
    let mut write_options = WriteOptions::default();
    for attr in &input.attrs {
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
                            _ => (),
                        }
                    }
                }
            }
        }
    }

    // TODO: Is there a way to use a field with quote like struct.field?
    let pad_after = write_options.pad_after;
    let align_after = write_options.align_after;
    let alignment_in_bytes = write_options.alignment;
    let magic = write_options.magic.clone();

    let write_magic = if let Some(magic) = magic {
        quote! { #magic.ssbh_write(writer, data_ptr)?; }
    } else {
        quote! {}
    };

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
            }) => {
                let named_fields: Vec<_> = fields.named.iter().map(|field| &field.ident).collect();
                let write_fields = quote! {
                    #write_magic

                    #(
                        self.#named_fields.ssbh_write(writer, data_ptr)?;
                    )*
                };
                (
                    write_fields,
                    generate_size_calculation_named(&named_fields, pad_after, write_options.magic.clone()),
                )
            }
            Data::Struct(DataStruct {
                fields: Fields::Unnamed(fields),
                ..
            }) => {
                let unnamed_fields: Vec<_> =
                    (0..fields.unnamed.len()).map(syn::Index::from).collect();
                let write_fields = quote! {
                    #(
                        self.#unnamed_fields.ssbh_write(writer, data_ptr)?;
                    )*
                };
                (
                    write_fields,
                    generate_size_calculation_unnamed(&unnamed_fields, pad_after, write_options.magic.clone()),
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
        },
    };

    // Alignment can be user specified or determined by the type.
    let calculate_alignment = match alignment_in_bytes {
        Some(alignment) => quote! { #alignment as u64 },
        None => match &write_options.repr {
            Some(repr) => quote! { std::mem::align_of::<#repr>() as u64 },
            None => quote! { std::mem::align_of::<Self>() as u64 },
        },
    };

    let expanded = generate_ssbh_write(
        name,
        &generics,
        &write_data,
        &calculate_size,
        pad_after,
        align_after,
        &calculate_alignment,
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
    calculate_alignment: &TokenStream2,
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Skip generating code for unspecified parameters.
    let write_align_after = match align_after {
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

                #write_padding
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

fn generate_size_calculation_named(
    named_fields: &[&Option<Ident>],
    pad_after: Option<usize>,
    magic: Option<LitByteStr>
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
        #(
            size += self.#named_fields.size_in_bytes();
        )*
        #add_padding;
        #add_magic;
        size
    }
}

fn generate_size_calculation_unnamed(
    unnamed_fields: &[Index],
    pad_after: Option<usize>,
    magic: Option<LitByteStr>
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
        #(
            size += self.#unnamed_fields.size_in_bytes();
        )*
        #add_padding;
        #add_magic;
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
