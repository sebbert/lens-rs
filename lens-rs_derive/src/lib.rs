extern crate proc_macro;
#[macro_use]
extern crate lazy_static;
use proc_macro::TokenStream;
use quote::*;
use syn::punctuated::Punctuated;
use syn::Token;
use syn::{parse_macro_input, Data, DeriveInput, parenthesized};
use proc_macro2::Span;

use std::collections::HashSet;
use std::sync::Mutex;
use syn::parse::{Parse, Result, ParseStream};

lazy_static! {
    static ref OPTIC_NAMES: Mutex<HashSet<String>> = {
        let mut m = HashSet::new();
        m.insert("_Ok".to_string());
        m.insert("_Err".to_string());
        m.insert("_Some".to_string());
        m.insert("_None".to_string());
        Mutex::new(m)
    };
}

enum OpticMutability {
    Move,
    Ref(Token![ref]),
    Mut(Token![mut]),
}

impl Parse for OpticMutability {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() { return Ok(Self::Move); }

        let content;

        parenthesized!(content in input);
        let lookahead = content.lookahead1();
        println!("{:?}", content);
        if lookahead.peek(Token![mut]) {
            Ok(Self::Mut(content.parse()?))
        } else if lookahead.peek(Token![ref]) {
            Ok(Self::Ref(content.parse()?))
        } else  {
            Err(input.error("only allow #[optic], #[optic(mut)] or #[optic(ref)] here"))
        }
    }
}


#[proc_macro_derive(Optic, attributes(optic))]
pub fn derive_optic(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let optics = match derive_input.data {
        Data::Enum(e) => e
            .variants
            .iter()
            .filter(|var| {
                var
                    .attrs
                    .iter()
                    .any(|attr| attr.path.is_ident(&syn::Ident::new("optic", Span::call_site())))
            })
            .flat_map(|x| {
                let optic_name = format_ident!("_{}", x.ident);
                let mut table = OPTIC_NAMES.lock().unwrap();
                if table.contains(&optic_name.clone().to_string()) { return quote! {}; }
                table.insert(optic_name.clone().to_string());
                quote! {
                    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
                    pub struct #optic_name<Optic>(pub Optic);
                }
            })
            .collect(),
        Data::Struct(st) => st
            .fields
            .iter()
            .filter(|var| {
                var
                    .attrs
                    .iter()
                    .any(|attr| attr.path.is_ident(&syn::Ident::new("optic", Span::call_site())))
            })
            .flat_map(|x| {
                let optic_name = format_ident!("_{}", x.ident.as_ref()?);
                let mut table = OPTIC_NAMES.lock().unwrap();
                if table.contains(&optic_name.clone().to_string()) { return None; }
                table.insert(optic_name.clone().to_string());
                Some(quote! {
                    #[derive(Copy, Clone)]
                    #[allow(non_camel_case_types)]
                    pub struct #optic_name<Optic>(pub Optic);
                })
            })
            .collect(),
        _ => quote! {},
    };
    TokenStream::from(optics)
}

#[proc_macro_derive(Review, attributes(optic))]
pub fn derive_review(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    let reviews: proc_macro2::TokenStream = match derive_input.data.clone() {
        Data::Enum(e) => e
            .variants
            .iter()
            .filter(|var| {
                var
                    .attrs
                    .iter()
                    .any(|attr| attr.path.is_ident(&syn::Ident::new("optic", Span::call_site())))
            })
            .flat_map(|var| {
                let data = derive_input.clone();
                let data_name = data.ident;
                let data_gen = data.generics;
                let data_gen_param = data_gen.params.iter().collect::<Vec<_>>();
                let data_gen_where = data_gen
                    .where_clause
                    .iter()
                    .flat_map(|x| x.predicates.clone())
                    .collect::<Punctuated<_, Token![,]>>();

                let var_name = &var.ident;
                let optic_name = format_ident!("_{}", var.ident);
                let ty = var
                    .fields
                    .iter()
                    .take(1)
                    .map(|field| field.ty.clone())
                    .collect::<Punctuated<_, Token![,]>>();

                // let fields = var
                //     .fields
                //     .iter()
                //     .enumerate()
                //     .flat_map(|(i, _)| {
                //         let i = syn::Index::from(i);
                //         quote! { #i }
                //     })
                //     .collect::<Vec<_>>();

                quote! {
                    impl<#(#data_gen_param,)* Rv> lens_rs::Review<#data_name #data_gen> for #optic_name<Rv>
                    where
                        Rv: lens_rs::Review<#ty>,
                        #data_gen_where
                    {
                        type From = Rv::From;

                        fn review(&self, from: Self::From) -> #data_name #data_gen {
                            // let tuple = self.0.review(from);
                            // <#data_name #data_gen>::#var_name(#(tuple . #fields,)*)
                            <#data_name #data_gen>::#var_name(self.0.review(from))
                        }
                    }
                }
            })
            .collect(),
        _ => panic!("union and struct can't derive the review"),
    };
    TokenStream::from(reviews)
}

