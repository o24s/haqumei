from enum import IntEnum
from typing import Final, Literal, TypeAlias

Phoneme: TypeAlias = Literal[
    "A", "E", "I", "O", "U", "N",
    "a", "b", "by", "ch", "cl", "d", "dy", "e", "f", "fy",
    "g", "gw", "gy", "h", "hy", "i", "j", "k", "kw", "ky",
    "m", "my", "n", "ny", "o", "p", "py", "r", "ry", "s",
    "sh", "t", "ts", "ty", "u", "v", "w", "y", "z",
    "sp", "pau", "unk",
]

ALL_PHONEMES: Final[tuple[Phoneme, ...]]

class UnicodeNormalization(IntEnum):
    """Unicode正規化の方式を指定する。"""

    None_ = 0
    Nfc = 1
    Nfkc = 2

class IuPronunciation(IntEnum):
    """「言う」の発音正規化方式を指定する。"""

    None_ = 0
    Iu = 1
    Yuu = 2
    KanjiIu = 3
    KanjiYuu = 4
    YuuBase = 5
    KanjiYuuBase = 6

class PitchAccent(IntEnum):
    """音素ごとのピッチアクセント (高低) を表す enum"""

    Low = 0
    High = 1

class ProsodyFormat(IntEnum):
    """出力するプロソディ表現のフォーマット"""

    Default = 0
    """tdmelodic 風 (`a [ o ] i #`) の記法"""
    Prefix = 1
    """`L_a`, `H_o` のようなプレフィックス表現"""
    Numeric = 2
    """`a:0`, `o:1` のような数値サフィックス表現"""

class NjdFeature:
    """
    このクラスは Rust 側で生成された読み取り専用のデータ構造です。
    各フィールドは OpenJTalk の内部表現に対応しています。
    """

    string: str
    """表層形"""

    pos: str
    """品詞"""

    pos_group1: str
    """品詞細分類1"""

    pos_group2: str
    """品詞細分類2"""

    pos_group3: str
    """品詞細分類3"""

    ctype: str
    """活用型"""

    cform: str
    """活用形"""

    orig: str
    """原形"""

    read: str
    """読み"""

    pron: str
    """発音"""

    acc: int
    """アクセント核の位置"""

    mora_size: int
    """モーラ数"""

    chain_rule: str
    """連結規則"""

    chain_flag: int
    """連結フラグ"""

class MecabMorph:
    """MeCabによる解析結果の詳細情報。"""

    surface: Final[str]
    """形態素の表層形。"""

    feature: Final[str]
    """MeCab が出力した特徴量文字列。"""

    left_id: Final[int]
    """left-id.def で定義された左文脈 ID。"""

    right_id: Final[int]
    """right-id.def で定義された右文脈 ID。"""

    pos_id: Final[int]
    """pos-id.def で定義された品詞 ID。"""

    word_cost: Final[int]
    """辞書に定義された単語コスト。"""

    is_unknown: Final[bool]
    """MeCab が未知語 (`MECAB_UNK_NODE`) と判定したかどうか。"""

    is_ignored: Final[bool]
    """`OpenJTalk` のパイプラインで無視される対象かどうか。(e.g, "記号,空白")"""

class WordPhonemeMap:
    """単語とその音素列の対応関係を表すデータクラス。

    `g2p_mapping` メソッドによって生成されます。
    """

    word: str
    """単語の表層形。"""

    phonemes: list[str]
    """その単語に対応する音素のリスト。"""

    is_unknown: bool
    """MeCab が未知語 (`MECAB_UNK_NODE`) と判定したかどうか。"""

    is_ignored: bool
    """pyopenjtalk のパイプラインで無視される対象として空白 (sp) に置き換えられたか、または音素が割り当てられなかったか"""

    char_span: tuple[int, int]
    """解析対象の文字列における位置 (文字単位、`[start, end)`)。

    指すのは入力そのものではなく、Unicode 正規化と `text2mecab` を通したあとの
    文字列である。`text2mecab` は制御文字と範囲外の文字を出力せず、半角カナと濁点の
    並び (`ｶﾞ`) を 1 文字にまとめるので、入力と文字数が変わることがある。
    """

    def __eq__(self, other: object) -> bool: ...

class WordPhonemeDetail:
    """形態素ごとの音素マッピングと、NJDによる詳細な解析情報を保持する構造体"""

    word: str
    """表層形 (surface)"""
    phonemes: list[str]
    """音素のリスト"""
    features: list[str]
    """Mecab が出力した features。既知語は 12 列、未知語は 8 列"""
    pos: str
    """品詞"""
    pos_group1: str
    """品詞細分類1"""
    pos_group2: str
    """品詞細分類2"""
    pos_group3: str
    """品詞細分類3"""
    ctype: str
    """活用型"""
    cform: str
    """活用形"""
    orig: str
    """原形"""
    read: str
    """読み"""
    pron: str
    """発音形式"""
    accent_nucleus: int
    """アクセント核位置 (0: 平板型, 1-n: n番目のモーラにアクセント核)"""
    mora_count: int
    """モーラ数"""
    chain_rule: str
    """アクセント結合規則 (C1-C5/F1-F5/P1-P2 等)"""
    chain_flag: int
    """アクセント句連結フラグ"""
    is_unknown: bool
    """MeCab が未知語と判定したかどうか"""
    is_ignored: bool
    """pyopenjtalk のパイプラインで無視される対象として空白 (sp) に置き換えられたか、または音素が割り当てられなかったか"""

    char_span: tuple[int, int]
    """解析対象の文字列における位置 (文字単位、`[start, end)`)。

    指すのは入力そのものではなく、Unicode 正規化と `text2mecab` を通したあとの
    文字列である。`text2mecab` は制御文字と範囲外の文字を出力せず、半角カナと濁点の
    並び (`ｶﾞ`) を 1 文字にまとめるので、入力と文字数が変わることがある。
    """

    def __eq__(self, other: object) -> bool: ...

class ProsodicPhoneme:
    """プロソディ（ピッチやポーズなどの韻律情報）を伴う音素構造体。"""

    kind: str
    """要素の種類 ("phoneme", "accent_phrase_boundary", "pause", "interrogative", "exclamatory")"""
    phoneme: str | None
    """音素文字列 (kind == "phoneme" の場合のみ値を持つ)"""
    pitch: PitchAccent | None
    """ピッチアクセントの高低 (kind == "phoneme" の場合のみ値を持つ)"""

