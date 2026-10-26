use haqumei::HaqumeiOptions;

#[test]
fn default_prediction_options_match_the_available_feature() {
    let options = HaqumeiOptions::default();
    assert_eq!(options.predict_nani, cfg!(feature = "inference"));
    assert_eq!(options.predict_kana_english, cfg!(feature = "inference"));
}

#[cfg(not(feature = "embed-dictionary"))]
#[test]
fn constructors_report_an_uninitialized_global_dictionary() {
    use haqumei::{Haqumei, OpenJTalk, errors::HaqumeiError};

    assert!(matches!(
        OpenJTalk::new(),
        Err(HaqumeiError::GlobalDictionaryNotInitialized)
    ));
    assert!(matches!(
        Haqumei::new(),
        Err(HaqumeiError::GlobalDictionaryNotInitialized)
    ));
}

#[cfg(all(feature = "embed-dictionary", not(feature = "inference")))]
#[test]
fn unavailable_predictions_are_rejected_without_changing_valid_options() {
    use haqumei::{Haqumei, errors::HaqumeiError};

    let mut engine = Haqumei::new().unwrap();
    for option in ["predict_nani", "predict_kana_english"] {
        let mut options = HaqumeiOptions::default();
        if option == "predict_nani" {
            options.predict_nani = true;
        } else {
            options.predict_kana_english = true;
        }
        let is_missing_inference = |error| {
            matches!(error, HaqumeiError::FeatureDisabled {
                option: actual,
                feature: "inference",
            } if actual == option)
        };
        assert!(Haqumei::with_options(options).is_err_and(is_missing_inference));
        assert!(engine.set_options(options).is_err_and(is_missing_inference));
        assert!(!engine.options.predict_nani);
        assert!(!engine.options.predict_kana_english);

        engine.options = options;
        assert!(
            engine
                .run_frontend("こんにちは")
                .is_err_and(is_missing_inference)
        );
        assert!(
            engine
                .g2p_candidates("こんにちは")
                .is_err_and(is_missing_inference)
        );
        assert!(
            engine
                .g2k_batch(&["こんにちは"])
                .is_err_and(is_missing_inference)
        );
        engine.set_options(HaqumeiOptions::default()).unwrap();
        assert_eq!(engine.g2k("こんにちは").unwrap(), "コンニチワ");
    }
}
