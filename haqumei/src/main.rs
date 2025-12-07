use haqumei::{Haqumei, open_jtalk::OpenJTalk};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = "叙々々々々々々々々々々々々々々々々々々々々々々々々々々々々々々々々々々苑";
    let mut haqumei = Haqumei::new()?;
    let mut openj_talk = OpenJTalk::new()?;

    let kana = haqumei.g2p(text, true)?;
    let phonemes = haqumei.g2p(text, false)?;
    // let features = haqumei.run_frontend(text)?;
    let open_kana = openj_talk.g2p(text, true)?;
    let open_phonemes = openj_talk.g2p(text, false)?;
    // let open_features = openj_talk.run_frontend(text)?;

    dbg!(&phonemes);
    dbg!(&kana);
    // dbg!(features);
    dbg!(&open_phonemes);
    dbg!(&open_kana);
    // dbg!(open_features);

    Ok(())
}