#[proc_macro_derive(Prism, attributes(optic))]
pub fn derive_prism(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    let prisms: proc_macro2::TokenStream  = match derive_input.data.clone() {
        Data::Enum(e) => e
            .variants
            .iter()
            .filter(|var| {
                var
                    .attrs
                    .iter()
                    .any(|attr| attr.path.is_ident(&syn::Ident::new("optic", Span::call_site())))
            })
            .flat_map(|var| {
                let data = derive_input.clone();
                let data_name = data.ident;
                let data_gen = data.generics;
                let data_gen_param = data_gen.params.iter().collect::<Vec<_>>();
                let data_gen_where = data_gen
                    .where_clause
                    .iter()
                    .flat_map(|x| x.predicates.clone())
                    .collect::<Punctuated<_, Token![,]>>();

                let var_name = &var.ident;
                let optic_name = format_ident!("_{}", var.ident);
                let ty = var
                    .fields
                    .iter()
                    .map(|field| field.ty.clone())
                    .take(1)
                    .collect::<Punctuated<_, Token![,]>>();
                let attr: syn::Attribute = var
                    .attrs
                    .clone()
                    .into_iter()
                    .find(|attr: &syn::Attribute| attr.path.is_ident(&syn::Ident::new("optic", Span::call_site())))
                    .unwrap();
                let mutability = syn::parse::<OpticMutability>(TokenStream::from(attr.tokens)).unwrap();



                let impl_ref = quote! {
                    impl<#(#data_gen_param,)* Tr> lens_rs::TraversalRef<#data_name #data_gen> for #optic_name<Tr>
                    where
                        Tr: lens_rs::TraversalRef<#ty>,
                        #data_gen_where
                    {
                        type To = Tr::To;

                        fn traverse_ref<'a>(&self, source: &'a #data_name #data_gen) -> Vec<&'a Self::To> {
                            use #data_name::*;
                            match source {
                                #var_name(x) => self.0.traverse_ref(x),
                                _ => vec![],
                            }
                        }
                    }

                    impl<#(#data_gen_param,)* Pm> lens_rs::PrismRef<#data_name #data_gen> for #optic_name<Pm>
                    where
                        Pm: lens_rs::PrismRef<#ty>,
                        #data_gen_where
                    {
                        type To = Pm::To;
                        fn pm_ref<'a>(&self, source: &'a #data_name #data_gen) -> Option<&'a Self::To> {
                            use #data_name::*;
                            match source {
                                #var_name(x) => self.0.pm_ref(x),
                                _ => None,
                            }
                        }
                    }
                };

                let impl_mut = quote! {
                    impl<#(#data_gen_param,)* Tr> lens_rs::TraversalMut<#data_name #data_gen> for #optic_name<Tr>
                    where
                        Tr: lens_rs::TraversalMut<#ty>,
                        #data_gen_where
                    {
                        fn traverse_mut<'a>(&self, source: &'a mut #data_name #data_gen) -> Vec<&'a mut Self::To> {
                            use #data_name::*;
                            match source {
                                #var_name(x) => self.0.traverse_mut(x),
                                _ => vec![],
                            }
                        }
                    }

                    impl<#(#data_gen_param,)* Pm> lens_rs::PrismMut<#data_name #data_gen> for #optic_name<Pm>
                    where
                        Pm: lens_rs::PrismMut<#ty>,
                        #data_gen_where
                    {
                        fn pm_mut<'a>(&self, source: &'a mut #data_name #data_gen) -> Option<&'a mut Self::To> {
                            use #data_name::*;
                            match source {
                                #var_name(x) => self.0.pm_mut(x),
                                _ => None,
                            }
                        }
                    }
                };

                let impl_mv = quote! {
                    impl<#(#data_gen_param,)* Tr> lens_rs::Traversal<#data_name #data_gen> for #optic_name<Tr>
                    where
                        Tr: lens_rs::Traversal<#ty>,
                        #data_gen_where
                    {
                        fn traverse(&self, source: #data_name #data_gen) -> Vec<Self::To> {
                            use #data_name::*;
                            match source {
                                #var_name(x) => self.0.traverse(x),
                                _ => vec![],
                            }
                        }
                    }

                    impl<#(#data_gen_param,)* Pm> lens_rs::Prism<#data_name #data_gen> for #optic_name<Pm>
                    where
                        Pm: lens_rs::Prism<#ty>,
                        #data_gen_where
                    {
                        fn pm(&self, source: #data_name #data_gen) -> Option<Self::To> {
                            use #data_name::*;
                            match source {
                                #var_name(x) => self.0.pm(x),
                                _ => None,
                            }
                        }
                    }
                };

                match mutability {
                    OpticMutability::Ref(_) => vec![impl_ref],
                    OpticMutability::Mut(_) => vec![impl_mut, impl_ref],
                    OpticMutability::Move   => vec![impl_mv, impl_mut, impl_ref]
                }.into_iter().flat_map(|x| x)
            })
            .collect(),
        _ => panic!("union and struct can't derive the review"),
    };

    TokenStream::from(prisms)
}

