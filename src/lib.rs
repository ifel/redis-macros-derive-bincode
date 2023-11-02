use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, token, Attribute, DeriveInput, GenericParam, Result,
};

struct ParseParenthesed {
    _p: token::Paren,
    field: TokenStream2,
}

impl Parse for ParseParenthesed {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(ParseParenthesed {
            _p: parenthesized!(content in input),
            field: content.parse()?,
        })
    }
}

fn get_serializer(attrs: Vec<Attribute>, default: &str) -> TokenStream2 {
    let default_token = default.parse::<TokenStream2>().unwrap();
    attrs
        .into_iter()
        .find(|a| a.path.segments.len() == 1 && a.path.segments[0].ident == "redis_serializer")
        .map(|Attribute { tokens, .. }| {
            let tokens = tokens.into();
            let ParseParenthesed { field, .. } = parse_macro_input!(tokens as ParseParenthesed);
            field.into()
        })
        .unwrap_or(default_token.into())
        .into()
}

/// Derive macro for the redis crate's [`FromRedisValue`](../redis/trait.FromRedisValue.html) trait to allow parsing Redis responses to this type.
///
/// *NOTE: This trait requires serde's [`Deserialize`](../serde/trait.Deserialize.html) to also be derived (or implemented).*
///
/// Simply use the `#[derive(FromRedisValue, Deserialize)]` before any structs (or serializable elements).
/// This allows, when using Redis commands, to set this as the return type and deserialize from binary data serialized with bincode automatically, while reading from Redis.
///
/// ```rust,no_run
/// # use redis::{Client, Commands, RedisResult};
/// use redis_macros_derive_bincode::{ToRedisArgs, FromRedisValue};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, FromRedisValue, ToRedisArgs, Debug)]
/// struct Test {
///     first_field: String,
///     second_field: i64,
///     third_field: f32,
/// }
///  
/// fn main () -> redis::RedisResult<()> {
/// let client = redis::Client::open("redis://localhost:6379/")?;
/// let mut con = client.get_connection()?;
/// let test = Test {
/// first_field: "Hello".to_string(),
/// second_field: 42,
/// third_field: 12.3,
/// };
/// con.set("test", &test)?;  ## TODO
/// let test1 = con.get("test")?;  // => Test { first_field: "Hello", second_field: 42, third_field: 12.3 }
/// Ok(())
/// }
/// ```
/// Data stored in Redis:
/// 127.0.0.1:6379> get "test"
/// "\x05\x00\x00\x00\x00\x00\x00\x00Hello*\x00\x00\x00\x00\x00\x00\x00\xcd\xccDA"
///
/// If you want to use a different serde format, you can set this with the `redis_serializer` attribute.
/// The only restriction is to have the deserializer implement the `deserialize` function.
///
/// ```rust,no_run
/// use redis_macros_derive_bincode::{FromRedisValue};
/// use serde::{Deserialize};
///
/// #[derive(FromRedisValue, Deserialize)]
/// #[redis_serializer(my_serializer)]
/// struct Test {
/// ```
///
/// For more information see the isomorphic pair of this trait: [ToRedisArgs].
#[proc_macro_derive(FromRedisValue, attributes(redis_serializer))]
pub fn from_redis_value_macro(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        attrs,
        generics,
        ..
    } = parse_macro_input!(input as DeriveInput);
    let serializer = get_serializer(attrs, "bincode");
    let ident_str = format!("{}", ident);
    let serializer_str = format!("{}", serializer);

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let has_types = generics
        .params
        .iter()
        .any(|g| matches!(g, GenericParam::Type(_)));

    let where_with_serialize = if let Some(w) = where_clause {
        quote! { #w, #ident #ty_generics : serde::de::DeserializeOwned }
    } else if has_types {
        quote! { where #ident #ty_generics : serde::de::DeserializeOwned }
    } else {
        quote! {}
    };

    let failed_parse_error = quote! {
        Err(redis::RedisError::from((
            redis::ErrorKind::TypeError,
            "Response was of incompatible type",
            format!("Response type not deserializable to {} with {}. (response was {:?})", #ident_str, #serializer_str, v)
        )))
    };

    quote! {
        impl #impl_generics redis::FromRedisValue for #ident #ty_generics #where_with_serialize {
            fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
                match *v {
                    redis::Value::Data(ref bytes) => {
                        if let Ok(s) = #serializer::deserialize(bytes) {
                            Ok(s)
                        } else {
                            #failed_parse_error
                        }
                    },
                    _ => Err(redis::RedisError::from((
                        redis::ErrorKind::TypeError,
                        "Response was of incompatible type",
                        format!("Response type was not deserializable to {}. (response was {:?})", #ident_str, v)
                    ))),
                }
            }
        }
    }
    .into()
}

