#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[macro_use]
extern crate syn;

use proc_macro::TokenStream;

/// ```
/// event_target!{
///     struct EventTarget{
///         event1=>|String,u32|,
///         event2=>|bool|,
///         event3=>||
///     }
/// }
/// ```
///
#[proc_macro]
pub fn event_target(input:TokenStream)->TokenStream{
    let input=parse_macro_input!(input);
    input.parse::<Token![struct]>();
    //匹配结构名
    let name:syn::Ident=input.parse();

    //匹配花括号
    let content;
    braced!(content in input);

    //进入花括号内
    
    custom_punctuation!(EventRegisterExpr,)
    let mut events=Vec::<(syn::Ident,Vec<syn::Type>)>::new();
}