class WordPhonemeProsody:
    """単語ごとのプロソディ情報とNJD特徴量を保持する構造体"""

    word: str
    """表層形 (surface)"""
    phonemes: list[ProsodicPhoneme]
    """プロソディ付き音素のリスト"""
    pos: str
    """品詞"""
    pos_group1: str
    """品詞細分類1"""
    pos_group2: str
    """品詞細分類2"""
    pos_group3: str
    """品詞細分類3"""
    ctype: str
    """活用型"""
    cform: str
    """活用形"""
    orig: str
    """原形"""
    read: str
    """読み"""
    pron: str
    """発音形式"""
    accent_nucleus: int
    """アクセント核位置 (0: 平板型, 1-n: n番目のモーラにアクセント核)"""
    mora_count: int
    """モーラ数"""
    chain_rule: str
    """アクセント結合規則 (C1-C5/F1-F5/P1-P2 等)"""
    chain_flag: int
    """アクセント句連結フラグ"""
    is_unknown: bool
    """MeCab が未知語と判定したかどうか"""
    is_ignored: bool
    """pyopenjtalk のパイプラインで無視される対象として空白 (sp) に置き換えられたか、または音素が割り当てられなかったか"""

    char_span: tuple[int, int]
    """解析対象の文字列における位置 (文字単位、`[start, end)`)。

    指すのは入力そのものではなく、Unicode 正規化と `text2mecab` を通したあとの
    文字列である。`text2mecab` は制御文字と範囲外の文字を出力せず、半角カナと濁点の
    並び (`ｶﾞ`) を 1 文字にまとめるので、入力と文字数が変わることがある。
    """

    def __eq__(self, other: object) -> bool: ...

class CandidateOptions:
    """`g2p_candidates` がラティスから経路を集める範囲と、返す候補の数の上限。"""

    def __init__(
        self,
        *,
        max_delta: int = 2000,
        max_alternatives_per_branch: int = 4,
        max_candidates: int = 32,
        branch_on_unknown_words: bool = False,
    ) -> None: ...

    max_delta: int
    """ラティスに残すノードの、最良経路とのコスト差の上限。

    大きくすると区間が繋がって、経路の数が組み合わせで増える。
    """

    max_alternatives_per_branch: int
    """分岐点ごとに残す経路の上限。`CandidateBranch.alternatives` は先頭に 1-best を
    置くので、長さは `max_alternatives_per_branch + 1` までになる。

    経路の多い分岐点があると、そこを動かしただけの組み合わせで
    `max_candidates` に達し、後ろの分岐点を動かした組み合わせが 1 つも組み立て
    られないことがある。分岐点ごとに先に上限を掛けると起きない。
    """

    max_candidates: int
    """返す候補の数の上限。1 未満は 1 として扱う。

    数が減るのは `candidates` だけで、`branches` は 1 にしても埋まる。
    """

    branch_on_unknown_words: bool
    """未知語のノードを経路に含めるか。

    未知語のノードは `CandidateReading.pron` が `*` で、読みは `read_unknown_kanji` と
    `restore_loanword_kana` が決める。辞書のエントリと並べても同じ読みになることが
    多いので、既定では False にしてラティスから外す。
    """

class CandidateReading:
    """経路を組み立てているラティスのノード。"""

    surface: str
    """表層形。"""
    char_span: tuple[int, int]
    """`Candidates.text` における位置 (文字単位、`[start, end)`)。"""
    pron: str
    """辞書のエントリが持つ発音。未知語のノードでは `*` になる。

    `mecab2njd` に渡す前の値なので、実際に出る音素は `Candidate.words` を見る。
    """
    feature: str
    """`mecab2njd` に渡す feature 文字列。表層形が先頭に付く。"""
    delta: int
    """この読みを通る最良経路と、文全体の最良経路のコスト差。1-best は 0。"""
    left_id: int
    """left-id.def で定義された左文脈 ID。"""
    right_id: int
    """right-id.def で定義された右文脈 ID。"""
    word_cost: int
    """辞書に定義された単語コスト。"""
    is_unknown: bool
    """MeCab が未知語と判定したかどうか。"""

    def __eq__(self, other: object) -> bool: ...

class CandidateAlternative:
    """分岐点の区間を通る経路。

    分割の違いも候補にするので、`nodes` の数は経路ごとに違う。
    「彼の」は `彼` + `の` (カレノ) と 連体詞 `彼の` (アノ) の 2 通りで、
    前者の `nodes` は 2 個、後者は 1 個になる。
    """

    nodes: list[CandidateReading]
    """経路のノード。`CandidateBranch.char_span` に隙間なく並ぶ。"""
    delta: int
    """`CandidateReading.delta` の和。1-best の経路は 0。"""

    def pron(self) -> str:
        """経路のノードの発音を連ねた文字列を返す。"""

    def __eq__(self, other: object) -> bool: ...

class CandidateBranch:
    """経路が 2 通り以上ある区間。"""

    char_span: tuple[int, int]
    """分岐する区間の位置 (`Candidates.text` における文字単位、`[start, end)`)。"""
    surface: str
    """分岐する区間の表層形。"""
    alternatives: list[CandidateAlternative]
    """その区間を通る経路。0 番目が 1-best で、以降はコスト差の小さい順に並ぶ。"""

    def __eq__(self, other: object) -> bool: ...

class Candidate:
    """分岐点ごとに経路を 1 つ選んで解析し直した、1 文ぶんの結果 (`WordPhonemeMap`)。"""

    words: list[WordPhonemeMap]
    """形態素ごとの音素マッピング。"""
    delta: int
    """選んだ経路の `CandidateAlternative.delta` の和。`Candidates.candidates` の
    並び順を決める値で、小さいものから順に組み立てる。

    MeCab のコストは分割と品詞を決めるための値で、読みの確からしさを測ったもの
    ではないため、**FST のアークの重みには使えない**。
    """
    choices: list[int]
    """分岐点ごとに何番目の代替を選んだか。`Candidates.branches` と長さが揃う。"""

