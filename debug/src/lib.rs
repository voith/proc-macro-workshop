use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{
    parse_macro_input, parse_quote, Attribute, DeriveInput, GenericArgument, PathArguments, Type,
    TypePath, WherePredicate,
};

// fn main() {
//     let ty: syn::Type = syn::parse_str("Option<Vec<String>>").unwrap();
//     println!("{:#?}", ty);
// }

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let data_struct = match &input.data {
        syn::Data::Struct(ds) => ds,
        _ => {
            return syn::Error::new_spanned(&input, "#[derive(CustomDebug)] only supports structs")
                .to_compile_error()
                .into();
        }
    };

    // Struct-level escape hatch: #[debug(bound = "...")]
    let (override_bounds, _struct_fmt_unused) = extract_debug_attrs(&input.attrs);
    let has_struct_level_override = !override_bounds.is_empty();

    // Determine which generic type parameters are used in non-PhantomData fields
    let mut used_type_params: HashSet<syn::Ident> = HashSet::new();
    // Collect associated type projections like T::Assoc that require a where bound `T::Assoc: Debug`
    let mut assoc_debug_bounds: Vec<Type> = Vec::new();
    // Collect field-level override bounds
    let mut field_level_overrides: Vec<WherePredicate> = Vec::new();

    // Set of generic type parameter idents for quick lookup
    let generic_idents: HashSet<syn::Ident> = input
        .generics
        .type_params()
        .map(|tp| tp.ident.clone())
        .collect();
    for field in &data_struct.fields {
        // Field-level escape hatch: #[debug(bound = "...")]
        let (field_overrides, _fmt_ignored_here) = extract_debug_attrs(&field.attrs);
        let has_field_override = !field_overrides.is_empty();
        field_level_overrides.extend(field_overrides);

        if has_struct_level_override {
            // Skip all inference when struct-level override is present
            continue;
        }

        if has_field_override {
            // Skip inference for this field; we already added its manual bounds
            continue;
        }

        collect_constraints_excluding_phantom(
            &field.ty,
            &generic_idents,
            &mut used_type_params,
            &mut assoc_debug_bounds,
        );
    }

    // Build generics / where clause
    let mut generics = input.generics.clone();
    if !has_struct_level_override {
        // Add Debug bounds only for used type parameters
        for type_param in generics.type_params_mut() {
            if used_type_params.contains(&type_param.ident) {
                type_param.bounds.push(parse_quote!(::core::fmt::Debug));
            }
        }
    }
    // Add where predicates for associated type projections and any overrides
    {
        let where_clause = generics.make_where_clause();

        if has_struct_level_override {
            for pred in override_bounds {
                where_clause.predicates.push(pred);
            }
        } else {
            for ty in assoc_debug_bounds {
                where_clause
                    .predicates
                    .push(parse_quote!(#ty: ::core::fmt::Debug));
            }
            for pred in field_level_overrides {
                where_clause.predicates.push(pred);
            }
        }
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = data_struct.fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();

        // parse #[debug = "format"] or #[debug("format")] via shared helper
        let (_bounds_unused, fmt_pattern) = extract_debug_attrs(&f.attrs);

        if let Some(fmt_str) = fmt_pattern {
            quote! {
                .field(#field_name_str, &format_args!(#fmt_str, self.#field_name))
            }
        } else {
            quote! {
                .field(#field_name_str, &self.#field_name)
            }
        }
    });

    let expanded = quote! {
        impl #impl_generics ::core::fmt::Debug for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(stringify!(#name))
                    #(#fields)*
                    .finish()
            }
        }
    };

    TokenStream::from(expanded)
}

fn collect_constraints_excluding_phantom(
    ty: &Type,
    generics: &HashSet<syn::Ident>,
    used_params: &mut HashSet<syn::Ident>,
    assoc_bounds: &mut Vec<Type>,
) {
    match ty {
        Type::Path(tp) => handle_type_path(tp, generics, used_params, assoc_bounds),
        Type::Reference(r) => {
            collect_constraints_excluding_phantom(&r.elem, generics, used_params, assoc_bounds)
        }
        Type::Tuple(t) => {
            for elem in &t.elems {
                collect_constraints_excluding_phantom(elem, generics, used_params, assoc_bounds);
            }
        }
        Type::Array(a) => {
            collect_constraints_excluding_phantom(&a.elem, generics, used_params, assoc_bounds)
        }
        Type::Slice(s) => {
            collect_constraints_excluding_phantom(&s.elem, generics, used_params, assoc_bounds)
        }
        Type::Ptr(p) => {
            collect_constraints_excluding_phantom(&p.elem, generics, used_params, assoc_bounds)
        }
        Type::Group(g) => {
            collect_constraints_excluding_phantom(&g.elem, generics, used_params, assoc_bounds)
        }
        Type::Paren(p) => {
            collect_constraints_excluding_phantom(&p.elem, generics, used_params, assoc_bounds)
        }
        _ => {}
    }
}

