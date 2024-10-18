#![doc = include_str!("../../../README.md")]
extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    spanned::Spanned,
    token::Comma,
    AttrStyle, Attribute, FnArg, Ident, Pat, PatType, ReturnType, Token, Type, Visibility,
};

macro_rules! extend_errors {
    ($errors: ident, $e: expr) => {
        match $errors {
            Ok(_) => $errors = Err($e),
            Err(ref mut errors) => errors.extend($e),
        }
    };
}

struct Service {
    attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    ipcs: Vec<IpcMethod>,
}

struct IpcMethod {
    attrs: Vec<Attribute>,
    ident: Ident,
    args: Vec<PatType>,
    output: ReturnType,
}

impl Parse for Service {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        input.parse::<Token![trait]>()?;
        let ident: Ident = input.parse()?;
        let content;
        braced!(content in input);
        let mut ipcs = Vec::<IpcMethod>::new();
        while !content.is_empty() {
            ipcs.push(content.parse()?);
        }
        let mut ident_errors = Ok(());
        for ipc in &ipcs {
            if ipc.ident == "new" {
                extend_errors!(
                    ident_errors,
                    syn::Error::new(
                        ipc.ident.span(),
                        format!(
                            "method name conflicts with generated fn `{}Client::new`",
                            ident.unraw()
                        )
                    )
                );
            }
            if ipc.ident == "serve" {
                extend_errors!(
                    ident_errors,
                    syn::Error::new(
                        ipc.ident.span(),
                        format!("method name conflicts with generated fn `{ident}::serve`")
                    )
                );
            }
        }
        ident_errors?;

        Ok(Self {
            attrs,
            vis,
            ident,
            ipcs,
        })
    }
}

impl Parse for IpcMethod {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        input.parse::<Token![fn]>()?;
        let ident = input.parse()?;
        let content;
        parenthesized!(content in input);
        let mut args = Vec::new();
        let mut errors = Ok(());
        for arg in content.parse_terminated(FnArg::parse, Comma)? {
            match arg {
                FnArg::Typed(captured) if matches!(&*captured.pat, Pat::Ident(_)) => {
                    args.push(captured);
                }
                FnArg::Typed(captured) => {
                    extend_errors!(
                        errors,
                        syn::Error::new(captured.pat.span(), "patterns aren't allowed in IPC args")
                    );
                }
                FnArg::Receiver(_) => {
                    extend_errors!(
                        errors,
                        syn::Error::new(arg.span(), "method args cannot start with self")
                    );
                }
            }
        }
        errors?;
        let output = input.parse()?;
        input.parse::<Token![;]>()?;

        Ok(Self {
            attrs,
            ident,
            args,
            output,
        })
    }
}