class Candidates:
    """1 文ぶんの候補集合 (`WordPhonemeMap`)。"""

    text: str
    """解析に使った文字列。

    入力に Unicode 正規化と `text2mecab` を掛けたあとのもので、`char_span` が指す
    先である。入力とは文字数が変わることがある。
    """
    branches: list[CandidateBranch]
    """経路が 2 通り以上ある区間。入力に現れる順に並び、`max_candidates` の上限を受けない。

    `candidates` を並べて FST を組むと、上限に達して組み立てなかった組み合わせが
    そのまま欠ける。すべて残したいなら `branches` から直積を組む。
    """
    candidates: list[Candidate]
    """候補。コスト差の小さい順に並び、先頭は 1-best である。

    音素列が同じ候補はコスト差の小さい方だけ残すので、`branches` の直積より少ない。
    入力が空でなければ空にならない。
    """

    def __len__(self) -> int: ...

class CandidateDetail:
    """`Candidate` の語を `WordPhonemeDetail` にしたもの。"""

    words: list[WordPhonemeDetail]
    delta: int
    choices: list[int]

class CandidatesDetail:
    """1 文ぶんの候補集合 (`WordPhonemeDetail`)。"""

    text: str
    branches: list[CandidateBranch]
    candidates: list[CandidateDetail]

    def __len__(self) -> int: ...

class CandidateProsody:
    """`Candidate` の語を `WordPhonemeProsody` にしたもの。"""

    words: list[WordPhonemeProsody]
    delta: int
    choices: list[int]

class CandidatesProsody:
    """1 文ぶんの候補集合 (`WordPhonemeProsody`)。"""

    text: str
    branches: list[CandidateBranch]
    candidates: list[CandidateProsody]

    def __len__(self) -> int: ...

class LabelPhoneme:
    """`Phoneme` field of full-context label."""

    p2: str | None
    p1: str | None
    c: str | None
    n1: str | None
    n2: str | None

class Mora:
    """`Mora` field of full-context label (`A` field)."""

    relative_accent_position: int
    position_forward: int
    position_backward: int

class Word:
    """`Word` field of full-context label (`B`, `C`, and `D` field)."""

    pos: int | None
    ctype: int | None
    cform: int | None

class AccentPhraseCurrent:
    """`AccentPhrase` field of full-context label for current accent phrase (`F` field)."""

    mora_count: int
    accent_position: int
    is_interrogative: bool
    accent_phrase_position_forward: int
    accent_phrase_position_backward: int
    mora_position_forward: int
    mora_position_backward: int
    is_exclamatory: bool

class AccentPhrasePrevNext:
    """`AccentPhrase` field of full-context label for previous or next accent phrase (`E` and `G` field)."""

    mora_count: int
    accent_position: int
    is_interrogative: bool
    is_pause_insertion: bool | None
    is_exclamatory: bool

class BreathGroupCurrent:
    """`BreathGroup` field of full-context label for current breath group (`I` field)."""

    accent_phrase_count: int
    mora_count: int
    breath_group_position_forward: int
    breath_group_position_backward: int
    accent_phrase_position_forward: int
    accent_phrase_position_backward: int
    mora_position_forward: int
    mora_position_backward: int

class BreathGroupPrevNext:
    """`BreathGroup` field of full-context label for previous or next breath group (`H` and `J` field)."""

    accent_phrase_count: int
    mora_count: int

class Utterance:
    """`Utterance` field of full-context label (`K` field)."""

    breath_group_count: int
    accent_phrase_count: int
    mora_count: int

class Label:
    """The structure representing a single line of HTS-style full-context label."""

    phoneme: LabelPhoneme
    mora: Mora | None
    word_prev: Word | None
    word_curr: Word | None
    word_next: Word | None
    accent_phrase_prev: AccentPhrasePrevNext | None
    accent_phrase_curr: AccentPhraseCurrent | None
    accent_phrase_next: AccentPhrasePrevNext | None
    breath_group_prev: BreathGroupPrevNext | None
    breath_group_curr: BreathGroupCurrent | None
    breath_group_next: BreathGroupPrevNext | None
    utterance: Utterance

class Dictionary:
    """OpenJTalk用の辞書データを管理するクラス。

    一度ロードした辞書データをメモリ上で保持します。
    このインスタンスを `OpenJTalk` に渡すことで、
    辞書データのメモリ共有が可能になり、
    Mecab による mmap syscall の時間を削減できます。
    """

    @staticmethod
    def from_path(dict_dir: str, user_dict: str | None = None) -> Dictionary:
        """指定されたパスから辞書をロードします。

        Args:
            dict_dir (str): システム辞書のディレクトリパス。
            user_dict (Optional[str], optional): ユーザー辞書のファイルパス。デフォルトは None。

        Returns:
            Dictionary: ロードされた辞書オブジェクト。

        Raises:
            RuntimeError: 指定されたパスに辞書が存在しない、またはフォーマットが不正な場合。
        """

    @staticmethod
    def from_embedded() -> Dictionary:
        """ライブラリに埋め込まれた辞書データをロードします。

        Returns:
            Dictionary: ロードされた辞書オブジェクト。
        """

