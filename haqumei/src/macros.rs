macro_rules! get_ptr {
    ($ptr:expr, $field:ident) => {
        {
            let p = $ptr;
            if p.is_null() {
                std::ptr::null_mut()
            } else {
                #[allow(unused_unsafe)]
                unsafe { (*p).$field }
            }
        }
    };
    ($ptr:expr, $field:ident $(, $rest:ident)+) => {
        {
            let p = $ptr;
            if p.is_null() {
                std::ptr::null_mut()
            } else {
                get_ptr!(unsafe { (*p).$field } $(, $rest)+)
            }
        }
    };
}

macro_rules! impl_batch_method_haqumei {
    (
        $(#[$meta:meta])*
        $batch_method:ident => $inner_method:ident $(($( $arg:ident : $arg_ty:ty),* ))? -> $ret_type:ty
    ) => {
        $(#[$meta])*
        ///
        #[doc = concat!(
            "複数のテキストに対して並行して `",
            stringify!($inner_method),
            "` を実行します。"
        )]
        pub fn $batch_method<S>(&mut self, texts: &[S], $( $($arg : $arg_ty),* )?) -> Result<Vec<$ret_type>, HaqumeiError>
        where
            S: AsRef<str> + Sync,
        {
                // インスタンスが辞書を持っている場合はそれを使う。
                // グローバル辞書を無条件に使うと、`from_path` や `from_dictionary` で
                // 指定した辞書が黙って無視されてしまう。
                let dict = match &self.open_jtalk.dict {
                    Some(dict) => dict.clone(),
                    None => GLOBAL_MECAB_DICTIONARY.load_full(),
                };
                if !dict.model.is_initialized() {
                    return Err(HaqumeiError::GlobalDictionaryNotInitialized);
                }
                let options = self.options;

                #[cfg(feature = "unidic-yomi")]
                let tokenizer = self.tokenizer.clone(); // かなり無料

                texts
                    .par_iter()
                    .map_init(
                    || {
                        let ojt = OpenJTalk::from_shared_dictionary(dict.clone())
                            .expect("Failed to initialize OpenJTalk worker");
                        Haqumei {
                            open_jtalk: ojt,
                            #[cfg(feature = "unidic-yomi")]
                            tokenizer: tokenizer.clone(),
                            #[cfg(feature = "unidic-yomi")]
                            rx: None,
                            options,
                        }
                    },
                    |haqumei, text| haqumei.$inner_method(text.as_ref(), $( $($arg),* )?),
                )
                .collect()
        }
    };
}

macro_rules! impl_batch_method_openjtalk {
    (
        $(#[$meta:meta])*
        $batch_method:ident => $inner_method:ident $(($( $arg:ident : $arg_ty:ty),* ))? -> $ret_type:ty
    ) => {
        $(#[$meta])*
        ///
        #[doc = concat!(
            "複数のテキストに対して並行して `",
            stringify!($inner_method),
            "` を実行します。"
        )]
        pub fn $batch_method<S>(&mut self, texts: &[S], $( $($arg : $arg_ty),* )?) -> Result<Vec<$ret_type>, HaqumeiError>
        where
            S: AsRef<str> + Sync,
        {
            // インスタンスが辞書を持っている場合はそれを使う。
            // グローバル辞書を無条件に使うと、`from_path` や `from_dictionary` で
            // 指定した辞書が黙って無視されてしまう。
            let dict = match &self.dict {
                Some(dict) => dict.clone(),
                None => GLOBAL_MECAB_DICTIONARY.load_full(),
            };
            if !dict.model.is_initialized() {
                return Err(HaqumeiError::GlobalDictionaryNotInitialized);
            }

            texts
            .par_iter()
            .map_init(
                || {
                    OpenJTalk::from_shared_dictionary(dict.clone())
                        .expect("Failed to initialize OpenJTalk worker")
                },
                |ojt, text| ojt.$inner_method(text.as_ref(), $( $($arg),* )?),
            )
            .collect()
        }
    };
}
