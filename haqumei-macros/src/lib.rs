use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{
    Ident, LitStr, Result, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

struct Entry {
    name: Ident,
    value: LitStr,
}

impl Parse for Entry {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }

        Ok(Self { name, value })
    }
}

struct Entries {
    items: Vec<Entry>,
}

impl Parse for Entries {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut items = Vec::new();

        while !input.is_empty() {
            items.push(input.parse()?);
        }

        Ok(Self { items })
    }
}

#[proc_macro]
pub fn phonemes(input: TokenStream) -> TokenStream {
    let Entries { items } = parse_macro_input!(input as Entries);

    if items.len() > 256 {
        return syn::Error::new_spanned(&items[255].name, "too many phonemes for repr(u8)")
            .to_compile_error()
            .into();
    }

    let mut seen = HashSet::new();

    for item in &items {
        let s = item.value.value();

        if !seen.insert(s.clone()) {
            return syn::Error::new_spanned(
                &item.value,
                format!("duplicate phoneme string: {s:?}"),
            )
            .to_compile_error()
            .into();
        }
    }

    let unk = match items.iter().find(|e| e.name == "Unk") {
        Some(e) => &e.name,
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "phonemes! requires `Unk = \"...\"`",
            )
            .to_compile_error()
            .into();
        }
    };

    let names = items.iter().map(|e| &e.name).collect::<Vec<_>>();
    let strs = items.iter().map(|e| &e.value).collect::<Vec<_>>();

    let expanded = quote! {
        #[repr(u8)]
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Phoneme {
            #( #names ),*
        }

        impl Phoneme {
            pub const ALL: &'static [Self] = &[
                #( Self::#names ),*
            ];

            pub const STRINGS: &'static [&'static str] = &[
                #( #strs ),*
            ];

            #[inline]
            pub const fn as_str(self) -> &'static str {
                match self {
                    #( Self::#names => #strs ),*
                }
            }

            /// u8 からの変換を試みる。対応するバリアントがなければ `None` を返す。
            #[inline]
            pub fn try_from_u8(v: u8) -> Option<Self> {
                match v {
                    #( x if x == Self::#names as u8 => Some(Self::#names) ),*,
                    _ => None,
                }
            }

            /// `*const c_char` からの変換を試みる。
            ///
            /// # Errors
            /// - `HaqumeiError::NullPhonemePtr` — ポインタが null
            /// - `HaqumeiError::InvalidPhonemeUtf8` — 無効な UTF-8
            /// - `HaqumeiError::UnknownPhoneme` — 未知の音素文字列
            ///
            /// # Safety
            /// `ptr` が有効な NUL 終端文字列を指していなければならない。
            #[inline]
            pub unsafe fn try_from_ptr(
                ptr: *const ::std::os::raw::c_char,
            ) -> ::std::result::Result<Self, crate::HaqumeiError> {
                if ptr.is_null() {
                    return Err(crate::HaqumeiError::NullPhonemePtr);
                }
                let s = unsafe { ::std::ffi::CStr::from_ptr(ptr) };
                let s = s.to_str().map_err(|_| crate::HaqumeiError::InvalidPhonemeUtf8)?;
                s.parse().map_err(|_| crate::HaqumeiError::UnknownPhoneme(s.to_owned()))
            }
        }

        impl ::core::fmt::Debug for Phoneme {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl ::core::str::FromStr for Phoneme {
            type Err = crate::HaqumeiError;

            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                match s {
                    #( #strs => Ok(Self::#names) ),*,
                    _ => Err(crate::HaqumeiError::UnknownPhoneme(s.to_owned())),
                }
            }
        }

        impl From<&str> for Phoneme {
            fn from(s: &str) -> Self {
                s.parse().unwrap_or(Self::#unk)
            }
        }

        impl From<u8> for Phoneme {
            fn from(v: u8) -> Self {
                Self::try_from_u8(v).unwrap_or(Self::#unk)
            }
        }

        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        impl From<*const ::std::os::raw::c_char> for Phoneme {
            fn from(ptr: *const ::std::os::raw::c_char) -> Self {
                unsafe { Self::try_from_ptr(ptr) }.unwrap_or(Self::#unk)
            }
        }

        impl From<*mut ::std::os::raw::c_char> for Phoneme {
            fn from(ptr: *mut ::std::os::raw::c_char) -> Self {
                unsafe { Self::try_from_ptr(ptr as *const _) }.unwrap_or(Self::#unk)
            }
        }

        impl From<Phoneme> for u8 {
            fn from(p: Phoneme) -> u8 {
                p as u8
            }
        }

        impl ::core::fmt::Display for Phoneme {
            fn fmt(
                &self,
                f: &mut ::core::fmt::Formatter<'_>,
            ) -> ::core::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl PartialEq<&str> for Phoneme {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<Phoneme> for &str {
            fn eq(&self, other: &Phoneme) -> bool {
                *self == other.as_str()
            }
        }

        impl PartialEq<String> for Phoneme {
            fn eq(&self, other: &String) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl PartialEq<Phoneme> for String {
            fn eq(&self, other: &Phoneme) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl ::core::convert::AsRef<str> for Phoneme {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl ::core::borrow::Borrow<str> for Phoneme {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        impl ::core::borrow::Borrow<str> for &Phoneme {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        #[cfg(feature = "serde")]
        impl ::serde::Serialize for Phoneme {
            fn serialize<S>(
                &self,
                serializer: S,
            ) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> ::serde::Deserialize<'de> for Phoneme {
            fn deserialize<D>(
                deserializer: D,
            ) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                let s = <&str>::deserialize(deserializer)?;
                s.parse().map_err(::serde::de::Error::custom)
            }
        }

        pub trait PhonemeVecExt {
            fn into_strs(self) -> ::std::vec::Vec<&'static str>;
            fn to_strs(&self) -> ::std::vec::Vec<&'static str>;
            fn into_strings(self) -> ::std::vec::Vec<::std::string::String>;
            fn to_strings(&self) -> ::std::vec::Vec<::std::string::String>;
        }

        impl PhonemeVecExt for ::std::vec::Vec<Phoneme> {
            #[inline]
            fn into_strs(self) -> ::std::vec::Vec<&'static str> {
                self.into_iter().map(|p| p.as_str()).collect()
            }

            #[inline]
            fn to_strs(&self) -> ::std::vec::Vec<&'static str> {
                self.iter().map(|p| p.as_str()).collect()
            }

            #[inline]
            fn into_strings(self) -> ::std::vec::Vec<::std::string::String> {
                self.into_iter().map(|p| p.as_str().to_owned()).collect()
            }

            #[inline]
            fn to_strings(&self) -> ::std::vec::Vec<::std::string::String> {
                self.iter().map(|p| p.as_str().to_owned()).collect()
            }
        }

        pub trait PhonemeVecVecExt {
            fn into_strs(self) -> ::std::vec::Vec<::std::vec::Vec<&'static str>>;
            fn to_strs(&self) -> ::std::vec::Vec<::std::vec::Vec<&'static str>>;
            fn into_strings(self) -> ::std::vec::Vec<::std::vec::Vec<::std::string::String>>;
            fn to_strings(&self) -> ::std::vec::Vec<::std::vec::Vec<::std::string::String>>;
        }

        impl PhonemeVecVecExt for ::std::vec::Vec<::std::vec::Vec<Phoneme>> {
            #[inline]
            fn into_strs(self) -> ::std::vec::Vec<::std::vec::Vec<&'static str>> {
                self.into_iter()
                    .map(|inner_vec| inner_vec.into_strs())
                    .collect()
            }

            #[inline]
            fn to_strs(&self) -> ::std::vec::Vec<::std::vec::Vec<&'static str>> {
                self.iter()
                    .map(|inner_vec| inner_vec.to_strs())
                    .collect()
            }

            #[inline]
            fn into_strings(self) -> ::std::vec::Vec<::std::vec::Vec<::std::string::String>> {
                self.into_iter()
                    .map(|inner_vec| inner_vec.into_strings())
                    .collect()
            }

            #[inline]
            fn to_strings(&self) -> ::std::vec::Vec<::std::vec::Vec<::std::string::String>> {
                self.iter()
                    .map(|inner_vec| inner_vec.to_strings())
                    .collect()
            }
        }
    };

    expanded.into()
}
