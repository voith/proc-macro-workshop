use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let input = parse_macro_input!(input as DeriveInput);
    let vis = &input.vis;
    let name = &input.ident;

    let data_struct = match &input.data {
        syn::Data::Struct(s) => s,
        _ => {
            return syn::Error::new_spanned(&input, "Expected a struct")
                .to_compile_error()
                .into()
        }
    };

    // Build a const expression that sums all field Specifier::BITS.
    let zero = 0usize;
    let mut total_bits = quote! { #zero };
    if let syn::Fields::Named(fields) = &data_struct.fields {
        for field in &fields.named {
            if let syn::Type::Path(tp) = &field.ty {
                let ty_path = &tp.path;
                // Rely on trait checking by referencing the associated const.
                // If the type does not implement bitfield::Specifier, this will
                // cause a compile error at the use site.
                total_bits = quote! {
                    #total_bits + <#ty_path as ::bitfield::Specifier>::BITS
                };
            }
        }
    } else {
        return syn::Error::new_spanned(&data_struct.fields, "#[bitfield] requires named fields")
            .to_compile_error()
            .into();
    }
    // Cannot evaluate to a concrete usize inside the macro because it depends on
    // associated consts. Keep it as tokens and evaluate in the generated code.
    let num_bytes = quote! { (#total_bits) / 8 };
    let rem_mod8 = quote! { ((#total_bits) % 8) };

    let expanded = quote! {
        #[repr(transparent)]
        #vis struct #name {
            data: [u8; #num_bytes]
        }
        const _: ::bitfield::checks::MultipleOfEight<[(); #rem_mod8]> = ();

        impl #name {
            fn new() -> Self {
                Self {
                    data: [0u8; #num_bytes]
                }
            }
        }
    };
    expanded.into()
}