fn handle_type_path(
    tp: &TypePath,
    generics: &HashSet<syn::Ident>,
    used_params: &mut HashSet<syn::Ident>,
    assoc_bounds: &mut Vec<Type>,
) {
    // Ignore PhantomData completely
    if let Some(seg) = tp.path.segments.last() {
        if seg.ident == "PhantomData" {
            return;
        }
    }

    // Qualified path like <T as Trait>::Assoc
    if let Some(q) = &tp.qself {
        if uses_generic(&q.ty, generics) {
            assoc_bounds.push(Type::Path(tp.clone()));
        }
        return;
    }

    // Bare type parameter like T
    if tp.path.segments.len() == 1 {
        let seg = tp.path.segments.last().unwrap();
        if seg.arguments.is_empty() && generics.contains(&seg.ident) {
            used_params.insert(seg.ident.clone());
            return;
        }
    }

    // Associated type projection like T::Assoc
    if let Some(first) = tp.path.segments.first() {
        if generics.contains(&first.ident) && tp.path.segments.len() >= 2 {
            assoc_bounds.push(Type::Path(tp.clone()));
            return;
        }
    }

    // Recurse into generic arguments
    for seg in &tp.path.segments {
        match &seg.arguments {
            PathArguments::AngleBracketed(ab) => {
                for arg in &ab.args {
                    if let GenericArgument::Type(t) = arg {
                        collect_constraints_excluding_phantom(
                            t,
                            generics,
                            used_params,
                            assoc_bounds,
                        );
                    }
                }
            }
            PathArguments::Parenthesized(pb) => {
                for t in &pb.inputs {
                    collect_constraints_excluding_phantom(t, generics, used_params, assoc_bounds);
                }
            }
            PathArguments::None => {}
        }
    }
}

fn uses_generic(ty: &Type, generics: &HashSet<syn::Ident>) -> bool {
    struct Finder<'a> {
        gens: &'a HashSet<syn::Ident>,
        found: bool,
    }
    impl<'a> Finder<'a> {
        fn visit(&mut self, ty: &Type) {
            match ty {
                Type::Path(tp) => {
                    if tp.qself.is_none() && tp.path.segments.len() == 1 {
                        let seg = tp.path.segments.last().unwrap();
                        if seg.arguments.is_empty() && self.gens.contains(&seg.ident) {
                            self.found = true;
                            return;
                        }
                    }
                    for seg in &tp.path.segments {
                        if let PathArguments::AngleBracketed(ab) = &seg.arguments {
                            for arg in &ab.args {
                                if let GenericArgument::Type(t) = arg {
                                    self.visit(t);
                                    if self.found {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
                Type::Reference(r) => self.visit(&r.elem),
                Type::Tuple(t) => {
                    for e in &t.elems {
                        self.visit(e);
                        if self.found {
                            return;
                        }
                    }
                }
                Type::Array(a) => self.visit(&a.elem),
                Type::Slice(s) => self.visit(&s.elem),
                Type::Ptr(p) => self.visit(&p.elem),
                Type::Group(g) => self.visit(&g.elem),
                Type::Paren(p) => self.visit(&p.elem),
                _ => {}
            }
        }
    }
    let mut f = Finder {
        gens: generics,
        found: false,
    };
    f.visit(ty);
    f.found
}

fn parse_bounds_str(s: &str) -> Vec<WherePredicate> {
    let mut preds = Vec::new();
    for part in s.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        match syn::parse_str::<WherePredicate>(trimmed) {
            Ok(p) => preds.push(p),
            Err(_) => {
                // If parsing fails, ignore silently to keep macro resilient
            }
        }
    }
    preds
}

fn extract_debug_attrs(attrs: &[Attribute]) -> (Vec<WherePredicate>, Option<String>) {
    let mut preds = Vec::new();
    let mut fmt: Option<String> = None;
    for attr in attrs {
        if attr.path().is_ident("debug") {
            // NameValue form: #[debug = "..."]
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &nv.value
                {
                    fmt = Some(lit_str.value());
                    continue;
                }
            }

            // Nested meta: #[debug(bound = "..." )] or #[debug("...")]
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("bound") {
                    let lit: syn::LitStr = meta.value()?.parse()?;
                    preds.extend(parse_bounds_str(&lit.value()));
                } else {
                    // Try to parse a fmt string like #[debug("...")]
                    if fmt.is_none() {
                        if let Ok(_) = meta.value() {
                            if let Ok(lit) = meta.input.parse::<syn::LitStr>() {
                                fmt = Some(lit.value());
                            }
                        }
                    }
                }
                Ok(())
            });
        }
    }
    (preds, fmt)
}
