use std::error::Error;
use std::path::PathBuf;

use haqumei_jpreprocess::*;

use clap::Parser;
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// システム辞書のディレクトリー。
    #[arg(short, long)]
    dictionary: PathBuf,

    /// The location of the user dictionary
    #[arg(short, long)]
    user_dictionary: Option<PathBuf>,

    /// The text to be processed
    input: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let dictionary = SystemDictionaryConfig::File(cli.dictionary);

    let users = cli.user_dictionary.into_iter().collect::<Vec<_>>();
    let haqumei_jpreprocess =
        JPreprocess::from_tokenizer(dictionary.load_with_user_dictionaries(&users)?);

    let njd_texts: Vec<String> = haqumei_jpreprocess.text_to_njd(&cli.input)?.into();
    for line in njd_texts {
        println!("{}", line);
    }

    let njd = haqumei_jpreprocess.run_frontend(&cli.input)?;

    println!("[NJD]");
    for line in &njd {
        println!("{}", line);
    }

    println!("\n[JPCommon]");
    for line in haqumei_jpreprocess.make_label(njd) {
        println!("{}", line);
    }

    Ok(())
}
