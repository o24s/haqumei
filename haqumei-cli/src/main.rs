use anyhow::{Context, Result};
use clap::{Args, Parser, ValueEnum};
use haqumei::{Haqumei, HaqumeiOptions, UnicodeNormalization};
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

    /// 詳細なログ (Unidic辞書のダウンロード状況やOpenJTalkの警告など) を表示します。
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
    /// フルコンテキストラベル
    Fullcontext,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum OutputFormat {
    /// 人間が読みやすいテキスト形式
    Text,
    /// 構造化された JSON (JSON Lines) 形式
    Json,
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

    /// 読み (read) を発音 (pron) の代わりに使用し、長音の自動変換などを無効化する
    #[arg(long)]
    use_read_as_pron: bool,

    /// 辞書によって自動的に長音化された発音を、元のテキストに忠実な読みに復元する
    #[arg(long)]
    revert_long_vowels: bool,

    /// 四つ仮名（ヅ・ヂ）を元のテキスト通りの表記に復元する
    #[arg(long)]
    revert_yotsugana: bool,

    /// フィラーのアクセント修正を無効にする (デフォルトは有効)
    #[arg(long)]
    no_modify_filler_accent: bool,

    /// Nani Predictor による「何」の読み修正を無効にする (デフォルトは有効)
    #[arg(long)]
    no_predict_nani: bool,

    /// Unidic を使って漢字の読みを修正する (初回実行時に辞書をダウンロードします)
    #[arg(long)]
    use_unidic_yomi: bool,

    /// アクセント核を1つ前のモーラにずらすルールを無効にする (デフォルトは有効)
    #[arg(long)]
    no_retreat_acc_nuc: bool,

    /// 品詞「特殊・マス」前のアクセント移動を無効にする (デフォルトは有効)
    #[arg(long)]
    no_modify_acc_after_chaining: bool,

    /// 踊り字 (々, ヽ, ヾ) の展開を無効にする (デフォルトは有効)
    #[arg(long)]
    no_process_odoriji: bool,
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    let default_log_level = if cli.verbose { "info" } else { "error" };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_log_level))
        .target(env_logger::Target::Stderr)
        .init();

    let haqumei_options = HaqumeiOptions {
        normalize_unicode: cli.options.normalize_unicode.into(),
        use_read_as_pron: cli.options.use_read_as_pron,
        revert_long_vowels: cli.options.revert_long_vowels,
        revert_yotsugana: cli.options.revert_yotsugana,
        modify_filler_accent: !cli.options.no_modify_filler_accent,
        predict_nani: !cli.options.no_predict_nani,
        use_unidic_yomi: cli.options.use_unidic_yomi,
        retreat_acc_nuc: !cli.options.no_retreat_acc_nuc,
        modify_acc_after_chaining: !cli.options.no_modify_acc_after_chaining,
        process_odoriji: !cli.options.no_process_odoriji,
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

    if let Some(text) = cli.text.as_deref() {
        process_line(&mut haqumei, text, &cli.mode, &cli.format, &mut writer)?;
    } else if let Some(input_path) = cli.input {
        let file = File::open(&input_path)
            .with_context(|| format!("Failed to open input file: {:?}", input_path))?;
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            let line = line.context("Failed to read line from file")?;
            if line.trim().is_empty() {
                writeln!(writer)?;
                continue;
            }
            process_line(&mut haqumei, &line, &cli.mode, &cli.format, &mut writer)?;
        }
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
                process_line(&mut haqumei, trimmed, &cli.mode, &cli.format, &mut writer)?;

                writer.flush()?;
            }
        } else {
            for line in stdin.lock().lines() {
                let line = line.context("Failed to read line from stdin")?;
                if line.trim().is_empty() {
                    writeln!(writer)?;
                    continue;
                }
                process_line(&mut haqumei, &line, &cli.mode, &cli.format, &mut writer)?;
            }
        }
    }

    writer.flush()?;
    Ok(())
}

#[inline(always)]
fn write_json<T: serde::Serialize>(writer: &mut dyn Write, data: &T) -> Result<()> {
    serde_json::to_writer(&mut *writer, data)?;
    writeln!(writer)?;
    Ok(())
}

fn process_line(
    haqumei: &mut Haqumei,
    text: &str,
    mode: &OutputMode,
    format: &OutputFormat,
    writer: &mut dyn Write,
) -> Result<()> {
    match mode {
        OutputMode::G2p => {
            let res = haqumei.g2p(text)?;
            match format {
                OutputFormat::Text => writeln!(writer, "{}", res.join(" "))?,
                OutputFormat::Json => write_json(writer, &res)?,
            }
        }
        OutputMode::G2pDetailed => {
            let res = haqumei.g2p_detailed(text)?;
            match format {
                OutputFormat::Text => writeln!(writer, "{}", res.join(" "))?,
                OutputFormat::Json => write_json(writer, &res)?,
            }
        }
        OutputMode::Kana => {
            let res = haqumei.g2p_kana(text)?;
            match format {
                OutputFormat::Text => writeln!(writer, "{}", res)?,
                OutputFormat::Json => write_json(writer, &res)?,
            }
        }
        OutputMode::KanaPerWord => {
            let res = haqumei.g2p_kana_per_word(text)?;
            match format {
                OutputFormat::Text => writeln!(writer, "{}", res.join(" "))?,
                OutputFormat::Json => write_json(writer, &res)?,
            }
        }
        OutputMode::PerWord => {
            let res = haqumei.g2p_per_word(text)?;
            match format {
                OutputFormat::Text => {
                    let formatted: Vec<String> = res
                        .into_iter()
                        .map(|phonemes| format!("[{}]", phonemes.join(", ")))
                        .collect();
                    writeln!(writer, "{}", formatted.join(" "))?;
                }
                OutputFormat::Json => write_json(writer, &res)?,
            }
        }
        OutputMode::Pairs => {
            let res = haqumei.g2p_pairs(text)?;
            match format {
                OutputFormat::Text => {
                    for pair in res {
                        writeln!(writer, "{}\t{}", pair.word, pair.phonemes.join(" "))?;
                    }
                }
                OutputFormat::Json => write_json(writer, &res)?,
            }
        }
        OutputMode::Mapping => {
            let res = haqumei.g2p_mapping(text)?;
            match format {
                OutputFormat::Text => {
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
                }
                OutputFormat::Json => write_json(writer, &res)?,
            }
        }
        OutputMode::MappingDetailed => {
            let res = haqumei.g2p_mapping_detailed(text)?;
            match format {
                OutputFormat::Text => {
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
                            "{} {}\tPOS: {}\tPOS_GROUP1: {}\tPRON: {}\tREAD: {}\tACC: {}/{}\tCHAIN_FLAG: {}\tCHAIN_RULE: {}",
                            status,
                            detail.word,
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
                }
                OutputFormat::Json => write_json(writer, &res)?,
            }
        }
        OutputMode::Fullcontext => {
            let res = haqumei.extract_fullcontext(text)?;
            match format {
                OutputFormat::Text => {
                    for label in res {
                        writeln!(writer, "{}", label)?;
                    }
                }
                OutputFormat::Json => write_json(writer, &res)?,
            }
        }
    }
    Ok(())
}
