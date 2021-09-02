use darling::{FromDeriveInput, FromMeta};
use quote::quote;
use syn::DeriveInput;

mod de;
mod ser;
mod util;

#[derive(Debug, Clone, FromMeta)]
#[darling(default)]
enum EncodingType {
    Basic, // considering removing Basic
    List,
    Map,
}

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(amqp_contract))]
struct DescribedAttr {
    #[darling(default)]
    pub name: Option<String>,
    #[darling(default)]
    pub code: Option<u64>,
    #[darling(default)]
    pub encoding: Option<EncodingType>,
    #[darling(default)]
    pub rename_all: String,
    #[darling(default)]
    pub no_descriptor: Option<()>,
}

struct DescribedStructAttr {
    name: String,
    code: Option<u64>,
    encoding: EncodingType,
    rename_field: String,
}

struct DescribedEnumAttr {
    name: String,
    code: Option<u64>,
    no_descriptor: bool,
}

#[proc_macro_derive(SerializeComposite, attributes(amqp_contract))]
pub fn derive_serialize_described(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as DeriveInput);
    let impl_ser = ser::expand_serialize(&input).unwrap();
    let output = quote! {
        const _: () = {
            #impl_ser
        };
    };
    output.into()
}

#[proc_macro_derive(DeserializeComposite, attributes(amqp_contract))]
pub fn derive_deserialize_described(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as DeriveInput);
    let impl_de = de::expand_deserialize(&input).unwrap();
    let output = quote! {
        const _:() = {
            #impl_de
        };
    };
    output.into()
}