fn collect_cfg_attrs(ipcs: &[IpcMethod]) -> Vec<Vec<&Attribute>> {
    ipcs.iter()
        .map(|ipc| {
            ipc.attrs
                .iter()
                .filter(|att| {
                    matches!(att.style, AttrStyle::Outer)
                        && match &att.meta {
                            syn::Meta::List(syn::MetaList { path, .. }) => {
                                path.get_ident() == Some(&Ident::new("cfg", ipc.ident.span()))
                            }
                            _ => false,
                        }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

#[proc_macro_attribute]
pub fn service(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let unit_type: &Type = &parse_quote!(());
    let Service {
        ref attrs,
        ref vis,
        ref ident,
        ref ipcs,
    } = parse_macro_input!(input as Service);

    let camel_case_fn_names: &Vec<_> = &ipcs
        .iter()
        .map(|ipc| snake_to_camel(&ipc.ident.unraw().to_string()))
        .collect();
    let args: &[&[PatType]] = &ipcs.iter().map(|ipc| &*ipc.args).collect::<Vec<_>>();

    let methods = ipcs.iter().map(|ipc| &ipc.ident).collect::<Vec<_>>();
    let request_names = methods
        .iter()
        .map(|m| format!("{ident}.{m}"))
        .collect::<Vec<_>>();

    ServiceGenerator {
        service_ident: ident,
        server_ident: &format_ident!("Serve{}", ident),
        client_ident: &format_ident!("{}Client", ident),
        request_ident: &format_ident!("{}Request", ident),
        response_ident: &format_ident!("{}Response", ident),
        vis,
        args,
        method_attrs: &ipcs.iter().map(|ipc| &*ipc.attrs).collect::<Vec<_>>(),
        method_cfgs: &collect_cfg_attrs(ipcs),
        method_idents: &methods,
        request_names: &request_names,
        attrs,
        ipcs,
        return_types: &ipcs
            .iter()
            .map(|ipc| match ipc.output {
                ReturnType::Type(_, ref ty) => ty.as_ref(),
                ReturnType::Default => unit_type,
            })
            .collect::<Vec<_>>(),
        arg_pats: &args
            .iter()
            .map(|args| args.iter().map(|arg| &*arg.pat).collect())
            .collect::<Vec<_>>(),
        camel_case_idents: &ipcs
            .iter()
            .zip(camel_case_fn_names.iter())
            .map(|(ipc, name)| Ident::new(name, ipc.ident.span()))
            .collect::<Vec<_>>(),
    }
    .into_token_stream()
    .into()
}

struct ServiceGenerator<'a> {
    service_ident: &'a Ident,
    server_ident: &'a Ident,
    client_ident: &'a Ident,
    request_ident: &'a Ident,
    response_ident: &'a Ident,
    vis: &'a Visibility,
    attrs: &'a [Attribute],
    ipcs: &'a [IpcMethod],
    camel_case_idents: &'a [Ident],
    method_idents: &'a [&'a Ident],
    request_names: &'a [String],
    method_attrs: &'a [&'a [Attribute]],
    method_cfgs: &'a [Vec<&'a Attribute>],
    args: &'a [&'a [PatType]],
    return_types: &'a [&'a Type],
    arg_pats: &'a [Vec<&'a Pat>],
}

