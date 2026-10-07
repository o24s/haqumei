use anyhow::{Context, Result};
use clap::{Args, Parser, ValueEnum};
use haqumei::{
    Haqumei, HaqumeiOptions, IpaBoundary, IpaToken, IpaTokenProsody, IuPronunciation,
    NumberReading, PitchAccent, ProsodicIpa, ProsodyFormat, UnicodeNormalization,
};
use std::fs::File;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// 処理する入力テキスト。
    /// `--input` と同時に指定することはできません。
    #[arg(value_name = "TEXT", conflicts_with = "input")]
    text: Option<String>,

    /// 入力ファイルへのパス。
    /// 指定がない場合は、引数 `TEXT` または標準入力から読み取ります。
    #[arg(short, long, value_name = "FILE")]
    input: Option<PathBuf>,

    /// 出力ファイルへのパス。指定がない場合は標準出力 (stdout) へ出力します。
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// 出力モード
    #[arg(short, long, value_enum, default_value_t = OutputMode::G2p)]
    mode: OutputMode,

    /// 出力フォーマット
    #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// プロソディ出力フォーマット
    /// (mode が prosody、mapping-prosody、ipa-prosody の場合に有効)
    #[arg(long, value_enum, default_value_t = CliProsodyFormat::Default)]
    prosody_format: CliProsodyFormat,

    /// 詳細なログ (OpenJTalk の警告など) を表示します。
    #[arg(short, long)]
    verbose: bool,

    #[command(flatten)]
    dict: DictArgs,

    #[command(flatten)]
    options: HaqumeiConfigArgs,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum OutputMode {
    /// 音素列 (フラット)
    G2p,
    /// 異音解決オプションに依存しない IPA の広い音声表記
    #[value(aliases = ["g2ipa", "ipa-mapping", "g2ipa-mapping"])]
    Ipa,
    /// ピッチアクセントと韻律境界を含む IPA の広い音声表記
    #[value(alias = "g2ipa-prosody")]
    IpaProsody,
    /// プロソディ記号付き音素列
    Prosody,
    /// 詳細な音素列 (記号等は sp, unk などに変換)
    G2pDetailed,
    /// カタカナ
    Kana,
    /// 単語(形態素)ごとのカタカナ
    KanaPerWord,
    /// 単語ごとの音素リスト
    PerWord,
    /// 形態素ごとの未知語情報を含めたマッピング
    Mapping,
    /// 未知語情報や NJD の詳細な特徴量を含めたマッピング
    MappingDetailed,
    /// 形態素ごとの詳細なプロソディ情報を含めたマッピング
    MappingProsody,
    /// 読みが分かれる箇所の候補
    Candidates,
    /// フルコンテキストラベル
    Fullcontext,
    /// フルコンテキストラベル文字列
    FullcontextString,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum OutputFormat {
    /// 人間が読みやすいテキスト形式
    Text,
    /// 構造化された JSON (JSON Lines) 形式
    Json,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum CliNumberReading {
    /// 十・百・千などの位を付けて読む。
    Cardinal,
    /// 0をゼロとして桁ごとに読む。
    Digits,
    /// 0をマルとして桁ごとに読む。
    DigitsWithMaru,
}

impl From<CliNumberReading> for NumberReading {
    fn from(value: CliNumberReading) -> Self {
        match value {
            CliNumberReading::Cardinal => Self::Cardinal,
            CliNumberReading::Digits => Self::Digits,
            CliNumberReading::DigitsWithMaru => Self::DigitsWithMaru,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum CliProsodyFormat {
    Default,
    Prefix,
    Numeric,
}

impl From<CliProsodyFormat> for ProsodyFormat {
    fn from(format: CliProsodyFormat) -> Self {
        match format {
            CliProsodyFormat::Default => ProsodyFormat::Default,
            CliProsodyFormat::Prefix => ProsodyFormat::Prefix,
            CliProsodyFormat::Numeric => ProsodyFormat::Numeric,
        }
    }
}

#[derive(Args, Debug)]
struct DictArgs {
    /// 辞書ディレクトリのパス (指定しない場合は組み込み辞書を使用)
    #[arg(long, value_name = "DIR")]
    dict_dir: Option<PathBuf>,

    /// ユーザー辞書のパス。複数回指定できる
    #[arg(long, value_name = "FILE")]
    user_dict: Vec<PathBuf>,
}

#[derive(Args, Debug)]
struct HaqumeiConfigArgs {
    /// Unicode正規化の方法を指定
    #[arg(long, value_enum, default_value_t = UnicodeNorm::None)]
    normalize_unicode: UnicodeNorm,

    /// 「言う」の発音正規化方式を指定する
    #[arg(long, value_enum)]
    normalize_iu: Option<IuPronMode>,

    /// 読み (read) を発音 (pron) の代わりに使用し、長音の自動変換などを無効化する
    #[arg(long)]
    use_read_as_pron: bool,

    /// 辞書によって自動的に長音化された発音を、元のテキストに忠実な読みに復元する
    #[arg(long)]
    revert_long_vowels: bool,

    /// 四つ仮名 (ヅ・ヂ) を元のテキスト通りの表記に復元する
    #[arg(long)]
    revert_yotsugana: bool,

    /// フィラーのアクセント修正を無効にする (デフォルトは有効)
    #[arg(long)]
    no_modify_filler_accent: bool,

    /// Nani Predictor による「何」の読み修正を無効にする (デフォルトは有効)
    #[arg(long)]
    no_predict_nani: bool,

    /// Kanalizer を使って、英語の読み予測を無効にする (デフォルトは有効)
    #[arg(long)]
    no_predict_kana_english: bool,

    /// 隣接する形態素で読みが決まる同形異音語の補正を無効にする (デフォルトは有効)
    #[arg(long)]
    no_modify_context_reading: bool,

    /// 旧国名に続く接尾辞「国」を「ノクニ」と読む補正を無効にする (デフォルトは有効)
    #[arg(long)]
    no_modify_old_province_yomi: bool,

    /// ユーザー辞書が与えた読みを後段の補正から守る (デフォルトは無効)
    #[arg(long)]
    protect_user_dict_readings: bool,

    /// ユーザー辞書に登録した核のアクセント後退を抑える (デフォルトは無効)
    #[arg(long)]
    protect_user_dict_accents: bool,

    /// 外来語表記の仮名 (ヴィ / テュ など) の復元を無効にする (デフォルトは有効)
    #[arg(long)]
    no_restore_loanword_kana: bool,

    /// CJKVI の字体対応による未知語の再解析を無効にする (デフォルトは有効)
    #[arg(long)]
    no_resolve_kanji_variants: bool,

    /// 顔文字の読みを省略する処理を無効にする
    #[arg(long)]
    no_ignore_kaomoji: bool,

    /// ローマ数字の数詞処理を無効にする。
    #[arg(long)]
    no_resolve_roman_numerals: bool,

    /// 号室・号線・型番の番号読みを無効にする。
    #[arg(long)]
    no_resolve_number_identifiers: bool,

    /// 号室の数字列の読み方。digitsでは0をゼロと読む。
    #[arg(long, value_enum, default_value_t = CliNumberReading::DigitsWithMaru)]
    room_number_reading: CliNumberReading,

    /// 号室の番号の先頭に続く0を読み飛ばす。すべて0なら1桁だけ残す。
    #[arg(long)]
    skip_room_number_leading_zeros: bool,

    /// 号線の数字列の読み方。
    #[arg(long, value_enum, default_value_t = CliNumberReading::Cardinal)]
    route_number_reading: CliNumberReading,

    /// 型番の数字列の読み方。
    #[arg(long, value_enum, default_value_t = CliNumberReading::Cardinal)]
    model_number_reading: CliNumberReading,

    /// 辞書に無い漢字へのフォールバック読みを無効にする (デフォルトは有効)
    #[arg(long)]
    no_read_unknown_kanji: bool,

    /// 数詞まわりの読みの補正を無効にする (デフォルトは有効)
    #[arg(long)]
    no_modify_numeral_reading: bool,

    /// 指示的な漢語接頭辞 (本, 当, 同, 全) の後のアクセント句の切れ目を無効にする (デフォルトは有効)
    #[arg(long)]
    no_split_prefix_accent_phrase: bool,

    /// アクセント核を1つ前のモーラにずらすルールを無効にする (デフォルトは有効)
    #[arg(long)]
    no_retreat_acc_nuc: bool,

    /// 品詞「特殊・マス」前のアクセント移動を無効にする (デフォルトは有効)
    #[arg(long)]
    no_modify_acc_after_chaining: bool,

    /// 踊り字 (々, ヽ, ヾ) の展開を無効にする (デフォルトは有効)
    #[arg(long)]
    no_process_odoriji: bool,

    /// 異音解決 (split_n_allophones, split_q_allophones, enable_final_glottal_stop) を一括で有効化する
    #[arg(long)]
    use_allophones: bool,

    /// 撥音「ン」を後続音素の環境に応じて異音 (Nm, Ng, Nd, Nq) に分岐させる
    #[arg(long)]
    split_n_allophones: bool,

    /// r/ry の前の撥音「ン」を専用ラベル Nr に分ける
    /// (--split-n-allophones または --use-allophones が必要)
    #[arg(long)]
    split_n_before_r: bool,

    /// ch, j の前の撥音「ン」をさらに専用の Npl `[ɲ]` (硬口蓋鼻音) に解決する
    /// (--split-n-allophones または --use-allophones が必要)
    #[arg(long)]
    split_n_before_palatal_affricate: bool,

    /// 語中の促音「ッ」を後続音素の環境に応じて異音 (ClP, ClT, ClK, ClS, ClV) に分岐させる
    #[arg(long)]
    split_q_allophones: bool,

    /// 語末やポーズ前における、後続子音を伴わない促音「ッ」を専用の声門閉鎖音 ClQ `[ʔ]` として出力する
    #[arg(long)]
    enable_final_glottal_stop: bool,
}

#[derive(ValueEnum, Clone, Debug)]
enum UnicodeNorm {
    None,
    Nfc,
    Nfkc,
}

impl From<UnicodeNorm> for UnicodeNormalization {
    fn from(norm: UnicodeNorm) -> Self {
        match norm {
            UnicodeNorm::None => UnicodeNormalization::None,
            UnicodeNorm::Nfc => UnicodeNormalization::Nfc,
            UnicodeNorm::Nfkc => UnicodeNormalization::Nfkc,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum IuPronMode {
    Iu,
    Yuu,
    KanjiIu,
    KanjiYuu,
    YuuBase,
    KanjiYuuBase,
}

impl From<IuPronMode> for IuPronunciation {
    fn from(mode: IuPronMode) -> Self {
        match mode {
            IuPronMode::Iu => IuPronunciation::Iu,
            IuPronMode::Yuu => IuPronunciation::Yuu,
            IuPronMode::KanjiIu => IuPronunciation::KanjiIu,
            IuPronMode::KanjiYuu => IuPronunciation::KanjiYuu,
            IuPronMode::YuuBase => IuPronunciation::YuuBase,
            IuPronMode::KanjiYuuBase => IuPronunciation::KanjiYuuBase,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let default_log_level = if cli.verbose { "info" } else { "error" };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_log_level))
        .target(env_logger::Target::Stderr)
        .init();

    let haqumei_options = HaqumeiOptions {
        normalize_unicode: cli.options.normalize_unicode.into(),
        normalize_iu: cli.options.normalize_iu.map(Into::into),
        ignore_kaomoji: !cli.options.no_ignore_kaomoji,
        resolve_roman_numerals: !cli.options.no_resolve_roman_numerals,
        resolve_number_identifiers: !cli.options.no_resolve_number_identifiers,
        room_number_reading: cli.options.room_number_reading.into(),
        skip_room_number_leading_zeros: cli.options.skip_room_number_leading_zeros,
        route_number_reading: cli.options.route_number_reading.into(),
        model_number_reading: cli.options.model_number_reading.into(),
        use_read_as_pron: cli.options.use_read_as_pron,
        revert_long_vowels: cli.options.revert_long_vowels,
        revert_yotsugana: cli.options.revert_yotsugana,
        modify_filler_accent: !cli.options.no_modify_filler_accent,
        predict_nani: !cli.options.no_predict_nani,
        predict_kana_english: !cli.options.no_predict_kana_english,
        modify_context_reading: !cli.options.no_modify_context_reading,
        modify_old_province_yomi: !cli.options.no_modify_old_province_yomi,
        restore_loanword_kana: !cli.options.no_restore_loanword_kana,
        protect_user_dict_readings: cli.options.protect_user_dict_readings,
        protect_user_dict_accents: cli.options.protect_user_dict_accents,
        resolve_kanji_variants: !cli.options.no_resolve_kanji_variants,
        read_unknown_kanji: !cli.options.no_read_unknown_kanji,
        modify_numeral_reading: !cli.options.no_modify_numeral_reading,
        split_prefix_accent_phrase: !cli.options.no_split_prefix_accent_phrase,
        retreat_acc_nuc: !cli.options.no_retreat_acc_nuc,
        modify_acc_after_chaining: !cli.options.no_modify_acc_after_chaining,
        process_odoriji: !cli.options.no_process_odoriji,
        use_allophones: cli.options.use_allophones,
        split_n_allophones: cli.options.split_n_allophones,
        split_n_before_r: cli.options.split_n_before_r,
        split_n_before_palatal_affricate: cli.options.split_n_before_palatal_affricate,
        split_q_allophones: cli.options.split_q_allophones,
        enable_final_glottal_stop: cli.options.enable_final_glottal_stop,
        ..Default::default()
    };

    let mut haqumei = match (cli.dict.dict_dir, cli.dict.user_dict) {
        (Some(dict), user_dicts) if !user_dicts.is_empty() => {
            Haqumei::from_paths(dict, &user_dicts, haqumei_options)
                .context("Failed to load dictionary and user dictionary")?
        }
        (Some(dict), _) => {
            Haqumei::from_path(dict, haqumei_options).context("Failed to load custom dictionary")?
        }
        _ => Haqumei::with_options(haqumei_options)
            .context("Failed to initialize with built-in dictionary")?,
    };

    let mut writer: Box<dyn Write> = match cli.output {
        Some(path) => {
            let file = File::create(&path)
                .with_context(|| format!("Failed to create output file: {:?}", path))?;
            Box::new(io::BufWriter::new(file))
        }
        None => Box::new(io::BufWriter::new(io::stdout())),
    };

    let prosody_format: ProsodyFormat = cli.prosody_format.into();

    if let Some(text) = cli.text.as_deref() {
        process_batch(
            &mut haqumei,
            &[text.to_string()],
            &cli.mode,
            &cli.format,
            prosody_format,
            &mut writer,
        )?;
    } else if let Some(input_path) = cli.input.as_ref() {
        let file = File::open(input_path)
            .with_context(|| format!("Failed to open input file: {:?}", input_path))?;
        let reader = io::BufReader::new(file);
        process_input(
            reader,
            &mut haqumei,
            &cli.mode,
            &cli.format,
            prosody_format,
            &mut writer,
        )?;
    } else {
        let stdin = io::stdin();
        let stdout = io::stdout();

        let is_repl = stdin.is_terminal() && stdout.is_terminal();

        if is_repl {
            eprintln!("Enter text to process (Ctrl+C or Ctrl+D to exit):");
            loop {
                eprint!("> ");
                io::stderr().flush()?;

                let mut line = String::new();
                let bytes = stdin.read_line(&mut line)?;
                if bytes == 0 {
                    break; // EOF
                }

                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                process_batch(
                    &mut haqumei,
                    &[trimmed.to_string()],
                    &cli.mode,
                    &cli.format,
                    prosody_format,
                    &mut writer,
                )?;

                writer.flush()?;
            }
        } else {
            let reader = stdin.lock();
            process_input(
                reader,
                &mut haqumei,
                &cli.mode,
                &cli.format,
                prosody_format,
                &mut writer,
            )?;
        }
    }

    writer.flush()?;
    Ok(())
}

fn process_input<R: BufRead>(
    reader: R,
    haqumei: &mut Haqumei,
    mode: &OutputMode,
    format: &OutputFormat,
    prosody_format: ProsodyFormat,
    writer: &mut dyn Write,
) -> Result<()> {
    let texts: Result<Vec<String>, _> = reader.lines().collect();
    let texts = texts.context("Failed to read input")?;

    if texts.is_empty() {
        return Ok(());
    }

    process_batch(haqumei, &texts, mode, format, prosody_format, writer)?;
    Ok(())
}

#[inline(always)]
fn write_json<T: serde::Serialize>(writer: &mut dyn Write, data: &T) -> Result<()> {
    serde_json::to_writer(&mut *writer, data)?;
    writeln!(writer)?;
    Ok(())
}

fn format_ipa_token(token: &IpaToken) -> String {
    token.to_string()
}

fn format_ipa_boundary(boundary: IpaBoundary) -> &'static str {
    match boundary {
        IpaBoundary::AccentPhrase => "#",
        IpaBoundary::Pause => "_",
        IpaBoundary::Interrogative => "?",
        IpaBoundary::Exclamatory => "!",
        _ => "{boundary}",
    }
}

fn uniform_pitch(prosody: &[IpaTokenProsody]) -> Option<Option<PitchAccent>> {
    let IpaTokenProsody::Pitch(first) = prosody.first()? else {
        return None;
    };
    prosody
        .iter()
        .all(|item| matches!(item, IpaTokenProsody::Pitch(pitch) if pitch == first))
        .then_some(*first)
}

fn format_ipa_prosody_path(prosody: &[IpaTokenProsody], numeric: bool) -> String {
    prosody
        .iter()
        .map(|item| match item {
            IpaTokenProsody::Pitch(Some(PitchAccent::Low)) => {
                if numeric {
                    "0"
                } else {
                    "L"
                }
            }
            IpaTokenProsody::Pitch(Some(PitchAccent::High)) => {
                if numeric {
                    "1"
                } else {
                    "H"
                }
            }
            IpaTokenProsody::Pitch(None) => "-",
            IpaTokenProsody::Boundary(boundary) => format_ipa_boundary(*boundary),
            _ => "?",
        })
        .collect()
}

fn update_previous_pitch(prosody: &[IpaTokenProsody], previous_pitch: &mut Option<PitchAccent>) {
    for item in prosody {
        match item {
            IpaTokenProsody::Pitch(Some(pitch)) => *previous_pitch = Some(*pitch),
            IpaTokenProsody::Pitch(None) => {}
            IpaTokenProsody::Boundary(_) => *previous_pitch = None,
            _ => {}
        }
    }
}

fn format_prosodic_ipa(
    item: &ProsodicIpa,
    format: ProsodyFormat,
    previous_pitch: &mut Option<PitchAccent>,
) -> Vec<String> {
    match item {
        ProsodicIpa::Token { token, prosody } => {
            let token = format_ipa_token(token);
            let pitch = uniform_pitch(prosody);
            match format {
                ProsodyFormat::Default => {
                    let Some(pitch) = pitch else {
                        let path = format_ipa_prosody_path(prosody, false);
                        update_previous_pitch(prosody, previous_pitch);
                        return vec![format!("{token}{{{path}}}")];
                    };
                    let mut output = Vec::new();
                    if let Some(current) = pitch {
                        match (*previous_pitch, current) {
                            (Some(PitchAccent::Low), PitchAccent::High) => {
                                output.push("[".to_owned());
                            }
                            (Some(PitchAccent::High), PitchAccent::Low) => {
                                output.push("]".to_owned());
                            }
                            _ => {}
                        }
                        *previous_pitch = Some(current);
                    }
                    output.push(token);
                    output
                }
                ProsodyFormat::Prefix => {
                    if let Some(pitch) = pitch {
                        let prefix = match pitch {
                            Some(PitchAccent::High) => "H_",
                            Some(PitchAccent::Low) => "L_",
                            None => "",
                        };
                        vec![format!("{prefix}{token}")]
                    } else {
                        let path = format_ipa_prosody_path(prosody, false);
                        vec![format!("{{{path}}}_{token}")]
                    }
                }
                ProsodyFormat::Numeric => {
                    if let Some(pitch) = pitch {
                        let suffix = match pitch {
                            Some(PitchAccent::High) => ":1",
                            Some(PitchAccent::Low) => ":0",
                            None => "",
                        };
                        vec![format!("{token}{suffix}")]
                    } else {
                        let path = format_ipa_prosody_path(prosody, true);
                        vec![format!("{token}:{{{path}}}")]
                    }
                }
            }
        }
        ProsodicIpa::Boundary(boundary) => {
            *previous_pitch = None;
            vec![format_ipa_boundary(*boundary).to_owned()]
        }
        _ => vec!["{ipa-prosody}".to_owned()],
    }
}

macro_rules! handle_batch {
    ($texts:expr, $writer:expr, $format:expr, $res_batch:expr, |$res:ident| $text_format:block) => {
        for (text, $res) in $texts.iter().zip($res_batch) {
            if text.trim().is_empty() {
                writeln!($writer)?;
                continue;
            }
            match $format {
                OutputFormat::Text => $text_format,
                OutputFormat::Json => write_json($writer, &$res)?,
            }
        }
    };
}

fn process_batch(
    haqumei: &mut Haqumei,
    texts: &[String],
    mode: &OutputMode,
    format: &OutputFormat,
    prosody_format: ProsodyFormat,
    writer: &mut dyn Write,
) -> Result<()> {
    match mode {
        OutputMode::G2p => {
            let res_batch = haqumei.g2p_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                writeln!(writer, "{}", res.join(" "))?;
            });
        }
        OutputMode::Ipa => {
            let res_batch = haqumei.g2ipa_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                let tokens: Vec<_> = res
                    .iter()
                    .flat_map(|word| word.tokens.iter())
                    .map(format_ipa_token)
                    .collect();
                writeln!(writer, "{}", tokens.join(" "))?;
            });
        }
        OutputMode::IpaProsody => {
            let res_batch = haqumei.g2ipa_prosody_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                let mut previous_pitch = None;
                let tokens: Vec<_> = res
                    .iter()
                    .flat_map(|word| word.tokens.iter())
                    .flat_map(|item| format_prosodic_ipa(item, prosody_format, &mut previous_pitch))
                    .collect();
                writeln!(writer, "{}", tokens.join(" "))?;
            });
        }
        OutputMode::Prosody => {
            let res_batch = haqumei.g2p_prosody_with_options_batch(texts, prosody_format)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                writeln!(writer, "{}", res.join(" "))?;
            });
        }
        OutputMode::G2pDetailed => {
            let res_batch = haqumei.g2p_detailed_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                writeln!(writer, "{}", res.join(" "))?;
            });
        }
        OutputMode::Kana => {
            let res_batch = haqumei.g2k_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                writeln!(writer, "{}", res)?;
            });
        }
        OutputMode::KanaPerWord => {
            let res_batch = haqumei.g2k_per_word_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                writeln!(writer, "{}", res.join(" "))?;
            });
        }
        OutputMode::PerWord => {
            let res_batch = haqumei.g2p_per_word_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                let formatted: Vec<String> = res
                    .into_iter()
                    .map(|phonemes| format!("[{}]", phonemes.join(", ")))
                    .collect();
                writeln!(writer, "{}", formatted.join(" "))?;
            });
        }
        OutputMode::Candidates => {
            let res_batch = haqumei.g2p_candidates_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                for branch in &res.branches {
                    let alts: Vec<String> = branch
                        .alternatives
                        .iter()
                        .map(|a| format!("{}({})", a.pron(), a.delta))
                        .collect();
                    writeln!(
                        writer,
                        "# {}..{}\t{}\t{}",
                        branch.char_span.start,
                        branch.char_span.end,
                        branch.surface,
                        alts.join(" / ")
                    )?;
                }
                for cand in &res.candidates {
                    let phonemes: Vec<&str> = cand
                        .words
                        .iter()
                        .flat_map(|w| w.phonemes.iter())
                        .map(|p| p.as_str())
                        .collect();
                    writeln!(writer, "{}\t{}", cand.delta, phonemes.join(" "))?;
                }
            });
        }
        OutputMode::Mapping => {
            let res_batch = haqumei.g2p_mapping_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                for map in res {
                    let status = if map.is_unknown {
                        "[UNK]"
                    } else if map.is_ignored {
                        "[IGN]"
                    } else {
                        "[OK] "
                    };
                    writeln!(
                        writer,
                        "{} {}\t{}",
                        status,
                        map.word,
                        map.phonemes.join(" "),
                    )?;
                }
            });
        }
        OutputMode::MappingDetailed => {
            let res_batch = haqumei.g2p_mapping_detailed_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                for detail in res {
                    let status = if detail.is_unknown {
                        "[UNK]"
                    } else if detail.is_ignored {
                        "[IGN]"
                    } else {
                        "[OK] "
                    };
                    writeln!(
                        writer,
                        "{} {}: {}\tPOS: {}\tPOS_GROUP1: {}\tPRON: {}\tREAD: {}\tACC: {}/{}\tCHAIN_FLAG: {}\tCHAIN_RULE: {}",
                        status,
                        detail.word,
                        detail.phonemes.join(" "),
                        detail.pos,
                        detail.pos_group1,
                        detail.pron,
                        detail.read,
                        detail.accent_nucleus,
                        detail.mora_count,
                        detail.chain_flag,
                        detail.chain_rule,
                    )?;
                }
            });
        }
        OutputMode::MappingProsody => {
            let res_batch = haqumei.g2p_mapping_prosody_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                let mut prev_pitch = None;
                for detail in res {
                    let status = if detail.is_unknown {
                        "[UNK]"
                    } else if detail.is_ignored {
                        "[IGN]"
                    } else {
                        "[OK] "
                    };

                    let phones = detail
                        .to_formatted_strings(prosody_format, &mut prev_pitch)
                        .join(" ");

                    writeln!(
                        writer,
                        "{} {}: {}\tPOS: {}\tPOS_GROUP1: {}\tPRON: {}\tREAD: {}\tACC: {}/{}",
                        status,
                        detail.word,
                        phones,
                        detail.pos,
                        detail.pos_group1,
                        detail.pron,
                        detail.read,
                        detail.accent_nucleus,
                        detail.mora_count,
                    )?;
                }
            });
        }
        OutputMode::Fullcontext => {
            let res_batch = haqumei.extract_fullcontext_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                for label in res {
                    writeln!(writer, "{}", label)?;
                }
            });
        }
        OutputMode::FullcontextString => {
            let res_batch = haqumei.extract_fullcontext_string_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                for label in res {
                    writeln!(writer, "{}", label)?;
                }
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use haqumei::{IpaPhone, SpecialPhone};

    #[test]
    fn ipa_text_distinguishes_phones_labels_unknowns_and_boundaries() {
        assert_eq!(format_ipa_token(&IpaToken::Phone(IpaPhone::LongK)), "kː");
        assert_eq!(format_ipa_boundary(IpaBoundary::AccentPhrase), "#");
        assert_eq!(format_ipa_boundary(IpaBoundary::Pause), "_");
        assert_eq!(format_ipa_token(&IpaToken::Unknown), "{unk}");
        assert_eq!(
            format_ipa_token(&IpaToken::Special(SpecialPhone::Sokuon)),
            "{Q}"
        );
    }

    #[test]
    fn ipa_modes_accept_api_name_aliases() {
        let ipa = Cli::try_parse_from(["haqumei-cli", "--mode", "g2ipa"]).unwrap();
        assert!(matches!(ipa.mode, OutputMode::Ipa));

        let mapping = Cli::try_parse_from(["haqumei-cli", "--mode", "g2ipa-mapping"]).unwrap();
        assert!(matches!(mapping.mode, OutputMode::Ipa));

        let prosody = Cli::try_parse_from(["haqumei-cli", "--mode", "g2ipa-prosody"]).unwrap();
        assert!(matches!(prosody.mode, OutputMode::IpaProsody));
    }

    #[test]
    fn ipa_text_keeps_compound_token_prosody() {
        let token = ProsodicIpa::Token {
            token: IpaToken::Phone(IpaPhone::LongO),
            prosody: vec![
                IpaTokenProsody::Pitch(Some(PitchAccent::Low)),
                IpaTokenProsody::Pitch(Some(PitchAccent::High)),
            ],
        };

        assert_eq!(
            format_prosodic_ipa(&token, ProsodyFormat::Prefix, &mut None),
            ["{LH}_oː"]
        );
        assert_eq!(
            format_prosodic_ipa(&token, ProsodyFormat::Numeric, &mut None),
            ["oː:{01}"]
        );
        assert_eq!(
            format_prosodic_ipa(&token, ProsodyFormat::Default, &mut None),
            ["oː{LH}"]
        );
    }
}
