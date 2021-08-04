#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[macro_use]
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{
    parse::{Error, Parse, ParseStream, Result},
    punctuated::Punctuated,
    Ident, Type,
};

struct EventRegister {
    name: Ident,
    args: Vec<Type>,
    once: bool,
}

impl Parse for EventRegister {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![=>]>()?;

        let once = match input.parse::<Ident>() {
            Ok(id) if id == "once" => true,
            Ok(other) => {
                return Err(
                    Error::new(
                        other.span(),
                        format!(
                            "Token here must be `once` or nothing, found `{}`. `once` here means this event can only be call once.",
                            other.to_string()
                        )
                    )
                );
            }
            Err(_) => false,
        };

        input.parse::<Token![|]>()?;
        let args = Punctuated::<Type, Token![,]>::parse_separated_nonempty(input)?
            .into_iter()
            .collect();
        input.parse::<Token![|]>()?;

        Ok(EventRegister { name, args, once })
    }
}

/// ```
/// event_target!{
///     pub struct EventTarget{
///         event1=>|String,u32|,
///         event2=>|bool|,
///         event3=>||
///     }
/// }
/// ```
///
#[proc_macro]
pub fn event_target(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::parse::ParseStream);

    //是否为pub struct
    let public = input.parse::<Token![pub]>().ok();

    input.parse::<Token![struct]>();

    //匹配结构名
    let name: Ident = input.parse().expect("You needs a struct name");

    //匹配花括号
    let content;
    braced!(content in input);

    //匹配所有事件
    let events: Vec<EventRegister> = Punctuated::<_, Token![,]>::parse_terminated(input)
        .unwrap()
        .into_iter()
        .collect();

    quote! {
        #public struct #name{

        }
    }
}
