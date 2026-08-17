//! `haqumei/dictionary` のような MeCab 辞書ソースを `sys.dic` などへコンパイルする。
//!
//! 入出力ディレクトリを指定できるので、上流の辞書更新に追従する際に
//! 新旧の辞書を別ディレクトリへ並べてビルドし、精度を比較できる。
//!
//! ```text
//! # 既定 (haqumei/dictionary -> compiled)
//! cargo run -p haqumei-dict-tool
//!
//! # 新旧を並べてビルドする
//! cargo run -p haqumei-dict-tool -- --dict-dir /path/to/old -o compiled_old
//! cargo run -p haqumei-dict-tool -- --dict-dir /path/to/new -o compiled_new
//!
//! # ユーザー辞書だけをコンパイルする
//! cargo run -p haqumei-dict-tool -- --user-dict extra.csv -o out/user.dic
//! ```

use std::{
    env,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::Parser;
use haqumei::MecabDictIndexCompiler;

/// 既定の辞書ソース (このクレートからの相対パス)
const DEFAULT_DICT_DIR: &str = "../haqumei/dictionary";
/// 既定の出力先 (このクレートからの相対パス)
const DEFAULT_OUT_DIR: &str = "../compiled";

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Cli {
    /// 辞書ソースのディレクトリ (`*.csv`, `*.def` を含む)
    #[arg(long, value_name = "DIR")]
    dict_dir: Option<PathBuf>,

    /// コンパイル結果の出力先ディレクトリ
    #[arg(short, long, value_name = "DIR")]
    out_dir: Option<PathBuf>,

    /// 辞書ソースの文字コード
    #[arg(long, default_value = "utf-8", value_name = "CHARSET")]
    charset: String,

    /// 出力する辞書の文字コード (指定しない場合は `--charset` と同じ)
    #[arg(long, value_name = "CHARSET")]
    dictionary_charset: Option<String>,

    /// システム辞書ではなくユーザー辞書としてコンパイルする対象の CSV
    ///
    /// 複数回指定できる。この場合 `--out-dir` は出力する `.dic` ファイルのパスとして扱う。
    #[arg(long, value_name = "FILE")]
    user_dict: Vec<PathBuf>,

    /// mecab-dict-index の進捗出力を抑制する
    #[arg(short, long)]
    quiet: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // 引数を省略した場合は、呼び出し位置に依存しないようクレートからの相対で解決する
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let dict_dir = cli
        .dict_dir
        .unwrap_or_else(|| manifest_dir.join(DEFAULT_DICT_DIR));
    let out = cli
        .out_dir
        .unwrap_or_else(|| manifest_dir.join(DEFAULT_OUT_DIR));

    if !dict_dir.is_dir() {
        eprintln!(
            "error: 辞書ソースのディレクトリが見つかりません: {}",
            dict_dir.display()
        );
        return ExitCode::FAILURE;
    }

    let is_user_dict = !cli.user_dict.is_empty();

    let mut compiler = MecabDictIndexCompiler::new();
    compiler
        .dict_dir(&dict_dir)
        .charset(&cli.charset)
        .quiet(cli.quiet);

    if let Some(charset) = &cli.dictionary_charset {
        compiler.dictionary_charset(charset);
    }

    if is_user_dict {
        // ユーザー辞書では `--out-dir` を出力ファイルのパスとして扱う
        let out_path = if out.is_dir() || out.extension().is_none() {
            out.join("user.dic")
        } else {
            out.clone()
        };
        if let Some(parent) = out_path.parent() {
            compiler.out_dir(parent);
        }
        compiler.userdict_out_path(&out_path);
        for path in &cli.user_dict {
            if !path.is_file() {
                eprintln!("error: 入力ファイルが見つかりません: {}", path.display());
                return ExitCode::FAILURE;
            }
            compiler.add_input_file(path);
        }
        report(&dict_dir, &out_path, "ユーザー辞書");
    } else {
        compiler.out_dir(&out);
        report(&dict_dir, &out, "システム辞書");
    }

    if let Err(e) = compiler.run() {
        eprintln!("error: 辞書のコンパイルに失敗しました: {e}");
        return ExitCode::FAILURE;
    }

    println!("done.");
    ExitCode::SUCCESS
}

fn report(dict_dir: &Path, out: &Path, kind: &str) {
    println!("{kind}をコンパイルします");
    println!("  ソース : {}", dict_dir.display());
    println!("  出力先 : {}", out.display());
}
