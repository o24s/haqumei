import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class UnihanGenerationTests(unittest.TestCase):
    def generate(self, text):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "Unihan_Readings.txt"
            output = Path(directory) / "readings.rs"
            source.write_text(text, encoding="utf-8")
            subprocess.run(
                [sys.executable, str(Path(__file__).with_name("build_unihan_readings.py")),
                 "--input", str(source), "--out", str(output)],
                check=True, capture_output=True, text=True,
            )
            return output.read_text(encoding="utf-8")

    def test_official_codepoints_and_reading_selection(self):
        output = self.generate(
            "# Unicode 17.0.0\n"
            "U+3400\tkJapanese\tキュウ おか\n"
            "U+34DD\tkJapanese\tケィ のり\n"
            "U+20000\tkJapanese\tはじめ\n"
            "U+3401\tkMandarin\ttian4\n"
        )
        self.assertIn("'㐀' => \"キュウ\"", output)
        self.assertIn("'㓝' => \"ケイ\"", output)
        self.assertIn("'𠀀' => \"ハジメ\"", output)
        self.assertNotIn("'㐁'", output)

    def test_correction_does_not_add_an_absent_character(self):
        self.assertNotIn("'㓝'", self.generate("U+3400\tkJapanese\tキュウ\n"))


if __name__ == "__main__":
    unittest.main()
