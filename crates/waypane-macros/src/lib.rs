use proc_macro::TokenStream;
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, FnArg, Lit, LitStr, Meta, Pat, ReturnType,
    parse::Parser, parse_macro_input,
};

#[proc_macro_derive(LuaClass, attributes(lua_class, lua_attr))]
pub fn derive_stubbed(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = &input.ident;
    let mut lua_name = ident.to_string();
    let mut parent_classes = Vec::new();

    // Override lua_name if #[lua_class(name = "...")] is provided
    for attr in &input.attrs {
        if attr.path().is_ident("lua_class")
            && let Err(err) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    lua_name = s.value();
                    Ok(())
                } else {
                    Err(meta.error("Unsupported #[lua_class] key. Expected: name = \"...\""))
                }
            })
        {
            return err.to_compile_error().into();
        }
    }

    // Process named fields to extract attributes and parent classes
    let (attrs_init, parent_merges) = if let Data::Struct(struct_data) = &input.data
        && let Fields::Named(fields) = &struct_data.fields
    {
        // Collect attributes and parent class merges
        let mut attr_quotes = Vec::new();
        let mut merge_quotes = Vec::new();

        for field in &fields.named {
            let field_name = field.ident.as_ref().expect("Expected named fields");
            let mut field_doc = extract_doc(&field.attrs);
            let field_type = &field.ty;

            let mut is_parent = false;
            let mut custom_name = field_name.to_string();
            let mut default_val = None;
            let mut is_optional = false;

            for attr in &field.attrs {
                // Process #[lua_attr(...)] attributes on the field
                if attr.path().is_ident("lua_attr")
                    && let Err(err) = attr.parse_nested_meta(|meta| {
                        // Check for simple attributes
                        if meta.path.is_ident("parent") {
                            is_parent = true;
                            Ok(())
                        } else if meta.path.is_ident("optional") {
                            is_optional = true;
                            Ok(())
                        } else if meta.path.is_ident("name") {
                            let value = meta.value()?;
                            let s: LitStr = value.parse()?;
                            custom_name = s.value();
                            Ok(())
                        } else if meta.path.is_ident("default") {
                            let expr: Expr = meta.value()?.parse()?;

                            if let Expr::Lit(expr_lit) = &expr
                                && let Lit::Str(s) = &expr_lit.lit
                            {
                                default_val = Some(s.value());
                            } else {
                                default_val = Some(quote!(#expr).to_string());
                            }
                            Ok(())
                        } else {
                            Err(meta.error(
                                "Unsupported #[lua_attr] key. Expected one of: parent, optional, name, default",
                            ))
                        }
                    }) {
                        return syn::Error::new_spanned(
                            attr,
                            format!("Invalid #[lua_attr(...)] attribute: {}", err),
                        )
                        .to_compile_error()
                        .into();
                }
            }

            let mut lua_type_quote =
                quote! { <#field_type as crate::lua::stubs::LuaType>::lua_type() };

            // Append default value info to the field documentation if provided
            if let Some(default) = &default_val {
                if !field_doc.is_empty() {
                    field_doc.push(' ');
                }

                field_doc.push_str(&format!("(Default: {})", default));
            }

            // Append ? to the beginning of the type if has default or is marked optional but its
            // type isn't explicitely Option<T> (e.g. Vec<String> that defaults to empty vec)
            if (default_val.is_some() || is_optional)
                && !lua_type_quote.to_string().starts_with('?')
            {
                lua_type_quote =
                    quote! { std::borrow::Cow::Owned(format!("? {}", #lua_type_quote)) };
            }

            // If this field is marked as a parent, merge its attributes into the current class's
            // attributes and add it to the list of parent classes
            if is_parent {
                merge_quotes.push(quote! {
                    let parent_attrs = <#field_type as crate::lua::stubs::Stubbed>::stubs().attrs;
                    attrs_vec.extend(parent_attrs.into_owned());
                });
                parent_classes.push(
                    quote! { <#field_type as crate::lua::stubs::Stubbed>::stubs().name.into() },
                );
            } else {
                // Otherwise, add it as a regular attribute of the class
                attr_quotes.push(quote! {
                    crate::lua::stubs::Attr {
                        name: #custom_name,
                        doc: #field_doc,
                        ty: #lua_type_quote,
                    }
                });
            }
        }

        // Initialize the attributes vector with the directly defined attributes
        // Parent class attributes will be merged in later
        // Using a Vec here to allow for dynamic merging of parent attributes, which may not be
        // known at compile time
        (quote! {vec![#(#attr_quotes),*]}, quote! {#(#merge_quotes)*})
    } else {
        return syn::Error::new_spanned(
            ident,
            "LuaClass can only be derived on structs with named fields",
        )
        .to_compile_error()
        .into();
    };

    let struct_doc = extract_doc(&input.attrs);

    let expanded = quote! {
        impl crate::lua::stubs::Stubbed for #ident {
            fn stubs() -> crate::lua::stubs::Class {
                let mut attrs_vec = #attrs_init;
                #parent_merges

                crate::lua::stubs::Class {
                    name: #lua_name,
                    parents: std::borrow::Cow::Owned(vec![#(#parent_classes),*]),
                    doc: #struct_doc,
                    attrs: std::borrow::Cow::Owned(attrs_vec),
                }
            }
        }

        impl crate::lua::stubs::LuaType for #ident {
            fn lua_type() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(#lua_name)
            }
        }

        // Register the class in the inventory for Lua stubs
        inventory::submit! {
            crate::lua::stubs::StubFactory {
                build: || crate::lua::stubs::Stub::Class(<#ident as crate::lua::stubs::Stubbed>::stubs()),
            }
        }
    };

    TokenStream::from(expanded)
}

/// Extracts doc comments from a list of attributes and concatenates them into a single string.
fn extract_doc(attrs: &[Attribute]) -> String {
    let mut docs = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("doc")
            && let Meta::NameValue(meta) = &attr.meta
            && let Expr::Lit(expr) = &meta.value
            && let Lit::Str(lit_str) = &expr.lit
        {
            docs.push(lit_str.value().trim().to_string());
        }
    }

    docs.join("\n")
}

#[proc_macro_derive(LuaEnum, attributes(lua))]
pub fn derive_lua_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();
    let enum_doc = extract_doc(&input.attrs);

    let mut variants_metadata = Vec::new();
    let mut lua_match_arms = Vec::new();

    if let Data::Enum(enum_data) = &input.data {
        for variant in &enum_data.variants {
            let variant_ident = &variant.ident;
            let variant_ident_str = variant_ident.to_string();

            // Allow variant name override via #[lua(name = "some-name")]
            let mut lua_variant_name = None;
            for attr in &variant.attrs {
                if attr.path().is_ident("lua") {
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("name") {
                            let value = meta.value()?;
                            let s: LitStr = value.parse()?;
                            lua_variant_name = Some(s.value());
                        }
                        Ok(())
                    });
                }
            }

            // Default to kebab-case if no override
            let lua_name = lua_variant_name.unwrap_or_else(|| {
                let mut s = String::new();
                for (i, c) in variant_ident_str.chars().enumerate() {
                    if i > 0 && c.is_uppercase() {
                        s.push('-');
                    }
                    s.extend(c.to_lowercase());
                }
                s
            });

            // Store for stubs and match arms
            variants_metadata.push(format!("\"{}\"", lua_name));
            lua_match_arms.push(quote! {
                #name::#variant_ident => #lua_name,
            });
        }
    } else {
        return syn::Error::new_spanned(name, "LuaEnum can only be derived on enums")
            .to_compile_error()
            .into();
    }

    let lua_type_variants = variants_metadata.join(" | ");

    let expanded = quote! {
        impl mlua::IntoLua for #name {
            fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
                let s = match self {
                    #(#lua_match_arms)*
                };
                s.into_lua(lua)
            }
        }

        impl crate::lua::stubs::LuaType for #name {
            fn lua_type() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(#name_str)
            }
        }

        inventory::submit! {
            crate::lua::stubs::StubFactory {
                build: || crate::lua::stubs::Stub::Enum(crate::lua::stubs::Enum {
                    name: #name_str,
                    doc: #enum_doc,
                    variants: #lua_type_variants,
                })
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn lua_func(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as syn::ItemFn);
    let name = &input.sig.ident;
    let mut name_str = name.to_string();
    let func_doc = extract_doc(&input.attrs);

    let mut module = None;
    let mut class = None;

    // Parse attributes passed directly to the macro
    let mut skip_args = HashSet::new();
    let attr_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("skip") {
            let value = meta.value()?;
            let s: LitStr = value.parse()?;
            skip_args.insert(s.value());
            Ok(())
        } else if meta.path.is_ident("name") {
            let value = meta.value()?;
            let s: LitStr = value.parse()?;
            name_str = s.value();
            Ok(())
        } else if meta.path.is_ident("module") {
            let value = meta.value()?;
            let s: LitStr = value.parse()?;
            module = Some(s.value());
            Ok(())
        } else if meta.path.is_ident("class") {
            let value = meta.value()?;
            let s: LitStr = value.parse()?;
            class = Some(s.value());
            Ok(())
        } else {
            Err(meta.error(
                "Unsupported #[lua_func(...)] key. Expected one of: skip, name, module, class",
            ))
        }
    });

    if let Err(err) = attr_parser.parse(attr) {
        return err.to_compile_error().into();
    }

    let fn_type = if let Some(class) = class {
        quote! { crate::lua::stubs::FnType::Method { class: #class } }
    } else {
        let module_quote = match module {
            Some(m) => quote! { Some(#m) },
            None => quote! { None },
        };
        quote! { crate::lua::stubs::FnType::Function { module: #module_quote } }
    };

    struct ArgOverride {
        ty: Option<String>,
        doc: Option<String>,
    }

    let mut arg_overrides = HashMap::new();
    let mut ret_ty_override = None;
    let mut ret_doc_override = None;
    let mut indices_to_remove = Vec::new();

    // Look for #[arg(...)] or #[ret(...)] attributes on the function itself
    for (i, attr) in input.attrs.iter().enumerate() {
        if attr.path().is_ident("arg") {
            let mut arg_name = String::new();
            let mut ty = None;
            let mut doc = None;

            if let Err(err) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    arg_name = s.value();
                    Ok(())
                } else if meta.path.is_ident("ty") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    ty = Some(s.value());
                    Ok(())
                } else if meta.path.is_ident("doc") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    doc = Some(s.value());
                    Ok(())
                } else {
                    Err(meta.error("Unsupported #[arg(...)] key. Expected one of: name, ty, doc"))
                }
            }) {
                return syn::Error::new_spanned(
                    attr,
                    format!("Invalid #[arg(...)] attribute: {}", err),
                )
                .to_compile_error()
                .into();
            }

            if !arg_name.is_empty() {
                arg_overrides.insert(arg_name, ArgOverride { ty, doc });
            }
            indices_to_remove.push(i);
        } else if attr.path().is_ident("ret") {
            if let Err(err) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("ty") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    ret_ty_override = Some(s.value());
                    Ok(())
                } else if meta.path.is_ident("doc") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    ret_doc_override = Some(s.value());
                    Ok(())
                } else {
                    Err(meta.error("Unsupported #[ret(...)] key. Expected one of: ty, doc"))
                }
            }) {
                return syn::Error::new_spanned(
                    attr,
                    format!("Invalid #[ret(...)] attribute: {}", err),
                )
                .to_compile_error()
                .into();
            }
            indices_to_remove.push(i);
        }
    }

    // Remove #[arg] and #[ret] attributes so they don't cause compile failures
    for &i in indices_to_remove.iter().rev() {
        input.attrs.remove(i);
    }

    let mut args = Vec::new();
    for input in &input.sig.inputs {
        if let FnArg::Typed(pat_type) = input
            && let Pat::Ident(ident) = &*pat_type.pat
        {
            let arg_name = ident.ident.to_string();

            // Skip arguments marked with 'skip' or those that start with underscores
            if skip_args.contains(&arg_name)
                || (arg_name.starts_with('_') && skip_args.contains(&arg_name[1..]))
            {
                continue;
            }

            // Determine the Lua type and documentation, using overrides if provided
            let (ty_quote, doc_val) = if let Some(over) = arg_overrides.get(&arg_name) {
                let t = over
                    .ty
                    .as_deref()
                    .map(|t| quote! { std::borrow::Cow::Borrowed(#t) })
                    .unwrap_or_else(|| {
                        let arg_type = &pat_type.ty;
                        quote! { <#arg_type as crate::lua::stubs::LuaType>::lua_type() }
                    });
                let d = over.doc.as_deref().unwrap_or("");
                (t, d)
            } else {
                let arg_type = &pat_type.ty;
                (
                    quote! { <#arg_type as crate::lua::stubs::LuaType>::lua_type() },
                    "",
                )
            };

            args.push(quote! {
                crate::lua::stubs::Attr {
                    name: #arg_name,
                    doc: #doc_val,
                    ty: #ty_quote,
                }
            });
        }
    }

    let ret_type = if let Some(ty) = ret_ty_override {
        quote! { std::borrow::Cow::Borrowed(#ty) }
    } else {
        match &input.sig.output {
            ReturnType::Default => quote! { std::borrow::Cow::Borrowed("nil") },
            ReturnType::Type(_, ty) => {
                quote! { <#ty as crate::lua::stubs::LuaType>::lua_type() }
            }
        }
    };
    let ret_doc = ret_doc_override.unwrap_or_default();

    let expanded = quote! {
        // Emit the original function with #[arg] and #[ret] attributes removed
        #input

        // Register the function stub in the global inventory
        inventory::submit! {
            crate::lua::stubs::StubFactory {
                build: || crate::lua::stubs::Stub::Function(crate::lua::stubs::Function {
                    ty: #fn_type,
                    name: #name_str,
                    doc: #func_doc,
                    args: std::borrow::Cow::Owned(vec![#(#args),*]),
                    ret: #ret_type,
                    ret_doc: #ret_doc,
                })
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(WidgetBuilder)]
pub fn derive_widget_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;

    let name = ident.to_string();
    let type_name = name.to_lowercase();
    let mut class_name = name.clone();
    let doc = extract_doc(&input.attrs);

    for attr in &input.attrs {
        if attr.path().is_ident("lua_class")
            && let Err(err) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    class_name = s.value();
                    Ok(())
                } else {
                    Err(meta.error("Unsupported #[lua_class] key. Expected: name = \"...\""))
                }
            })
        {
            return err.to_compile_error().into();
        }
    }

    let expanded = quote! {
        inventory::submit! {
            crate::lua::stubs::StubFactory {
                build: || crate::lua::stubs::Stub::WidgetBuilder(crate::lua::stubs::WidgetBuilder {
                    name: #name,
                    type_name: #type_name,
                    class_name: #class_name,
                    doc: #doc,
                }),
            }
        }

        inventory::submit! {
            crate::widgets::WidgetFactory {
                name: #type_name,
                build: |value, lua| {
                    let widget = #ident::from_lua(value, lua)?;
                    Ok(Box::new(widget))
                },
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(LuaModule, attributes(lua_module))]
pub fn derive_lua_module(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;

    let mut name = ident.to_string().to_lowercase();
    let doc = extract_doc(&input.attrs);
    let mut parent = quote! { None };

    for attr in &input.attrs {
        if attr.path().is_ident("lua_module")
            && let Err(err) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    name = s.value();
                    Ok(())
                } else if meta.path.is_ident("parent") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    parent = quote! { Some(#s) };
                    Ok(())
                } else {
                    Err(meta.error("Unsupported #[lua_module] key. Expected one of: name, parent"))
                }
            })
        {
            return err.to_compile_error().into();
        }
    }

    let expanded = quote! {
        inventory::submit! {
            crate::lua::stubs::StubFactory {
                build: || crate::lua::stubs::Stub::Module(crate::lua::stubs::Module {
                    name: #name,
                    parent: #parent,
                    doc: #doc,
                }),
            }
        }
    };

    TokenStream::from(expanded)
}
