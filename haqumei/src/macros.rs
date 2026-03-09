macro_rules! impl_batch_method_haqumei {
    (
        $(#[$meta:meta])*
        $batch_method:ident => $inner_method:ident -> $ret_type:ty
    ) => {
        $(#[$meta])*
        ///
        /// `HaqumeiOptions` の `modify_kanji_yomi` が無効な場合、マルチスレッドで処理を行います。
        /// 有効な場合は、シングルスレッドでの逐次処理にフォールバックします。
        pub fn $batch_method<S>(&mut self, texts: &[S]) -> Result<Vec<$ret_type>, HaqumeiError>
        where
            S: AsRef<str> + Sync,
        {
            if !self.options.modify_kanji_yomi {
                let dict = GLOBAL_MECAB_DICTIONARY.load_full();
                if !dict.model.is_initialized() {
                    return Err(HaqumeiError::GlobalDictionaryNotInitialized);
                }
                let options = self.options;

                return texts
                    .par_iter()
                    .map_init(
                        || {
                            let ojt = OpenJTalk::from_shared_dictionary(dict.clone())
                                .expect("Failed to initialize OpenJTalk worker");
                            Haqumei::from_open_jtalk(ojt, options).unwrap()
                        },
                        |haqumei, text| haqumei.$inner_method(text.as_ref()),
                    )
                    .collect();
            }

            texts.iter().map(|text| self.$inner_method(text.as_ref())).collect()
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
            stringify!($batch_method),
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
