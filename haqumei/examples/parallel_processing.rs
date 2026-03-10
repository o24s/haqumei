use std::{fs, path::PathBuf, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let waganeko_path = manifest_dir.join("../resources/waganeko.txt");
    let waganeko = fs::read_to_string(waganeko_path)?;
    let waganeko: Vec<&str> = waganeko.split('\n').collect();

    let mut haqumei = haqumei::Haqumei::new()?;

    let start = Instant::now();
    let result = haqumei.g2p_batch(&waganeko).unwrap();
    let elapsed = start.elapsed();

    let sentences_per_sec = waganeko.len() as f64 / elapsed.as_secs_f64();
    println!(
        "finished in {:.2?}. ({:.2} sentences/sec)",
        elapsed, sentences_per_sec
    );

    result.iter().take(3).for_each(|v| println!("{v:?}"));

    Ok(())
}
