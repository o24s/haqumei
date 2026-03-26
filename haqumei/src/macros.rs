macro_rules! impl_batch_method_haqumei {
    (
        $(#[$meta:meta])*
        $batch_method:ident => $inner_method:ident -> $ret_type:ty
    ) => {
        $(#[$meta])*
        ///
        #[doc = concat!(
            "複数のテキストに対して並行して `",
            stringify!($inner_method),
            "` を実行します。"
        )]
        pub fn $batch_method<S>(&mut self, texts: &[S]) -> Result<Vec<$ret_type>, HaqumeiError>
        where
            S: AsRef<str> + Sync,
        {
                let dict = GLOBAL_MECAB_DICTIONARY.load_full();
                if !dict.model.is_initialized() {
                    return Err(HaqumeiError::GlobalDictionaryNotInitialized);
                }
                let options = self.options;
                let tokenizer = self.tokenizer.clone(); // かなり無料

                texts
                    .par_iter()
                    .map_init(
                    || {
                        let ojt = OpenJTalk::from_shared_dictionary(dict.clone())
                            .expect("Failed to initialize OpenJTalk worker");
                        Haqumei {
                            open_jtalk: ojt,
                            tokenizer: tokenizer.clone(),
                            rx: None,
                            options,
                        }
                    },
                    |haqumei, text| haqumei.$inner_method(text.as_ref()),
                )
                .collect()
        }
    };
}

macro_rules! impl_batch_method_openjtalk {
    (
        $(#[$meta:meta])*
        $batch_method:ident => $inner_method:ident -> $ret_type:ty
    ) => {
        $(#[$meta])*
        ///
        #[doc = concat!(
            "複数のテキストに対して並行して `",
            stringify!($inner_method),
            "` を実行します。"
        )]
        pub fn $batch_method<S>(&mut self, texts: &[S]) -> Result<Vec<$ret_type>, HaqumeiError>
        where
            S: AsRef<str> + Sync,
        {
            let dict = GLOBAL_MECAB_DICTIONARY.load_full();
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
                |ojt, text| ojt.$inner_method(text.as_ref()),
            )
            .collect()
        }
    };
}
