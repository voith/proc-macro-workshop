use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

// Register the `builder` helper attribute so it is allowed on fields
#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;

    let data_struct = match &input.data {
        syn::Data::Struct(ds) => ds,
        _ => {
            return syn::Error::new_spanned(&ident, "#[derive(Builder)] only supports structs")
                .to_compile_error()
                .into();
        }
    };

    let mut builder_fields = Vec::new();
    let mut defaults = Vec::new();
    let mut setters = Vec::new();
    let mut build_inits = Vec::new();
    let mut each_setter = Vec::new();
    match &data_struct.fields {
        syn::Fields::Named(fields) => {
            for f in &fields.named {
                let name = &f.ident;
                let field_type = &f.ty;
                let mut same_name_as_each = false;
                let mut has_each = false;
                for attr in &f.attrs {
                    if attr.path().is_ident("builder") {
                        let parse_result = attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("each") {
                                let lit: syn::LitStr = meta.value()?.parse()?;
                                let each = syn::Ident::new(&lit.value(), lit.span());
                                let inner_ty =
                                    get_inner_type_from_vec(field_type).expect("Vec inner type");
                                has_each = true;
                                if let Some(n) = name {
                                    if n == lit.value().as_str() {
                                        same_name_as_each = true;
                                    }
                                }
                                each_setter.push(quote! {
                                    pub fn #each(&mut self, value: #inner_ty) -> &mut Self {
                                        if let Some(vec) = &mut self.#name {
                                            vec.push(value);
                                        } else {
                                            self.#name = Some(vec![value]);
                                        }
                                        self
                                    }
                                });
                                ::core::result::Result::Ok(())
                            } else {
                                let msg = "expected `builder(each = \"...\")`";
                                let error = syn::Error::new_spanned(&attr.meta, msg);
                                return ::core::result::Result::Err(error);
                            }
                        });
                        if let ::core::result::Result::Err(e) = parse_result {
                            return e.to_compile_error().into();
                        }
                    }
                }

                if is_option(field_type) {
                    builder_fields.push(quote! { #name: #field_type });
                    defaults.push(quote! { #name: None });
                    let inner_field_type = get_inner_type_from_option(field_type).unwrap();
                    setters.push(quote! {
                        pub fn #name(&mut self, value: #inner_field_type) -> &mut Self {
                            self.#name = Some(value);
                            self
                        }
                    });
                    build_inits.push(quote! {
                        #name: if self.#name.is_some() {
                            self.#name.take()
                        } else {
                            None
                        }
                    });
                } else {
                    builder_fields.push(quote! { #name: ::core::option::Option<#field_type> });
                    if has_each {
                        defaults.push(quote! { #name: Some(Vec::new()) });
                    } else {
                        defaults.push(quote! { #name: None });
                    }
                    if !same_name_as_each {
                        setters.push(quote! {
                            pub fn #name(&mut self, value: #field_type) -> &mut Self {
                                self.#name = Some(value);
                                self
                            }
                        });
                    }
                    let msg: &'static str = "missing field";
                    build_inits.push(quote! {
                        #name: if self.#name.is_some() {
                            self.#name.take().ok_or(#msg)?
                        } else {
                            return ::core::result::Result::Err(#msg)
                        }
                    });
                }
            }
        }
        _ => {
            return syn::Error::new_spanned(
                &ident,
                "#[derive(Builder)] supports only structs with named fields",
            )
            .to_compile_error()
            .into()
        }
    };

    let tokens = quote! {
        struct Builder {
            #(#builder_fields,)*
        }

        impl Builder {

            pub fn build(&mut self) -> ::core::result::Result<#ident, &'static str> {
                ::core::result::Result::Ok( #ident {
                    #(#build_inits, )*
                })
            }

            #(#setters)*

            #(#each_setter)*

        }

        impl #ident {
            pub fn builder() -> Builder {
                Builder {
                    #(#defaults,)*
                }
            }
        }
    };
    TokenStream::from(tokens)
}

fn is_option(ty: &syn::Type) -> bool {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Option";
        }
    }
    false
}

fn get_inner_type_from_option(ty: &syn::Type) -> ::core::option::Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        // Check the last path segment, e.g. "Option"
        if let Some(seg) = type_path.path.segments.last() {
            if seg.ident == "Option" {
                // Check for generic argument <T>
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

fn get_inner_type_from_vec(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}