/// Derive macro for the redis crate's [`ToRedisArgs`](../redis/trait.ToRedisArgs.html) trait to allow passing the type to Redis commands.
///
/// *NOTE: This trait requires serde's [`Serialize`](../serde/trait.Serialize.html) to also be derived (or implemented).*
///
/// ***WARNING: This trait panics if the underlying serialization fails.***
///
/// Simply use the `#[derive(ToRedisArgs, Serialize)]` before any structs (or serializable elements).
/// This allows to pass this type to Redis commands like SET. The type will be serialized into binary automatically while saving to Redis.
///
/// ```rust,no_run
/// # use redis::{Client, Commands, RedisResult};
/// use redis_macros_derive_bincode::{ToRedisArgs, FromRedisValue};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, FromRedisValue, ToRedisArgs, Debug)]
/// struct Test {
///     first_field: String,
///     second_field: i64,
///     third_field: f32,
/// }
///  
/// fn main () -> redis::RedisResult<()> {
/// let client = redis::Client::open("redis://localhost:6379/")?;
/// let mut con = client.get_connection()?;
/// let test = Test {
/// first_field: "Hello".to_string(),
/// second_field: 42,
/// third_field: 12.3,
/// };
/// con.set("test", &test)?;  ## TODO
/// let test1 = con.get("test")?;  // => Test { first_field: "Hello", second_field: 42, third_field: 12.3 }
/// Ok(())
/// }
/// ```
///
/// If you want to use a different serde format, you can set this with the `redis_serializer` attribute.
/// The only restriciton is to have the serializer implement the `serialize` function.
///
/// ```rust,no_run
/// # use redis::{Client, Commands, RedisResult};
/// use redis_macros_derive_bincode::{ToRedisArgs};
/// use serde::{Serialize};
///
/// #[derive(ToRedisArgs, Serialize)]
/// #[redis_serializer(my_serializer)]
/// struct Test{
/// ```
///
/// For more information see the isomorphic pair of this trait: [FromRedisValue].
#[proc_macro_derive(ToRedisArgs, attributes(redis_serializer))]
pub fn to_redis_args_macro(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        attrs,
        generics,
        ..
    } = parse_macro_input!(input as DeriveInput);
    let serializer = get_serializer(attrs, "bincode");

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let has_types = generics
        .params
        .iter()
        .any(|g| matches!(g, GenericParam::Type(_)));

    let where_with_serialize = if let Some(w) = where_clause {
        quote! { #w, #ident #ty_generics : serde::Serialize }
    } else if has_types {
        quote! { where #ident #ty_generics : serde::Serialize }
    } else {
        quote! {}
    };

    quote! {
        impl #impl_generics redis::ToRedisArgs for #ident #ty_generics #where_with_serialize {
            fn write_redis_args<W>(&self, out: &mut W)
            where
                W: ?Sized + redis::RedisWrite,
            {
                let buf = #serializer::serialize(&self).unwrap();
                return out.write_arg(&buf);
            }
        }
    }
    .into()
}