#[proc_macro_derive(Lens, attributes(optic))]
pub fn derive_lens(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    let lens: proc_macro2::TokenStream = match derive_input.data.clone() {
        Data::Struct(syn::DataStruct { fields: syn::Fields::Named(fs), .. }) => fs
            .named
            .iter()
            .filter(|var| {
                var
                    .attrs
                    .iter()
                    .any(|attr| attr.path.is_ident(&syn::Ident::new("optic", Span::call_site())))
            })
            .flat_map(|f| {
                let data = derive_input.clone();
                let data_name = data.ident;
                let data_gen = data.generics;
                let data_gen_param = data_gen.params.iter().collect::<Vec<_>>();
                let data_gen_where = data_gen
                    .where_clause
                    .iter()
                    .flat_map(|x| x.predicates.clone())
                    .collect::<Punctuated<_, Token![,]>>();

                let optics_name = format_ident!("_{}", f.ident.as_ref().unwrap());
                let to = &f.ty;
                let field_name = f.ident.as_ref().unwrap();

                let attr: syn::Attribute = f
                    .attrs
                    .clone()
                    .into_iter()
                    .find(|attr: &syn::Attribute| attr.path.is_ident(&syn::Ident::new("optic", Span::call_site())))
                    .unwrap();
                let mutability = syn::parse::<OpticMutability>(TokenStream::from(attr.tokens)).unwrap();

                let impl_ref = quote! {
                    impl<#(#data_gen_param,)* Tr> lens_rs::TraversalRef<#data_name #data_gen> for #optics_name<Tr>
                    where
                        Tr: lens_rs::TraversalRef<#to>,
                        #data_gen_where
                    {
                        type To = Tr::To;

                        fn traverse_ref<'a>(&self, source: &'a #data_name #data_gen) -> Vec<&'a Self::To> {
                            self.0.traverse_ref(&source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Pm> lens_rs::PrismRef<#data_name #data_gen> for #optics_name<Pm>
                    where
                        Pm: lens_rs::PrismRef<#to>,
                        #data_gen_where
                    {
                        type To = Pm::To;

                        fn pm_ref<'a>(&self, source: &'a #data_name #data_gen) -> Option<&'a Self::To> {
                            self.0.pm_ref(&source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Ls> lens_rs::LensRef<#data_name #data_gen> for #optics_name<Ls>
                    where
                        Ls: LensRef<#to>,
                        #data_gen_where
                    {
                        type To = Ls::To;

                        fn view_ref<'a>(&self, source: &'a #data_name #data_gen) -> &'a Self::To {
                            self.0.view_ref(&source.#field_name)
                        }
                    }
                };

                let impl_mut = quote! {
                    impl<#(#data_gen_param,)* Tr> lens_rs::TraversalMut<#data_name #data_gen> for #optics_name<Tr>
                    where
                        Tr: lens_rs::Traversal<#to>,
                        #data_gen_where
                    {
                        fn traverse_mut<'a>(&self, source: &'a mut #data_name #data_gen) -> Vec<&'a mut Self::To> {
                            self.0.traverse_mut(&mut source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Pm> lens_rs::PrismMut<#data_name #data_gen> for #optics_name<Pm>
                    where
                        Pm: lens_rs::PrismMut<#to>,
                        #data_gen_where
                    {
                        fn pm_mut<'a>(&self, source: &'a mut #data_name #data_gen) -> Option<&'a mut Self::To> {
                            self.0.pm_mut(&mut source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Ls> lens_rs::LensMut<#data_name #data_gen> for #optics_name<Ls>
                    where
                        Ls: LensMut<#to>,
                        #data_gen_where
                    {
                        fn view_mut<'a>(&self, source: &'a mut #data_name #data_gen) -> &'a mut Self::To {
                            self.0.view_mut(&mut source.#field_name)
                        }
                    }

                };

                let impl_mv = quote! {
                    impl<#(#data_gen_param,)* Tr> lens_rs::Traversal<#data_name #data_gen> for #optics_name<Tr>
                    where
                        Tr: lens_rs::Traversal<#to>,
                        #data_gen_where
                    {
                        fn traverse(&self, source: #data_name #data_gen) -> Vec<Self::To> {
                            self.0.traverse(source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Pm> lens_rs::Prism<#data_name #data_gen> for #optics_name<Pm>
                    where
                        Pm: lens_rs::Prism<#to>,
                        #data_gen_where
                    {
                        fn pm(&self, source: #data_name #data_gen) -> Option<Self::To> {
                            self.0.pm(source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Ls> lens_rs::Lens<#data_name #data_gen> for #optics_name<Ls>
                    where
                        Ls: Lens<#to>,
                        #data_gen_where
                    {
                        fn view(&self, source: #data_name #data_gen) -> Self::To {
                            self.0.view(source.#field_name)
                        }
                    }
                };

                match mutability {
                    OpticMutability::Ref(_) => vec![impl_ref],
                    OpticMutability::Mut(_) => vec![impl_mut, impl_ref],
                    OpticMutability::Move   => vec![impl_mv, impl_mut, impl_ref]
                }.into_iter().flat_map(|x| x)
            }).collect(),
        Data::Struct(syn::DataStruct { fields: syn::Fields::Unnamed(fs), .. }) => fs
            .unnamed
            .iter()
            .take(7)
            .filter(|var| {
                var
                    .attrs
                    .iter()
                    .any(|attr| attr.path.is_ident(&syn::Ident::new("optic", Span::call_site())))
            })
            .enumerate()
            .flat_map(|(i, f)| {
                let data = derive_input.clone();
                let data_name = data.ident;
                let data_gen = data.generics;
                let data_gen_param = data_gen.params.iter().collect::<Vec<_>>();
                let data_gen_where = data_gen
                    .where_clause
                    .iter()
                    .flat_map(|x| x.predicates.clone())
                    .collect::<Punctuated<_, Token![,]>>();

                let optics_name = format_ident!("_{}", i);
                let to = &f.ty;
                let field_name = syn::Index::from(i);

                let attr: syn::Attribute = f
                    .attrs
                    .clone()
                    .into_iter()
                    .find(|attr: &syn::Attribute| attr.path.is_ident(&syn::Ident::new("optic", Span::call_site())))
                    .unwrap();
                let mutability = syn::parse::<OpticMutability>(TokenStream::from(attr.tokens)).unwrap();

                let impl_ref = quote! {
                    impl<#(#data_gen_param,)* Tr> lens_rs::TraversalRef<#data_name #data_gen> for #optics_name<Tr>
                    where
                        Tr: lens_rs::TraversalRef<#to>,
                        #data_gen_where
                    {
                        type To = Tr::To;

                        fn traverse_ref<'a>(&self, source: &'a #data_name #data_gen) -> Vec<&'a Self::To> {
                            self.0.traverse_ref(&source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Pm> lens_rs::PrismRef<#data_name #data_gen> for #optics_name<Pm>
                    where
                        Pm: lens_rs::PrismRef<#to>,
                        #data_gen_where
                    {
                        type To = Pm::To;

                        fn pm_ref<'a>(&self, source: &'a #data_name #data_gen) -> Option<&'a Self::To> {
                            self.0.pm_ref(&source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Ls> lens_rs::LensRef<#data_name #data_gen> for #optics_name<Ls>
                    where
                        Ls: LensRef<#to>,
                        #data_gen_where
                    {
                        type To = Ls::To;

                        fn view_ref<'a>(&self, source: &'a #data_name #data_gen) -> &'a Self::To {
                            self.0.view_ref(&source.#field_name)
                        }
                    }
                };

                let impl_mut = quote! {
                    impl<#(#data_gen_param,)* Tr> lens_rs::TraversalMut<#data_name #data_gen> for #optics_name<Tr>
                    where
                        Tr: lens_rs::Traversal<#to>,
                        #data_gen_where
                    {
                        fn traverse_mut<'a>(&self, source: &'a mut #data_name #data_gen) -> Vec<&'a mut Self::To> {
                            self.0.traverse_mut(&mut source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Pm> lens_rs::PrismMut<#data_name #data_gen> for #optics_name<Pm>
                    where
                        Pm: lens_rs::PrismMut<#to>,
                        #data_gen_where
                    {
                        fn pm_mut<'a>(&self, source: &'a mut #data_name #data_gen) -> Option<&'a mut Self::To> {
                            self.0.pm_mut(&mut source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Ls> lens_rs::LensMut<#data_name #data_gen> for #optics_name<Ls>
                    where
                        Ls: LensMut<#to>,
                        #data_gen_where
                    {
                        fn view_mut<'a>(&self, source: &'a mut #data_name #data_gen) -> &'a mut Self::To {
                            self.0.view_mut(&mut source.#field_name)
                        }
                    }

                };

                let impl_mv = quote! {
                    impl<#(#data_gen_param,)* Tr> lens_rs::Traversal<#data_name #data_gen> for #optics_name<Tr>
                    where
                        Tr: lens_rs::Traversal<#to>,
                        #data_gen_where
                    {
                        fn traverse(&self, source: #data_name #data_gen) -> Vec<Self::To> {
                            self.0.traverse(source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Pm> lens_rs::Prism<#data_name #data_gen> for #optics_name<Pm>
                    where
                        Pm: lens_rs::Prism<#to>,
                        #data_gen_where
                    {
                        fn pm(&self, source: #data_name #data_gen) -> Option<Self::To> {
                            self.0.pm(source.#field_name)
                        }
                    }

                    impl<#(#data_gen_param,)* Ls> lens_rs::Lens<#data_name #data_gen> for #optics_name<Ls>
                    where
                        Ls: Lens<#to>,
                        #data_gen_where
                    {
                        fn view(&self, source: #data_name #data_gen) -> Self::To {
                            self.0.view(source.#field_name)
                        }
                    }
                };

                match mutability {
                    OpticMutability::Ref(_) => vec![impl_ref],
                    OpticMutability::Mut(_) => vec![impl_mut, impl_ref],
                    OpticMutability::Move   => vec![impl_mv, impl_mut, impl_ref]
                }.into_iter().flat_map(|x| x)
            }).collect(),
        _ => panic!("union and enum can't derive the lens"),
    };

    TokenStream::from(lens)
}
