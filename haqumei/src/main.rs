use std::{fs, path::PathBuf, time::Instant};


fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let waganeko_path = manifest_dir.join("../resources/waganeko.txt");
    let waganeko = fs::read_to_string(waganeko_path)?;
    let waganeko: Vec<&str> = waganeko.split('\n').collect();

    let pojt = haqumei::ParallelJTalk::new()?;
    // let mut ojt = haqumei::OpenJTalk::new()?;

    let start = Instant::now();
    let result = pojt.g2p(&waganeko).unwrap();
    // let result: Vec<Vec<String>> = waganeko.iter().map(|l| ojt.g2p(l).unwrap()).collect();
    let elapsed = start.elapsed();

    let sentences_per_sec = waganeko.len() as f64 / elapsed.as_secs_f64();
    println!("finished in {:.2?}. ({:.2} sentences/sec)", elapsed, sentences_per_sec);

    (0..3).for_each(|i| println!("{:?}", result[i]));
    Ok(())
}