class OpenJTalk:
    """OpenJTalk の機能を提供するラッパークラス。

    [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) の辞書を使用しています。

    `g2p_**`の実装において、フルコンテキストラベルを経由せず、JPCommon で構築された内部ポインタを追って
    g2p を行うため、他の Open JTalk バインディング実装より若干高速です。
    また、他のバインディングにない以下の関数が実装されています。
    - `g2p_per_word`: テキストを単語ごとに区切られた音素リストに変換します。
    - `g2p_candidates`: 読みが分かれる箇所で複数の候補を返します。


    スレッドセーフに設計されていますが、内部で排他ロック (Mutex) を使用するため、
    Python の `threading` を用いても並列処理による高速化は期待できません。
    並行に処理をしたい場合は、各種 `*_batch` メソッドを使用してください。

    Examples:

    >>> ojt = OpenJTalk()
    >>> ojt.g2p("こんにちは")
    ['k', 'o', 'N', 'n', 'i', 'ch', 'i', 'w', 'a']
    """

    def __init__(self) -> None:
        """新しい OpenJTalk インスタンスを初期化します。

        グローバル辞書が設定されている場合はそれを使用し、
        設定されていない場合は埋め込み辞書またはデフォルトパスからのロードを試みます。

        Raises:
            RuntimeError: 辞書のロードに失敗した場合。
        """

    @staticmethod
    def from_dictionary(dict: Dictionary) -> OpenJTalk:
        """既存の Dictionary オブジェクトを共有してインスタンスを作成します。

        Args:
            dict (Dictionary): `Dictionary` クラスによってロード済みの辞書オブジェクト。

        Returns:
            OpenJTalk: 初期化されたインスタンス。
        """

    @staticmethod
    def from_path(dict_dir: str, user_dict: str | None = None) -> OpenJTalk:
        """指定されたパスから辞書をロードしてインスタンスを作成します。

        Args:
            dict_dir (str): システム辞書のディレクトリパス。
            user_dict (Optional[str], optional): ユーザー辞書のファイルパス。

        Returns:
            OpenJTalk: 初期化されたインスタンス。
        """

    def run_frontend(self, text: str) -> list[NjdFeature]:
        """テキストを解析し、NJD特徴量のリストを返します。"""

    def run_frontend_detailed(
        self, text: str
    ) -> tuple[list[NjdFeature], list[MecabMorph]]:
        """
        テキストを詳細に解析し、NJD特徴量とMeCab形態素情報の両方を返します。

        Args:
            text (str): 解析対象のテキスト。

        Returns:
            Tuple[List[PyNjdFeature], List[MecabMorph]]:
                NJD特徴量のリストと、詳細な形態素情報のリストのタプル。
        """

    def extract_fullcontext(self, text: str) -> list[Label]:
        """フルコンテキストラベルを構造化データとして抽出します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[Label]: フルコンテキストラベルのリスト。
        """

    def extract_fullcontext_string(self, text: str) -> list[str]:
        """フルコンテキストラベルを抽出します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: フルコンテキストラベルのリスト。
        """

    def g2p(self, text: str) -> list[str]:
        """テキストを音素リストに変換します。

        pyopenjtalk のような音素文字列を得るためには、
        `phonemes = " ".join(phonemes)` をしてください。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: 音素記号のリスト (例: `['k', 'o', 'N', ...]`)。
        """

    def g2p_detailed(self, text: str) -> list[str]:
        """より詳細な G2P 変換。
        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        pyopenjtalk のような音素文字列を得るためには、
        `phonemes = " ".join(phonemes)` をしてください。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: 音素記号のリスト (例: `['k', 'o', 'N', ...]`)。
        """

    def g2k(self, text: str) -> str:
        """テキストをカタカナ読みに変換します。

        pyopenjtalk と同様に、記号や未知語などは元の表記のまま出力されます。

        Args:
            text (str): 入力テキスト。

        Returns:
            str: カタカナ文字列 (例: `"コンニチワ"`)。
        """

    def g2k_per_word(self, text: str) -> list[str]:
        """入力テキストを単語（形態素）ごとのカタカナリストに変換します。

        pyopenjtalk と同様に、記号や未知語などは元の表記のまま出力されます。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: カタカナ文字列のリスト。
        """

    def g2p_prosody(
        self, text: str, format: ProsodyFormat = ProsodyFormat.Default
    ) -> list[str]:
        """
        入力テキストを [ProsodyFormat] の設定をもとにプロソディ記号付き音素リストに変換します。

        出力には、共通して以下のプロソディ記号が含まれます。

        | 記号 | 意味 | 出現位置 |
        | :--- | :--- | :--- |
        | `^` | 発話の開始 (BOS) | 文頭 |
        | `$` | 発話の終結 (EOS) | 文末 |
        | `?` | 疑問文の終結 (？) | 文中 |
        | `!` | 感嘆の終結 (独自拡張) | 文中 |
        | `_` | ポーズ・読点 (、) | 文中 |
        | `#` | アクセント句境界 | 文中 |
        | `{...}` | 未知語 | 文中 |

        日本語のアクセントについて: [tdmelodic 利用マニュアル/予備知識](https://tdmelodic.readthedocs.io/ja/latest/pages/introduction.html)

        ## [ProsodyFormat.Default]

        出力には上記のものに追加して、以下のプロソディ記号が含まれます。

        | 記号 | 意味 | 出現位置 |
        | :--- | :--- | :--- |
        | `[` | ピッチ上昇 (句頭) | 句の開始付近 |
        | `]` | ピッチ下降 (アクセント核) | 核モーラの直後 |

        記号 `[` および `]` は、tdmelodic 等で一般的なアクセント記法に基づいています。
        "Prosodic Features Control by Symbols as Input of Sequence-to-Sequence Acoustic Modeling for Neural TTS"
        (Kurihara et al., 2021) のアルゴリズムにおける `^` および `!` に相当します。

        ## [ProsodyFormat.Prefix]

        ピッチ上昇/下降記号 (`[` や `]`) を使用せず、各音素のプレフィックスとしてピッチの高低を付与します。
        - `H_` : ピッチが高い (High)
        - `L_` : ピッチが低い (Low)

        音素ごとにピッチが明示されます。
        例: `"青い空"` -> `["^", "L_a", "H_o", "L_i", "#", "H_s", "H_o", "L_r", "L_a", "$"]`

        ## [ProsodyFormat.Numeric]

        各音素のサフィックスとして、ピッチの高低を数値で付与します。
        - `:1` : ピッチが高い (High)
        - `:0` : ピッチが低い (Low)

        例: `"青い空"` -> `["^", "a:0", "o:1", "i:0", "#", "s:1", "o:1", "r:0", "a:0", "$"]`

        Args:
            text (str): 入力テキスト。
            format (ProsodyFormat): 出力フォーマットの指定。

        Returns:
            List[str]: プロソディ記号付き音素記号のリスト (例: `['^', 'a', '[', ...]`)。
        """

    def g2p_per_word(self, text: str) -> list[list[str]]:
        """テキストを単語ごとに区切られた音素リストに変換します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[List[str]]: 単語ごとの音素リストのリスト。
            (例: `[['k', 'o', 'N', ...], ['pau'], ['s', 'e', 'k', 'a', 'i']]`)
        """

    def g2p_mapping(self, text: str) -> list[WordPhonemeMap]:
        """入力テキストの形態素ごとの音素マッピングを返します。
        MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。

        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        Args:
            text (str): 入力テキスト。

        Returns:
            List[WordPhonemeMap]: 単語と音素のマッピングオブジェクトのリスト。
        """

    def g2p_mapping_detailed(self, text: str) -> list[WordPhonemeDetail]:
        """
        入力テキストの形態素ごとの音素マッピングを、NJD が付与する情報を含めて返します。

        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        Args:
            text (str): 入力テキスト。

        Returns:
            List[WordPhonemeDetail]: NJD情報と音素のマッピングオブジェクトのリスト。
        """

    def g2p_mapping_prosody(self, text: str) -> list[WordPhonemeProsody]:
        """入力テキストを解析し、形態素ごとの詳細な言語情報と、プロソディ記号付き音素をマッピングして取得します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[WordPhonemeProsody]: 形態素ごとのプロソディ情報とNJD特徴量を保持する構造体のリスト。
        """

    def run_frontend_batch(self, texts: list[str]) -> list[list[NjdFeature]]:
        """複数のテキストに対して `run_frontend` を実行します。

        Args:
            texts (List[str]): 解析対象のテキストのリスト。

        Returns:
            List[List[PyNjdFeature]]: 各テキストに対応するNJD特徴量リストのリスト。
        """

    def run_frontend_detailed_batch(
        self, texts: list[str]
    ) -> list[tuple[list[NjdFeature], list[MecabMorph]]]:
        """複数のテキストに対して `run_frontend_detailed` を実行します。

        Args:
            texts (List[str]): 解析対象のテキストのリスト。

        Returns:
            List[Tuple[List[PyNjdFeature], List[MecabMorph]]]:
                各テキストに対応する（NJD特徴量リスト, MeCab形態素情報リスト）のタプルのリスト。
        """

    def g2p_batch(self, texts: list[str]) -> list[list[str]]:
        """複数のテキストに対して `g2p` を実行します。

        Python の GIL を解放してバッチ処理を行います。大量のテキストデータセットの前処理などに最適です。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 各テキストに対応する音素リストのリスト。
        """

    def g2p_detailed_batch(self, texts: list[str]) -> list[list[str]]:
        """複数のテキストに対して詳細な G2P 変換を実行します。

        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        Python の GIL を解放してバッチ処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 各テキストに対応する詳細な音素リストのリスト。
        """

    def g2k_batch(self, texts: list[str]) -> list[str]:
        """複数のテキストをカタカナ読みに変換します。

        Python の GIL を解放してバッチ処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[str]: 各テキストに対応するカタカナ文字列のリスト。
        """

    def g2k_per_word_batch(self, texts: list[str]) -> list[list[str]]:
        """複数の入力テキストを単語（形態素）ごとのカタカナリストに変換します。

        Python の GIL を解放してバッチ処理を行います。

        pyopenjtalk と同様に、記号や未知語などは元の表記のまま出力されます。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 単語ごとのカタカナ文字列のリストのリスト。
        """

    def g2p_prosody_batch(
        self, texts: list[str], format: ProsodyFormat = ProsodyFormat.Default
    ) -> list[list[str]]:
        """
        複数の入力テキストを [ProsodyFormat] の設定をもとにプロソディ記号付き音素リストのリストに変換します。

        出力には、共通して以下のプロソディ記号が含まれます。

        | 記号 | 意味 | 出現位置 |
        | :--- | :--- | :--- |
        | `^` | 発話の開始 (BOS) | 文頭 |
        | `$` | 発話の終結 (EOS) | 文末 |
        | `?` | 疑問文の終結 (？) | 文中 |
        | `!` | 感嘆の終結 (独自拡張) | 文中 |
        | `_` | ポーズ・読点 (、) | 文中 |
        | `#` | アクセント句境界 | 文中 |
        | `{...}` | 未知語 | 文中 |

        日本語のアクセントについて: [tdmelodic 利用マニュアル/予備知識](https://tdmelodic.readthedocs.io/ja/latest/pages/introduction.html)

        ## [ProsodyFormat.Default]

        出力には上記のものに追加して、以下のプロソディ記号が含まれます。

        | 記号 | 意味 | 出現位置 |
        | :--- | :--- | :--- |
        | `[` | ピッチ上昇 (句頭) | 句の開始付近 |
        | `]` | ピッチ下降 (アクセント核) | 核モーラの直後 |

        記号 `[` および `]` は、tdmelodic 等で一般的なアクセント記法に基づいています。
        "Prosodic Features Control by Symbols as Input of Sequence-to-Sequence Acoustic Modeling for Neural TTS"
        (Kurihara et al., 2021) のアルゴリズムにおける `^` および `!` に相当します。

        ## [ProsodyFormat.Prefix]

        ピッチ上昇/下降記号 (`[` や `]`) を使用せず、各音素のプレフィックスとしてピッチの高低を付与します。
        - `H_` : ピッチが高い (High)
        - `L_` : ピッチが低い (Low)

        音素ごとにピッチが明示されます。
        例: `"青い空"` -> `["^", "L_a", "H_o", "L_i", "#", "H_s", "H_o", "L_r", "L_a", "$"]`

        ## [ProsodyFormat.Numeric]

        各音素のサフィックスとして、ピッチの高低を数値で付与します。
        - `:1` : ピッチが高い (High)
        - `:0` : ピッチが低い (Low)

        例: `"青い空"` -> `["^", "a:0", "o:1", "i:0", "#", "s:1", "o:1", "r:0", "a:0", "$"]`

        Args:
            texts (List[str]): 入力テキストのリスト。
            format (ProsodyFormat): 出力フォーマットの指定。

        Returns:
            List[List[str]]: プロソディ記号付き音素記号のリストのリスト (例: `[['^', 'a', '[', ...], ...]`)。
        """

    def g2p_per_word_batch(self, texts: list[str]) -> list[list[list[str]]]:
        """複数のテキストを単語ごとに区切られた音素リストに変換します。

        Python の GIL を解放してバッチ処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[List[str]]]: 3次元リスト (テキスト -> 単語 -> 音素リスト)。
        """

    def g2p_mapping_batch(self, texts: list[str]) -> list[list[WordPhonemeMap]]:
        """入力テキストの形態素ごとの音素マッピング（詳細版）をバッチ処理で返します。

        MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。

        注意:
            Rust 側での解析計算は並列・バッチ化されますが、最終的な Python オブジェクトへの変換は
            メインスレッド (GIL下) で行われるため、オブジェクト数が多い場合は変換コストが発生します。

        Python の GIL を解放してバッチ処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[WordPhonemeMap]]: 各テキストに対応する詳細なマッピング情報のリスト。
        """

    def g2p_mapping_detailed_batch(
        self, texts: list[str]
    ) -> list[list[WordPhonemeDetail]]:
        """
        入力テキストのリストに対して `g2p_mapping_detailed` を並列に実行します。

        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[WordPhonemeDetail]]: NJD情報と音素のマッピングオブジェクトのリスト。
        """

    def g2p_mapping_prosody_batch(
        self, texts: list[str]
    ) -> list[list[WordPhonemeProsody]]:
        """複数のテキストに対して `g2p_mapping_prosody` を並行に実行します。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[WordPhonemeProsody]]: 形態素ごとのプロソディ情報とNJD特徴量を保持する構造体のリストのリスト。
        """

    def extract_fullcontext_batch(self, texts: list[str]) -> list[list[Label]]:
        """複数のテキストからフルコンテキストラベルを構造化データとして抽出します。

        Python の GIL を解放してバッチ処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[Label]]: 各テキストに対応するフルコンテキストラベルのリストのリスト。
        """

    def extract_fullcontext_string_batch(self, texts: list[str]) -> list[list[str]]:
        """複数のテキストからフルコンテキストラベル文字列を抽出します。

        Python の GIL を解放してバッチ処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 各テキストに対応するフルコンテキストラベルのリストのリスト。
        """

