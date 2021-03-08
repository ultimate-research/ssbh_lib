extern crate proc_macro;

use crate::proc_macro::TokenStream;

use quote::quote;

use syn::{Attribute, Data, DataStruct, DeriveInput, Fields, parse_macro_input};

fn get_padding_size(attrs: &Vec<Attribute>) -> usize {
    for attr in attrs {
        if attr.path.is_ident("padding") {
            let lit: syn::LitInt = attr.parse_args().unwrap();
            return lit.base10_parse::<usize>().unwrap();
        }
    }

    0
}

#[proc_macro_derive(SsbhWrite, attributes(padding))]
pub fn ssbh_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let padding_size = get_padding_size(&input.attrs);

    // TODO: Support enums.
    let implementing_type = &input.ident;
    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let field_names: Vec<_> = fields.iter().map(|field| &field.ident).collect();

    // Create the trait implementation.
    let expanded = quote! {
        impl crate::SsbhWrite for #implementing_type {
            fn write_ssbh<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                data_ptr: &mut u64,
            ) -> std::io::Result<()> {
                // The data pointer must point past the containing struct.
                let current_pos = writer.seek(std::io::SeekFrom::Current(0))?;
                if *data_ptr <= current_pos {
                    *data_ptr = current_pos + self.size_in_bytes();
                }

                #(
                    self.#field_names.write_ssbh(writer, data_ptr)?;
                )*
                
                writer.write(&[0u8; #padding_size])?;
                Ok(())
            }

            fn size_in_bytes(&self) -> u64 {
                let mut size = 0;
                #(
                    size += self.#field_names.size_in_bytes();
                )*
                size += #padding_size as u64;
                size
            }

            fn alignment_in_bytes(&self) -> u64 {
                8
            }
        }
    };

    TokenStream::from(expanded)
}
