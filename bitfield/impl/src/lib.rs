use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Ident, ItemStruct};

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let input = parse_macro_input!(input as ItemStruct);
    let vis = &input.vis;
    let name = &input.ident;

    // Build a const expression that sums all field Specifier::BITS.
    let zero = 0usize;
    let mut total_bits = quote! { #zero };
    let mut getters_setters = Vec::new();
    let mut bit_offset = quote! { #zero };
    if let syn::Fields::Named(fields) = &input.fields {
        for field in &fields.named {
            if let syn::Type::Path(tp) = &field.ty {
                let ty_path = &tp.path;
                // Rely on trait checking by referencing the associated const.
                // If the type does not implement bitfield::Specifier, this will
                // cause a compile error at the use site.
                let field_bits = quote! {<#ty_path as ::bitfield::Specifier>::BITS};
                total_bits = quote! {
                    #total_bits + #field_bits
                };
                let field_type = quote! {
                    <#ty_path as ::bitfield::Specifier>::Ty
                };
                let ident = field.ident.as_ref().unwrap();
                let getter = Ident::new(&format!("get_{}", ident), Span::call_site());
                let setter = Ident::new(&format!("set_{}", ident), Span::call_site());

                getters_setters.push(quote! {
                    fn #getter(&self) -> #field_type {
                        let mut bits_placed: usize = #bit_offset as usize; // total bits placed so far
                        let mut remaining: u8 = #field_bits as u8;
                        let mut value: u64 = 0b0;
                        let mut previous_bits: u64 = 0; // data bits already collected 
                        while remaining > 0 {
                            let byte = bits_placed / 8;
                            let start = (bits_placed % 8) as u8;
                            let vacancy = 8 - start;
                            let take = remaining.min(vacancy);
                            let end = start + take - 1;

                            value |= ((((self.data[byte] & create_get_bit_mask(start, end)) as u64) >> start)<< previous_bits) as u64;
                            previous_bits += (end - start + 1) as u64;

                            bits_placed += take as usize;
                            remaining -= take;
                        }
                        value as #field_type
                    }

                     fn #setter(&mut self, value: #field_type) {
                        let value = value as u64;
                        let mut bits_placed: usize = #bit_offset as usize; // total bits placed so far
                        let mut remaining: u8 = #field_bits as u8;
                        let mut num_bits_set: u64 = 0;
                        while remaining > 0 {
                            let byte = bits_placed / 8;
                            let start = (bits_placed % 8) as u8;
                            let vacancy = 8 - start;
                            let take = remaining.min(vacancy);
                            let end = start + take - 1;

                            self.data[byte] &= !create_get_bit_mask(start, end);
                            let slot_bits = ((value >> num_bits_set) & create_set_width_bit_mask(start, end)) << start;
                            self.data[byte] |= slot_bits as u8;
                            num_bits_set += (end - start + 1) as u64;

                            bits_placed += take as usize;
                            remaining -= take;
                        }
                    }
                });
                bit_offset = quote! { #bit_offset + #field_bits};
            }
        }
    } else {
        return syn::Error::new_spanned(&input.fields, "#[bitfield] requires named fields")
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

            #(#getters_setters)*
        }
    };
    expanded.into()
}
