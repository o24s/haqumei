use haqumei::OpenJTalk;

fn main() {
    let mut open_jtalk = OpenJTalk::new().unwrap();

    dbg!(open_jtalk.g2p_mapping_prosody("Nür").unwrap());
}
