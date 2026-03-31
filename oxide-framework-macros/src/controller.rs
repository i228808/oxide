use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, FnArg, ImplItem, ImplItemFn, ItemImpl, LitStr, Result, Type, parse2};

const ROUTE_ATTRS: &[&str] = &["get", "post", "put", "delete", "patch", "head", "options"];

struct RouteMethod {
    http_method: String,
    path: String,
    method_name: syn::Ident,
    has_self: bool,
    param_types: Vec<Type>,
}

pub fn expand(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let prefix: LitStr = parse2(attr)?;
    let prefix_str = prefix.value();

    let mut impl_block: ItemImpl = parse2(item)?;
    let self_ty = impl_block.self_ty.clone();

    let mut routes = Vec::new();
    let mut has_new = false;
    let mut has_middleware = false;

    for item in &impl_block.items {
        if let ImplItem::Fn(method) = item {
            if method.sig.ident == "new" {
                has_new = true;
            }
            if method.sig.ident == "middleware" {
                has_middleware = true;
            }
            if let Some(route) = parse_route_method(method)? {
                routes.push(route);
            }
        }
    }

    strip_route_attrs(&mut impl_block);

    let registrations = routes.iter().map(|route| {
        let router_method = format_ident!("{}", route.http_method);
        let path = &route.path;
        let method_name = &route.method_name;

        if route.has_self {
            let param_names: Vec<_> = (0..route.param_types.len())
                .map(|i| format_ident!("__p{}", i))
                .collect();
            let param_types = &route.param_types;

            if param_names.is_empty() {
                quote! {
                    {
                        let __ctrl = self.clone();
                        __router = __router.#router_method(#path, move || {
                            let __ctrl = __ctrl.clone();
                            async move { __ctrl.#method_name().await }
                        });
                    }
                }
            } else {
                quote! {
                    {
                        let __ctrl = self.clone();
                        __router = __router.#router_method(#path, move |#(#param_names: #param_types),*| {
                            let __ctrl = __ctrl.clone();
                            async move { __ctrl.#method_name(#(#param_names),*).await }
                        });
                    }
                }
            }
        } else {
            quote! {
                __router = __router.#router_method(#path, Self::#method_name);
            }
        }
    });

    let from_state_body = if has_new {
        quote! { Self::new(state) }
    } else {
        quote! { Self::default() }
    };

    let configure_router_impl = if has_middleware {
        quote! {
            fn configure_router(router: ::axum::Router) -> ::axum::Router {
                Self::middleware(router)
            }
        }
    } else {
        quote! {}
    };

    let output = quote! {
        #impl_block

        impl ::oxide_framework_core::Controller for #self_ty {
            const PREFIX: &'static str = #prefix_str;

            fn from_state(state: &::oxide_framework_core::AppState) -> Self {
                #from_state_body
            }

            fn register(self: ::std::sync::Arc<Self>) -> ::oxide_framework_core::OxideRouter {
                let mut __router = ::oxide_framework_core::OxideRouter::new();
                #(#registrations)*
                __router
            }

            #configure_router_impl
        }
    };

    Ok(output)
}

fn is_route_attr(attr: &Attribute) -> bool {
    attr.path()
        .get_ident()
        .map(|id| ROUTE_ATTRS.contains(&id.to_string().as_str()))
        .unwrap_or(false)
}

fn parse_route_method(method: &ImplItemFn) -> Result<Option<RouteMethod>> {
    for attr in &method.attrs {
        if let Some(ident) = attr.path().get_ident() {
            let method_str = ident.to_string();
            if ROUTE_ATTRS.contains(&method_str.as_str()) {
                let path: LitStr = attr.parse_args()?;

                let has_self = method
                    .sig
                    .inputs
                    .iter()
                    .any(|arg| matches!(arg, FnArg::Receiver(_)));

                let param_types: Vec<Type> = method
                    .sig
                    .inputs
                    .iter()
                    .filter_map(|arg| {
                        if let FnArg::Typed(pat_type) = arg {
                            Some((*pat_type.ty).clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                return Ok(Some(RouteMethod {
                    http_method: method_str,
                    path: path.value(),
                    method_name: method.sig.ident.clone(),
                    has_self,
                    param_types,
                }));
            }
        }
    }
    Ok(None)
}

fn strip_route_attrs(impl_block: &mut ItemImpl) {
    for item in &mut impl_block.items {
        if let ImplItem::Fn(method) = item {
            method.attrs.retain(|attr| !is_route_attr(attr));
        }
    }
}