class Haqumei:
    """`OpenJTalk` を拡張した G2P エンジン。

    [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) の辞書を使用しています。

    `g2p_**`の実装において、フルコンテキストラベルを経由せず、JPCommon で構築された内部ポインタを追って
    g2p を行うため、他の Open JTalk バインディング実装より若干高速です。
    また、他のバインディングにない以下の関数が実装されています。
    - `g2p_per_word`: テキストを単語ごとに区切られた音素リストに変換します。
    - `g2p_candidates`: 読みが分かれる箇所で複数の候補を返します。

    [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) に実装されている、
    Rustで実装された以下の処理によって `OpenJTalk` よりも精度の高い読み推定を行います。
    - フィラーアクセントの修正
    - 文脈に応じた `何` の読み分け予測 (NaniPredictor)
    - 長母音、重母音、撥音や品詞「特殊・マス」に関するアクセント核の修正
    - 踊り字（々）と一の字点（ゝ、ゞ、ヽ、ヾ）の読みの修正

    Examples:

    >>> haqumei = Haqumei()
    >>> haqumei.g2k("何を言っても何の問題もありません。")
    'ナニヲイッテモナンノモンダイモアリマセン。'
    """

    def __init__(
        self,
        normalize_unicode: UnicodeNormalization = UnicodeNormalization.None_,
        *,
        use_read_as_pron: bool = False,
        revert_long_vowels: bool = False,
        revert_yotsugana: bool = False,
        normalize_iu: IuPronunciation = IuPronunciation.None_,
        modify_filler_accent: bool = True,
        predict_nani: bool = True,
        predict_kana_english: bool = True,
        modify_context_reading: bool = True,
        modify_old_province_yomi: bool = True,
        restore_loanword_kana: bool = True,
        protect_user_dict_readings: bool = False,
        read_unknown_kanji: bool = True,
        modify_numeral_reading: bool = True,
        split_prefix_accent_phrase: bool = True,
        retreat_acc_nuc: bool = True,
        modify_acc_after_chaining: bool = True,
        process_odoriji: bool = True,
        use_allophones: bool = False,
        split_n_allophones: bool = False,
        split_n_before_palatal_affricate: bool = False,
        split_n_before_r: bool = False,
        split_q_allophones: bool = False,
        enable_final_glottal_stop: bool = False,
    ) -> None:
        """新しい Haqumei インスタンスを初期化します。"""

    @staticmethod
    def from_dictionary(dict: Dictionary) -> Haqumei:
        """既存の Dictionary オブジェクトを使用してインスタンスを初期化します。

        オプションはすべてデフォルトになります。変更したい場合は
        `Haqumei(...)` を使ってください。

        Args:
            dict (Dictionary): ロード済みの辞書オブジェクト。

        Returns:
            Haqumei: 初期化されたインスタンス。
        """

    def run_frontend(self, text: str) -> list[NjdFeature]:
        """テキストを解析し、NJD特徴量のリストを返します。"""

    def run_frontend_detailed(
        self, text: str
    ) -> tuple[list[NjdFeature], list[MecabMorph]]:
        """
        テキストを詳細に解析し、NJD特徴量とMeCab形態素情報の両方を返します。

        Args:
            text (str): 解析対象のテキスト。

        Returns:
            Tuple[List[PyNjdFeature], List[MecabMorph]]
                NJD特徴量のリストと、詳細な形態素情報のリストのタプル。
        """

    def extract_fullcontext(self, text: str) -> list[Label]:
        """フルコンテキストラベルを構造化データとして抽出します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[Label]: フルコンテキストラベルのリスト。
        """

    def extract_fullcontext_string(self, text: str) -> list[str]:
        """フルコンテキストラベルを抽出します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: フルコンテキストラベルのリスト。
        """

    def g2p(self, text: str) -> list[str]:
        """テキストを音素リストに変換します。

        pyopenjtalk のような音素文字列を得るためには、
        `phonemes = " ".join(phonemes)` をしてください。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: 音素記号のリスト。
        """

    def g2p_detailed(self, text: str) -> list[str]:
        """より詳細な G2P 変換。
        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        pyopenjtalk のような音素文字列を得るためには、
        `phonemes = " ".join(phonemes)` をしてください。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: 音素記号のリスト (例: `['k', 'o', 'N', ...]`)。
        """

    def g2k(self, text: str) -> str:
        """テキストをカタカナ読みに変換します。

        Args:
            text (str): 入力テキスト。

        Returns:
            str: カタカナ文字列。
        """

    def g2k_per_word(self, text: str) -> list[str]:
        """入力テキストを単語（形態素）ごとのカタカナリストに変換します。

        pyopenjtalk と同様に、記号や未知語などは元の表記のまま出力されます。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: カタカナ文字列のリスト (例: `["コンニチワ"]`)。
        """

    def g2p_per_word(self, text: str) -> list[list[str]]:
        """テキストを単語ごとに区切られた音素リストに変換します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[List[str]]: 単語ごとの音素リストのリスト。
        """

    def g2p_prosody(
        self, text: str, format: ProsodyFormat = ProsodyFormat.Default
    ) -> list[str]:
        """
        入力テキストを [ProsodyFormat] の設定をもとにプロソディ記号付き音素リストに変換します。

        出力には、共通して以下のプロソディ記号が含まれます。

        | 記号 | 意味 | 出現位置 |
        | :--- | :--- | :--- |
        | `^` | 発話の開始 (BOS) | 文頭 |
        | `$` | 発話の終結 (EOS) | 文末 |
        | `?` | 疑問文の終結 (？) | 文中 |
        | `!` | 感嘆の終結 (独自拡張) | 文中 |
        | `_` | ポーズ・読点 (、) | 文中 |
        | `#` | アクセント句境界 | 文中 |
        | `{...}` | 未知語 | 文中 |

        日本語のアクセントについて: [tdmelodic 利用マニュアル/予備知識](https://tdmelodic.readthedocs.io/ja/latest/pages/introduction.html)

        ## [ProsodyFormat.Default]

        出力には上記のものに追加して、以下のプロソディ記号が含まれます。

        | 記号 | 意味 | 出現位置 |
        | :--- | :--- | :--- |
        | `[` | ピッチ上昇 (句頭) | 句の開始付近 |
        | `]` | ピッチ下降 (アクセント核) | 核モーラの直後 |

        記号 `[` および `]` は、tdmelodic 等で一般的なアクセント記法に基づいています。
        "Prosodic Features Control by Symbols as Input of Sequence-to-Sequence Acoustic Modeling for Neural TTS"
        (Kurihara et al., 2021) のアルゴリズムにおける `^` および `!` に相当します。

        ## [ProsodyFormat.Prefix]

        ピッチ上昇/下降記号 (`[` や `]`) を使用せず、各音素のプレフィックスとしてピッチの高低を付与します。
        - `H_` : ピッチが高い (High)
        - `L_` : ピッチが低い (Low)

        音素ごとにピッチが明示されます。
        例: `"青い空"` -> `["^", "L_a", "H_o", "L_i", "#", "H_s", "H_o", "L_r", "L_a", "$"]`

        ## [ProsodyFormat.Numeric]

        各音素のサフィックスとして、ピッチの高低を数値で付与します。
        - `:1` : ピッチが高い (High)
        - `:0` : ピッチが低い (Low)

        例: `"青い空"` -> `["^", "a:0", "o:1", "i:0", "#", "s:1", "o:1", "r:0", "a:0", "$"]`

        Args:
            text (str): 入力テキスト。
            format (ProsodyFormat): 出力フォーマットの指定。

        Returns:
            List[str]: プロソディ記号付き音素記号のリスト (例: `['^', 'a', '[', ...]`)。
        """

    def g2p_mapping(self, text: str) -> list[WordPhonemeMap]:
        """入力テキストの形態素ごとの音素マッピングを返します。
        MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。

        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        Args:
            text (str): 入力テキスト。

        Returns:
            List[WordPhonemeMap]: 単語と音素のマッピングオブジェクトのリスト。
        """

    def g2p_mapping_detailed(self, text: str) -> list[WordPhonemeDetail]:
        """
        入力テキストの形態素ごとの音素マッピングを、NJD が付与する情報を含めて返します。

        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        Args:
            text (str): 入力テキスト。

        Returns:
            List[WordPhonemeDetail]: NJD情報と音素のマッピングオブジェクトのリスト。
        """

    def g2p_mapping_prosody(self, text: str) -> list[WordPhonemeProsody]:
        """入力テキストを解析し、形態素ごとの詳細な言語情報と、プロソディ記号付き音素をマッピングして取得します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[WordPhonemeProsody]: 形態素ごとのプロソディ情報とNJD特徴量を保持する構造体のリスト。
        """

    def g2p_candidates(
        self, text: str, options: CandidateOptions | None = None
    ) -> Candidates:
        """読みの候補を、形態素ごとの音素マッピングとして返します。

        経路が 2 通り以上ある区間を分岐点として、読みの候補を複数返します。読みを
        1 つに決める `g2p_mapping` と違い、分かれたまま受け取れます。

        どの候補も形態素列を 1 つに決めてから解析するので、返る候補の中身は
        `g2p_mapping` の返り値と同じ形になります。候補は MeCab のラティスからだけ来るので、辞書のエントリが
        分かれていない読み、未知語の読み、読みを決める補正が書き込む箇所
        (「何」・数字・文脈読み) は候補になりません。

        Args:
            text (str): 入力テキスト。
            options (CandidateOptions | None): 候補の作り方。省略すると既定値。

        Returns:
            Candidates: 先頭が `g2p_mapping` と一致する候補集合。
        """

    def g2p_candidates_detailed(
        self, text: str, options: CandidateOptions | None = None
    ) -> CandidatesDetail:
        """読みの候補を、NJD が付与する情報を含めて返します。

        Args:
            text (str): 入力テキスト。
            options (CandidateOptions | None): 候補の作り方。省略すると既定値。

        Returns:
            CandidatesDetail: 先頭が `g2p_mapping_detailed` と一致する候補集合。
        """

    def g2p_candidates_prosody(
        self, text: str, options: CandidateOptions | None = None
    ) -> CandidatesProsody:
        """読みの候補を、プロソディ記号付きの音素として返します。

        アクセント句の切れ目は候補ごとに変わるので、音素が同じでもプロソディが
        違えば別の候補として残ります。

        Args:
            text (str): 入力テキスト。
            options (CandidateOptions | None): 候補の作り方。省略すると既定値。

        Returns:
            CandidatesProsody: 先頭が `g2p_mapping_prosody` と一致する候補集合。
        """

    def g2p_candidates_batch(
        self, texts: list[str], options: CandidateOptions | None = None
    ) -> list[Candidates]:
        """複数のテキストに対して `g2p_candidates` を実行します。

        Args:
            texts (List[str]): 解析対象のテキストのリスト。
            options (CandidateOptions | None): 候補の作り方。省略すると既定値。

        Returns:
            List[Candidates]: 各テキストに対応する候補集合のリスト。
        """

    def run_frontend_batch(self, texts: list[str]) -> list[list[NjdFeature]]:
        """複数のテキストに対して `run_frontend` を実行します。

        Args:
            texts (List[str]): 解析対象のテキストのリスト。

        Returns:
            List[List[PyNjdFeature]]: 各テキストに対応するNJD特徴量リストのリスト。
        """

    def run_frontend_detailed_batch(
        self, texts: list[str]
    ) -> list[tuple[list[NjdFeature], list[MecabMorph]]]:
        """複数のテキストに対して `run_frontend_detailed` を実行します。

        Args:
            texts (List[str]): 解析対象のテキストのリスト。

        Returns:
            List[Tuple[List[PyNjdFeature], List[MecabMorph]]]:
                各テキストに対応する（NJD特徴量リスト, MeCab形態素情報リスト）のタプルのリスト。
        """

    def g2p_batch(self, texts: list[str]) -> list[list[str]]:
        """複数のテキストに対して `g2p` を実行します。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 各テキストに対応する音素リストのリスト。
        """

    def g2p_detailed_batch(self, texts: list[str]) -> list[list[str]]:
        """すべてのトークンを保持する詳細な G2P 変換のバッチ処理。

        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 各テキストに対応する詳細な音素リストのリスト。
        """

    def g2k_batch(self, texts: list[str]) -> list[str]:
        """カタカナ変換のバッチ処理。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[str]: 各テキストに対応するカタカナ文字列のリスト。
        """

    def g2k_per_word_batch(self, texts: list[str]) -> list[list[str]]:
        """複数の入力テキストを単語（形態素）ごとのカタカナリストに変換します。

        Python の GIL を解放してバッチ処理を行います。

        pyopenjtalk と同様に、記号や未知語などは元の表記のまま出力されます。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 単語ごとのカタカナ文字列のリストのリスト。
        """

    def g2p_prosody_batch(
        self, texts: list[str], format: ProsodyFormat = ProsodyFormat.Default
    ) -> list[list[str]]:
        """
        複数の入力テキストを [ProsodyFormat] の設定をもとにプロソディ記号付き音素リストのリストに変換します。

        出力には、共通して以下のプロソディ記号が含まれます。

        | 記号 | 意味 | 出現位置 |
        | :--- | :--- | :--- |
        | `^` | 発話の開始 (BOS) | 文頭 |
        | `$` | 発話の終結 (EOS) | 文末 |
        | `?` | 疑問文の終結 (？) | 文中 |
        | `!` | 感嘆の終結 (独自拡張) | 文中 |
        | `_` | ポーズ・読点 (、) | 文中 |
        | `#` | アクセント句境界 | 文中 |
        | `{...}` | 未知語 | 文中 |

        日本語のアクセントについて: [tdmelodic 利用マニュアル/予備知識](https://tdmelodic.readthedocs.io/ja/latest/pages/introduction.html)

        ## [ProsodyFormat.Default]

        出力には上記のものに追加して、以下のプロソディ記号が含まれます。

        | 記号 | 意味 | 出現位置 |
        | :--- | :--- | :--- |
        | `[` | ピッチ上昇 (句頭) | 句の開始付近 |
        | `]` | ピッチ下降 (アクセント核) | 核モーラの直後 |

        記号 `[` および `]` は、tdmelodic 等で一般的なアクセント記法に基づいています。
        "Prosodic Features Control by Symbols as Input of Sequence-to-Sequence Acoustic Modeling for Neural TTS"
        (Kurihara et al., 2021) のアルゴリズムにおける `^` および `!` に相当します。

        ## [ProsodyFormat.Prefix]

        ピッチ上昇/下降記号 (`[` や `]`) を使用せず、各音素のプレフィックスとしてピッチの高低を付与します。
        - `H_` : ピッチが高い (High)
        - `L_` : ピッチが低い (Low)

        音素ごとにピッチが明示されます。
        例: `"青い空"` -> `["^", "L_a", "H_o", "L_i", "#", "H_s", "H_o", "L_r", "L_a", "$"]`

        ## [ProsodyFormat.Numeric]

        各音素のサフィックスとして、ピッチの高低を数値で付与します。
        - `:1` : ピッチが高い (High)
        - `:0` : ピッチが低い (Low)

        例: `"青い空"` -> `["^", "a:0", "o:1", "i:0", "#", "s:1", "o:1", "r:0", "a:0", "$"]`

        Args:
            texts (List[str]): 入力テキストのリスト。
            format (ProsodyFormat): 出力フォーマットの指定。

        Returns:
            List[List[str]]: プロソディ記号付き音素記号のリストのリスト (例: `[['^', 'a', '[', ...], ...]`)。
        """

    def g2p_per_word_batch(self, texts: list[str]) -> list[list[list[str]]]:
        """単語ごとに分割された音素リストのバッチ処理。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[List[str]]]: 3次元リスト (テキスト -> 単語 -> 音素リスト)。
        """

    def g2p_mapping_batch(self, texts: list[str]) -> list[list[WordPhonemeMap]]:
        """形態素ごとの音素マッピング（詳細版）のバッチ処理。

        マルチスレッドで処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[WordPhonemeMap]]: 各テキストに対応する詳細なマッピング情報のリスト。
        """

    def g2p_mapping_detailed_batch(
        self, texts: list[str]
    ) -> list[list[WordPhonemeDetail]]:
        """
        入力テキストのリストに対して `g2p_mapping_detailed` を並列に実行します。

        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[WordPhonemeDetail]]: NJD情報と音素のマッピングオブジェクトのリスト。
        """

    def g2p_mapping_prosody_batch(
        self, texts: list[str]
    ) -> list[list[WordPhonemeProsody]]:
        """複数のテキストに対して `g2p_mapping_prosody` を並行に実行します。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[WordPhonemeProsody]]: 形態素ごとのプロソディ情報とNJD特徴量を保持する構造体のリストのリスト。
        """

    def extract_fullcontext_batch(self, texts: list[str]) -> list[list[Label]]:
        """フルコンテキストラベル抽出のバッチ処理。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[Label]]: 各テキストに対応するフルコンテキストラベルのリストのリスト。
        """

    def extract_fullcontext_string_batch(self, texts: list[str]) -> list[list[str]]:
        """フルコンテキストラベル文字列抽出のバッチ処理。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 各テキストに対応するフルコンテキストラベル文字列のリストのリスト。
        """

def update_global_dictionary(dict: Dictionary) -> None:
    """OpenJTalk で使用されるグローバル辞書を更新 (設定) します。

    この関数を呼び出した後、引数なしで `OpenJTalk()` や `Haqumei()` を初期化すると、
    ここで設定した辞書がデフォルトで使用されます。

    既存のインスタンスは、次のメソッド呼び出し時に内部で辞書が更新されます。

    Args:
        dict (Dictionary): 設定する辞書オブジェクト。
    """

def unset_user_dictionary() -> None:
    """グローバル辞書からユーザー辞書設定を解除します。

    システム辞書のみを使用する状態に戻します。
    """
