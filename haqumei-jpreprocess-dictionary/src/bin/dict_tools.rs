use clap::{Parser, Subcommand};
use haqumei_jpreprocess_dictionary::{
    mecab::Model,
    mecab_compile::{build_system_with_charsets, build_user, BuildOptions},
};
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// システム辞書を構築します。
    Build {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, default_value = "utf-8")]
        charset: String,
    },
    /// システム辞書の定義を使ってユーザー辞書を構築します。
    BuildUser {
        #[arg(long)]
        dictionary: PathBuf,
        #[arg(required = true)]
        input: Vec<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// テキストに一致する全ラティス候補を表示します。
    Inspect {
        input: PathBuf,
        text: String,
        #[arg(short, long)]
        user_dictionary: Vec<PathBuf>,
    },
}

fn main() -> std::io::Result<()> {
    match Cli::parse().command {
        Command::Build {
            input,
            output,
            charset,
        } => {
            build_system_with_charsets(&input, &output, &BuildOptions::default(), &charset, "utf-8")
        }
        Command::BuildUser {
            dictionary,
            input,
            output,
        } => build_user(&dictionary, &input, &output),
        Command::Inspect {
            input,
            text,
            user_dictionary,
        } => {
            let analysis = Model::open(&input, &user_dictionary)?.analyze(&text)?;
            for node in analysis.nodes {
                println!("{node:?}");
            }
            Ok(())
        }
    }
}
