extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use syn::parse::{Parse, Parser};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput};

#[proc_macro_derive(AutoIndexed)]
pub fn derive_auto_indexed(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_identifier = &input.ident;
    match &input.data {
        Data::Struct(DataStruct { fields, .. }) => {
            quote! {
                #[automatically_derived]
                impl Indexed for #struct_identifier {
                    fn index(&self) -> usize {
                        *self.index
                    }
                }

                #[automatically_derived]
                impl From<#struct_identifier> for usize {
                    fn from(value: #struct_identifier) -> usize {
                        *value.index
                    }
                }

                #[automatically_derived]
                impl<'a> From<&'a #struct_identifier> for usize {
                    fn from(value: &'a #struct_identifier) -> usize {
                        *value.index
                    }
                }

                #[automatically_derived]
                impl From<#struct_identifier> for isize {
                    fn from(value: #struct_identifier) -> isize {
                        *value.index as isize
                    }
                }

                #[automatically_derived]
                impl<'a> From<&'a #struct_identifier> for isize {
                    fn from(value: &'a #struct_identifier) -> isize {
                        *value.index as isize
                    }
                }
            }
        }
        _ => unimplemented!()
    }.into()
}

#[proc_macro_derive(ManualIndexed)]
pub fn derive_manual_indexed(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_identifier = &input.ident;
    match &input.data {
        Data::Struct(DataStruct { fields, .. }) => {
            quote! {
                #[automatically_derived]
                impl Indexed for #struct_identifier {
                    fn index(&self) -> usize {
                        *self.index
                    }
                }

                #[automatically_derived]
                impl ManualIndexed for #struct_identifier {
                    fn indexed(&self) -> bool {
                        self.index.indexed()
                    }

                    fn set_index(&self, index: usize) {
                        self.index.set_index(index)
                    }
                }

                #[automatically_derived]
                impl From<#struct_identifier> for usize {
                    fn from(value: #struct_identifier) -> usize {
                        *value.index
                    }
                }

                #[automatically_derived]
                impl<'a> From<&'a #struct_identifier> for usize {
                    fn from(value: &'a #struct_identifier) -> usize {
                        *value.index
                    }
                }

                #[automatically_derived]
                impl From<#struct_identifier> for isize {
                    fn from(value: #struct_identifier) -> isize {
                        *value.index as isize
                    }
                }

                #[automatically_derived]
                impl<'a> From<&'a #struct_identifier> for isize {
                    fn from(value: &'a #struct_identifier) -> isize {
                        *value.index as isize
                    }
                }
            }
        }
        _ => unimplemented!()
    }.into()
}
