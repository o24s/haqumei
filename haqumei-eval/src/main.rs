use pyo3::prelude::*;
use pyo3::types::PyDict;
use regex::Regex;
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone)]
struct RohanEntry {
    text: String,
    label: String,
}

fn load_rohan_data(path: &Path) -> Result<Vec<RohanEntry>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file);
    let re = Regex::new(r"\(.*?\)").unwrap();
    let mut entries = Vec::new();

    for result in rdr.records() {
        let record = result?;
        let text_with_id = record.get(0).ok_or("Missing text column")?;
        let label = record.get(1).ok_or("Missing label column")?;

        if let Some(text_part) = text_with_id.split(':').nth(1) {
            let cleaned_text = re.replace_all(text_part.trim(), "").to_string();
            if !cleaned_text.is_empty() {
                entries.push(RohanEntry {
                    text: cleaned_text,
                    label: label.to_string(),
                });
            }
        }
    }
    Ok(entries)
}

fn calculate_bleu_with_pyo3(references: Vec<String>, hypotheses: Vec<String>) -> PyResult<()> {
    Python::attach(|py| {
        setup_local_venv(py)?;
        let nltk_bleu = py.import("nltk.translate.bleu_score")?;
        let smoothing_function_class = nltk_bleu.getattr("SmoothingFunction")?;

        let smoothing_function_instance = smoothing_function_class.call0()?;

        let method1 = smoothing_function_instance.getattr("method1")?;

        let corpus_bleu = nltk_bleu.getattr("corpus_bleu")?;

        let py_references: Vec<Vec<Vec<char>>> = references
            .into_iter()
            .map(|r| vec![r.chars().collect()])
            .collect();
        let py_hypotheses: Vec<Vec<char>> = hypotheses
            .into_iter()
            .map(|h| h.chars().collect())
            .collect();

        let kwargs = PyDict::new(py);
        kwargs.set_item("smoothing_function", method1)?;

        println!("\n-- BLEU Scores (calculated via PyO3) --");

        kwargs.set_item("weights", (1.0, 0.0, 0.0, 0.0))?;
        let score1: f64 = corpus_bleu
            .call((&py_references, &py_hypotheses), Some(&kwargs))?
            .extract()?;
        println!("Corpus BLEU-1: {:.6}", score1);

        kwargs.set_item("weights", (0.5, 0.5, 0.0, 0.0))?;
        let score2: f64 = corpus_bleu
            .call((&py_references, &py_hypotheses), Some(&kwargs))?
            .extract()?;
        println!("Corpus BLEU-2: {:.6}", score2);

        kwargs.set_item("weights", (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0, 0.0))?;
        let score3: f64 = corpus_bleu
            .call((&py_references, &py_hypotheses), Some(&kwargs))?
            .extract()?;
        println!("Corpus BLEU-3: {:.6}", score3);

        kwargs.set_item("weights", (0.25, 0.25, 0.25, 0.25))?;
        let score4: f64 = corpus_bleu
            .call((&py_references, &py_hypotheses), Some(&kwargs))?
            .extract()?;
        println!("Corpus BLEU-4: {:.6}", score4);

        Ok(())
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Loading ROHAN4600 data...");
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rohan_data_path = manifest_dir
        .join("../resources")
        .join("Rohan4600_transcript_utf8.txt");
    let rohan_data = load_rohan_data(&rohan_data_path)?;
    println!("> Loaded {} entries.", rohan_data.len());

    println!("\nInitializing Haqumei...");
    let mut haqumei = haqumei::Haqumei::new()?;
    println!("> Initialization complete.");

    println!(
        "\nGenerating g2p results for {} sentences...",
        rohan_data.len()
    );
    let start_time = Instant::now();

    let hypotheses: Vec<String> = rohan_data
        .iter()
        .map(|entry| haqumei.g2p_kana(&entry.text).unwrap())
        .collect();

    let elapsed = start_time.elapsed();

    let sentences_per_sec = rohan_data.len() as f64 / elapsed.as_secs_f64();
    println!(
        "> Generation finished in {:.2?}. ({:.2} sentences/sec)",
        elapsed, sentences_per_sec
    );

    let references: Vec<String> = rohan_data.clone().into_iter().map(|e| e.label).collect();

    println!("\nCalculating BLEU scores...");
    calculate_bleu_with_pyo3(references, hypotheses)?;
    println!("> Evaluation complete.");

    let mut failed_cases = Vec::new();
    let mut correct_count = 0;

    for entry in rohan_data.iter() {
        let haqumei_result = haqumei.g2p_kana(&entry.text).unwrap_or_default();

        if haqumei_result != entry.label {
            failed_cases.push((entry.text.clone(), haqumei_result, entry.label.clone()));
        } else {
            correct_count += 1;
        }
    }

    let total_count = rohan_data.len();
    let failed_count = failed_cases.len();
    let accuracy = (correct_count as f64 / total_count as f64) * 100.0;

    println!("\n--- Analysis Complete ---");
    println!("Total sentences: {}", total_count);
    println!("Correctly predicted: {}", correct_count);
    println!("Incorrectly predicted: {}", failed_count);
    println!("Accuracy: {:.2}%", accuracy);

    let mut writer = csv::Writer::from_path("failed_cases.csv")?;
    writer.write_record([
        "Original_Text",
        "Haqumei_Result(Hypothesis)",
        "Correct_Label(Reference)",
    ])?;

    for (text, haqumei_result, label) in failed_cases {
        writer.write_record([&text, &haqumei_result, &label])?;
    }
    writer.flush()?;

    println!("\nDetails of failed cases have been saved to 'failed_cases.csv'.");

    Ok(())
}

/// カレントディレクトリの .venv を探し、sys.path に追加する
fn setup_local_venv(py: Python) -> PyResult<()> {
    let current_dir = env::current_dir()?;
    let venv_dir = current_dir.join(".venv");

    if venv_dir.exists() {
        let lib_dir = venv_dir.join("lib");

        if let Ok(entries) = fs::read_dir(lib_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && let Some(name) = path.file_name()
                    && name.to_string_lossy().starts_with("python")
                {
                    let site_packages = path.join("site-packages");

                    if site_packages.exists() {
                        let sys = py.import("sys")?;
                        let sys_path = sys.getattr("path")?;

                        if let Some(sp_str) = site_packages.to_str() {
                            sys_path.call_method1("insert", (0, sp_str))?;
                            println!("> Auto-detected venv: {}", sp_str);
                        }
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
