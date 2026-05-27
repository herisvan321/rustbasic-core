use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item, TraitItem, ImplItem, Signature, ReturnType, FnArg};

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
