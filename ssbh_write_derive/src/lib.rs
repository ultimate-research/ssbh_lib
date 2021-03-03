extern crate proc_macro;

use crate::proc_macro::TokenStream;

use quote::quote;


use syn::{parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, Ident};

#[proc_macro_derive(SsbhWrite)]
pub fn ssbh_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let implementing_type = &input.ident;
    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let field_name = fields.iter().map(|field| &field.ident);
    let field_type = fields.iter().map(|field| &field.ty);

    // Create the trait implementation.
    let expanded = quote! {
        impl crate::SsbhWrite for #implementing_type {
            fn write_ssbh<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                data_ptr: &mut u64,
            ) -> std::io::Result<()> {
                #(
                    self.#field_name.write_ssbh(writer, data_ptr)?;
                )*
                Ok(())
            }

            // TODO: Compute this at compile time?
            fn size_in_bytes() -> u64 {
                let mut size = 0;
                #(
                    size += #field_type::size_in_bytes();
                )*
                size
            }

            fn alignment_in_bytes() -> u64 {
                8
            }
        }
    };

    TokenStream::from(expanded)
}
