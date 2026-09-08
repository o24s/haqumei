use std::error::Error;
use std::path::PathBuf;

use haqumei_jpreprocess::*;

use clap::{Args, Parser};
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    dict: DictionaryArgs,

    /// The location of the user dictionary
    #[arg(short, long)]
    user_dictionary: Option<PathBuf>,

    /// The text to be processed
    input: String,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
struct DictionaryArgs {
    /// The location of the system dictionary
    #[arg(short, long)]
    dictionary: Option<PathBuf>,

    /// Use bundled naist-jdic dictionary
    #[cfg(feature = "naist-jdic")]
    #[arg(short, long)]
    naist_jdic: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let dictionary = if let Some(dict) = cli.dict.dictionary {
        SystemDictionaryConfig::File(dict)
    } else {
        #[cfg(not(feature = "naist-jdic"))]
        unreachable!("This build of haqumei_jpreprocess does not have the bundled dictionary, and it is not supporsed to reach here.");
        #[cfg(feature = "naist-jdic")]
        SystemDictionaryConfig::Bundled(kind::JPreprocessDictionaryKind::NaistJdic)
    };

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
