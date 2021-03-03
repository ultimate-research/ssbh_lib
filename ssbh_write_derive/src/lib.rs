extern crate proc_macro;

use crate::proc_macro::TokenStream;

use quote::quote;

use syn::{
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, GenericArgument, Ident,
    PathArguments, Type,
};

#[proc_macro_derive(SsbhWrite)]
pub fn ssbh_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

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
    let field_types = fields.iter().map(|field| &field.ty);

    // Create the trait implementation.
    let expanded = quote! {
        impl crate::SsbhWrite for #implementing_type {
            fn write_ssbh<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                data_ptr: &mut u64,
            ) -> std::io::Result<()> {
                #(
                    self.#field_names.write_ssbh(writer, data_ptr)?;
                )*
                Ok(())
            }

            fn size_in_bytes(&self) -> u64 {
                let mut size = 0;
                #(
                    size += self.#field_names.size_in_bytes();
                )*
                size
            }

            fn alignment_in_bytes(&self) -> u64 {
                8
            }
        }
    };

    TokenStream::from(expanded)
}
