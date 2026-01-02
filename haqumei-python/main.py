import haqumei
import pprint

def basic_usage():
    model = haqumei.Haqumei()

    text = "こんにちは、世界。"

    phonemes = model.g2p(text)
    print(f"音素: {phonemes}\n")

    kana = model.g2p_kana(text)
    print(f"カナ: {kana}\n")

    per_word = model.g2p_per_word(text)
    print(f"単語ごと: {per_word}\n")

    mapping = model.g2p_mapping(text)
    pprint.pprint(mapping)
    print()

    features = model.run_frontend(text)
    pprint.pprint(features)

if __name__ == "__main__":
    basic_usage()