impl<'a> ServiceGenerator<'a> {
    fn trait_service(&self) -> TokenStream2 {
        let &Self {
            attrs,
            ipcs,
            vis,
            return_types,
            service_ident,
            server_ident,
            ..
        } = self;

        let ipc_fns = ipcs.iter().zip(return_types.iter()).map(
            |(
                IpcMethod {
                    attrs, ident, args, ..
                },
                output,
            )| {
                quote! {
                    #( #attrs )*
                    fn #ident(&mut self, #( #args ),*) -> #output;
                }
            },
        );

        quote! {
            #( #attrs )*
            #vis trait #service_ident: ::core::marker::Sized {
                #( #ipc_fns )*
                fn server(self) -> #server_ident<Self> {
                    #server_ident { service: self }
                }
            }
        }
    }

    fn struct_server(&self) -> TokenStream2 {
        let &Self {
            vis, server_ident, ..
        } = self;

        quote! {
            #[derive(Clone)]
            #vis struct #server_ident<S> {
                service: S,
            }
        }
    }

    fn impl_serve_for_server(&self) -> TokenStream2 {
        let &Self {
            request_ident,
            server_ident,
            service_ident,
            response_ident,
            camel_case_idents,
            arg_pats,
            method_idents,
            method_cfgs,
            ..
        } = self;

        quote! {
            impl<S> ckb_script_ipc_common::ipc::Serve for #server_ident<S>
                where S: #service_ident
            {
                type Req = #request_ident;
                type Resp = #response_ident;


                fn serve(&mut self, req: #request_ident)
                    -> ::core::result::Result<#response_ident, ckb_script_ipc_common::error::IpcError> {
                    match req {
                        #(
                            #( #method_cfgs )*
                            #request_ident::#camel_case_idents{ #( #arg_pats ),* } => {
                                let ret = self.service.#method_idents(#( #arg_pats ),*);
                                Ok(#response_ident::#camel_case_idents(ret))
                            }
                        )*
                    }
                }
            }
        }
    }

    fn enum_request(&self) -> TokenStream2 {
        let &Self {
            vis,
            request_ident,
            camel_case_idents,
            args,
            method_cfgs,
            ..
        } = self;

        quote! {
            #[derive(serde::Serialize, serde::Deserialize)]
            #vis enum #request_ident {
                #(
                    #( #method_cfgs )*
                    #camel_case_idents{ #( #args ),* }
                ),*
            }
        }
    }

    fn enum_response(&self) -> TokenStream2 {
        let &Self {
            vis,
            response_ident,
            camel_case_idents,
            return_types,
            ..
        } = self;

        quote! {
            #[derive(serde::Serialize, serde::Deserialize)]
            #vis enum #response_ident {
                #( #camel_case_idents(#return_types) ),*
            }
        }
    }

    fn struct_client(&self) -> TokenStream2 {
        let &Self {
            vis, client_ident, ..
        } = self;

        quote! {
            #[allow(unused)]
            #vis struct #client_ident<R, W>
            where
                R: ckb_script_ipc_common::io::Read<Error = ckb_script_ipc_common::error::IpcError>,
                W: ckb_script_ipc_common::io::Write<Error = ckb_script_ipc_common::error::IpcError>,
            {
                channel: ckb_script_ipc_common::channel::Channel<R, W>,
            }
        }
    }

    fn impl_client_new(&self) -> TokenStream2 {
        let &Self {
            client_ident, vis, ..
        } = self;

        quote! {
            impl<R, W> #client_ident<R, W>
            where
                R: ckb_script_ipc_common::io::Read<Error = ckb_script_ipc_common::error::IpcError>,
                W: ckb_script_ipc_common::io::Write<Error = ckb_script_ipc_common::error::IpcError>,
            {
                #vis fn new(reader: R, writer: W) -> Self {
                    let channel = ckb_script_ipc_common::channel::Channel::new(reader, writer);
                    Self { channel }
                }
            }
        }
    }

    fn impl_client_ipc_methods(&self) -> TokenStream2 {
        let &Self {
            client_ident,
            request_ident,
            response_ident,
            method_attrs,
            vis,
            method_idents,
            args,
            return_types,
            arg_pats,
            camel_case_idents,
            request_names,
            ..
        } = self;

        quote! {
            impl<R, W> #client_ident<R, W>
            where
                R: ckb_script_ipc_common::io::Read<Error = ckb_script_ipc_common::error::IpcError>,
                W: ckb_script_ipc_common::io::Write<Error = ckb_script_ipc_common::error::IpcError>
            {
                #(
                    #[allow(unused)]
                    #( #method_attrs )*
                    #vis fn #method_idents(&mut self, #( #args ),*) -> #return_types {
                        let request = #request_ident::#camel_case_idents { #( #arg_pats ),* };
                        let resp: Result<_, ckb_script_ipc_common::error::IpcError> = self
                                .channel
                                .call::<_, #response_ident>(#request_names, request);
                        match resp {
                            Ok(#response_ident::#camel_case_idents(ret)) => ret,
                            Err(e) => {
                                panic!("IPC error: {:?}", e);
                            },
                            _ => {
                                panic!("IPC error: wrong method id");
                            }
                        }
                    }
                )*
            }
        }
    }
}

impl<'a> ToTokens for ServiceGenerator<'a> {
    fn to_tokens(&self, output: &mut TokenStream2) {
        output.extend(vec![
            self.trait_service(),
            self.struct_server(),
            self.impl_serve_for_server(),
            self.enum_request(),
            self.enum_response(),
            self.struct_client(),
            self.impl_client_new(),
            self.impl_client_ipc_methods(),
        ]);
    }
}

fn snake_to_camel(ident_str: &str) -> String {
    ident_str
        .split('_')
        .map(|word| word[..1].to_uppercase() + &word[1..].to_lowercase())
        .collect()
}
