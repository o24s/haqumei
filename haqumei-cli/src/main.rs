use anyhow::{Context, Result};
use clap::{Args, Parser, ValueEnum};
use haqumei::{Haqumei, HaqumeiOptions, IuPronunciation, ProsodyFormat, UnicodeNormalization};
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
    /// 指定がない場合は、引数 [TEXT] または標準入力から読み取ります。
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

    /// プロソディ出力フォーマット (mode が prosody または mapping-prosody の場合に有効)
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
    /// 形態素ごとの音素マッピング (word: phonemes)
    Pairs,
    /// 形態素ごとの未知語情報を含めたマッピング
    Mapping,
    /// 未知語情報や NJD の詳細な特徴量を含めたマッピング
    MappingDetailed,
    /// 形態素ごとの詳細なプロソディ情報を含めたマッピング
    MappingProsody,
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

    /// ユーザー辞書のパス (.csv)
    #[arg(long, value_name = "FILE")]
    user_dict: Option<PathBuf>,
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

    /// 辞書が潰した稀な音節 (ヴィ / テュ など) の復元を無効にする (デフォルトは有効)
    #[arg(long)]
    no_restore_rare_syllables: bool,

    /// 辞書に無い漢字へのフォールバック読みを無効にする (デフォルトは有効)
    #[arg(long)]
    no_read_unknown_kanji: bool,

    /// 数詞まわりの読みの補正を無効にする (デフォルトは有効)
    #[arg(long)]
    no_modify_numeral_reading: bool,

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

    /// r/ry の前の撥音「ン」をさらに専用の Nr [n̠] (後部歯茎鼻音) に解決する (split_n_allophones が必要)
    #[arg(long)]
    split_n_before_r: bool,

    /// ch, j の前の撥音「ン」をさらに専用の Npl [ɲ] (硬口蓋鼻音) に解決する (split_n_allophones が必要)
    #[arg(long)]
    split_n_before_palatal_affricate: bool,

    /// 語中の促音「ッ」を後続音素の環境に応じて異音 (ClP, ClT, ClK, ClS, ClV) に分岐させる
    #[arg(long)]
    split_q_allophones: bool,

    /// 語末やポーズ前における、後続子音を伴わない促音「ッ」を専用の声門閉鎖音 ClQ [ʔ] として出力する
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
        use_read_as_pron: cli.options.use_read_as_pron,
        revert_long_vowels: cli.options.revert_long_vowels,
        revert_yotsugana: cli.options.revert_yotsugana,
        modify_filler_accent: !cli.options.no_modify_filler_accent,
        predict_nani: !cli.options.no_predict_nani,
        predict_kana_english: !cli.options.no_predict_kana_english,
        modify_context_reading: !cli.options.no_modify_context_reading,
        modify_old_province_yomi: !cli.options.no_modify_old_province_yomi,
        restore_rare_syllables: !cli.options.no_restore_rare_syllables,
        read_unknown_kanji: !cli.options.no_read_unknown_kanji,
        modify_numeral_reading: !cli.options.no_modify_numeral_reading,
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
        (Some(dict), Some(user_dict)) => {
            Haqumei::from_path_with_userdict(dict, user_dict, haqumei_options)
                .context("Failed to load dictionary and user dictionary")?
        }
        (Some(dict), None) => {
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
        OutputMode::Pairs => {
            let res_batch = haqumei.g2p_pairs_batch(texts)?;
            handle_batch!(texts, writer, format, res_batch, |res| {
                for pair in res {
                    writeln!(writer, "{}\t{}", pair.word, pair.phonemes.join(" "))?;
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
