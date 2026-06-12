use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item, TraitItem, ImplItem, Signature, ReturnType, FnArg, DeriveInput, Meta, Expr, Lit};

#[proc_macro_attribute]
pub fn async_trait(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as Item);
    match input {
        Item::Trait(mut trait_item) => {
            for item in &mut trait_item.items {
                if let TraitItem::Fn(method) = item {
                    if method.sig.asyncness.is_some() {
                        let original_body = method.default.clone();
                        transform_signature(&mut method.sig);
                        if let Some(body) = original_body {
                            method.default = Some(syn::parse2(quote! {
                                {
                                    ::std::boxed::Box::pin(async move {
                                        #body
                                    })
                                }
                            }).unwrap());
                        }
                    }
                }
            }
            TokenStream::from(quote!(#trait_item))
        }
        Item::Impl(mut impl_item) => {
            for item in &mut impl_item.items {
                if let ImplItem::Fn(method) = item {
                    if method.sig.asyncness.is_some() {
                        let original_body = method.block.clone();
                        transform_signature(&mut method.sig);
                        method.block = syn::parse2(quote! {
                            {
                                ::std::boxed::Box::pin(async move {
                                    #original_body
                                })
                            }
                        }).unwrap();
                    }
                }
            }
            TokenStream::from(quote!(#impl_item))
        }
        _ => TokenStream::from(quote!(#input)),
    }
}

fn transform_signature(sig: &mut Signature) {
    sig.asyncness = None;
    let ret_type = match &sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, ty) => quote!(#ty),
    };

    // Find any lifetime in the generics. If none, default to '_
    let mut lifetime_str = quote!('_);
    for param in &sig.generics.params {
        if let syn::GenericParam::Lifetime(lt) = param {
            let lt_ident = &lt.lifetime;
            lifetime_str = quote!(#lt_ident);
            break;
        }
    }

    // If there is self, and we have a specific lifetime like 'a, bind self to 'a
    if lifetime_str.to_string() != "'_" {
        for arg in &mut sig.inputs {
            if let FnArg::Receiver(receiver) = arg {
                if let Some((_, ref mut opt_lifetime)) = receiver.reference {
                    if opt_lifetime.is_none() {
                        let lt: syn::Lifetime = syn::parse2(quote!(#lifetime_str)).unwrap();
                        *opt_lifetime = Some(lt);
                    }
                }
            }
        }
    }

    sig.output = syn::parse2(quote! {
        -> ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output = #ret_type> + ::std::marker::Send + #lifetime_str>>
    }).unwrap();
}

#[proc_macro_derive(RustEmbed, attributes(folder))]
pub fn derive_rust_embed(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let mut folder_path = None;
    for attr in &ast.attrs {
        if attr.path().is_ident("folder") {
            if let Meta::NameValue(meta) = &attr.meta {
                if let Expr::Lit(expr_lit) = &meta.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        folder_path = Some(lit_str.value());
                    }
                }
            }
        }
    }

    let folder = folder_path.expect("attribute #[folder = \"...\"] is required for RustEmbed");
    
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let abs_path = std::path::Path::new(&manifest_dir).join(&folder);

    // Recursively read all files in directory
    let mut files = Vec::new();
    if abs_path.exists() {
        files = get_files_recursively(&abs_path, &abs_path);
    }

    let file_paths: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
    let file_bytes: Vec<Vec<u8>> = files.iter().map(|(_, b)| b.clone()).collect();

    let file_bytes_tokens: Vec<_> = file_bytes.iter().map(|bytes| {
        quote! { &[#(#bytes),*] }
    }).collect();

    let gen_code = quote! {
        impl #name {
            pub fn get(file_path: &str) -> Option<rustbasic_core::rust_embed::EmbeddedFile> {
                let normalized_path = file_path.replace("\\", "/");
                match normalized_path.as_str() {
                    #(
                        #file_paths => Some(rustbasic_core::rust_embed::EmbeddedFile {
                            data: std::borrow::Cow::Borrowed(#file_bytes_tokens),
                            metadata: rustbasic_core::rust_embed::Metadata {
                                last_modified: None,
                                created: None,
                                sha256_hash: [0; 32],
                            }
                        }),
                    )*
                    _ => None,
                }
            }

            pub fn iter() -> impl Iterator<Item = std::borrow::Cow<'static, str>> {
                let items: Vec<std::borrow::Cow<'static, str>> = vec![
                    #( std::borrow::Cow::Borrowed(#file_paths) ),*
                ];
                items.into_iter()
            }
        }
    };

    gen_code.into()
}

fn get_files_recursively(dir: &std::path::Path, base_dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(get_files_recursively(&path, base_dir));
            } else if path.is_file() {
                if let Ok(content) = std::fs::read(&path) {
                    let rel_path = path.strip_prefix(base_dir).unwrap().to_string_lossy().to_string();
                    let rel_path = rel_path.replace("\\", "/");
                    files.push((rel_path, content));
                }
            }
        }
    }
    files
}
