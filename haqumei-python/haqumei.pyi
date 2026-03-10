from typing import List, Optional

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

class WordPhonemeMap:
    """単語とその音素列の対応関係を表すデータクラス。

    `g2p_mapping` メソッドによって生成されます。
    """

    word: str
    """単語の表層形。"""

    phonemes: List[str]
    """その単語に対応する音素のリスト。"""

class WordPhonemeDetail:
    """単語とその音素列の対応関係を表すデータクラス。

    `g2p_mapping_detailed` メソッドによって生成されます。
    """

    word: str
    """単語の表層形。"""

    phonemes: List[str]
    """その単語に対応する音素のリスト。"""

    is_unknown: bool
    """MeCab が未知語 (`MECAB_UNK_NODE`) と判定したかどうか。"""

    is_ignored: bool
    """`OpenJTalk` のパイプラインで無視される対象かどうか。"""

class Dictionary:
    """OpenJTalk用の辞書データを管理するクラス。

    一度ロードした辞書データをメモリ上で保持します。
    このインスタンスを `OpenJTalk` に渡すことで、
    辞書データのメモリ共有が可能になり、
    Mecab による mmap syscall の時間を削減できます。
    """

    @staticmethod
    def from_path(dict_dir: str, user_dict: Optional[str] = None) -> "Dictionary":
        """指定されたパスから辞書をロードします。

        Args:
            dict_dir (str): システム辞書のディレクトリパス。
            user_dict (Optional[str], optional): ユーザー辞書のファイルパス。デフォルトは None。

        Returns:
            Dictionary: ロードされた辞書オブジェクト。

        Raises:
            RuntimeError: 指定されたパスに辞書が存在しない、またはフォーマットが不正な場合。
        """
        ...

    @staticmethod
    def from_embedded() -> "Dictionary":
        """ライブラリに埋め込まれた辞書データをロードします。

        Returns:
            Dictionary: ロードされた辞書オブジェクト。
        """
        ...

class OpenJTalk:
    """OpenJTalk の機能を提供するラッパークラス。

    [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) の辞書を使用しています。

    `g2p_**`の実装において、フルコンテキストラベルを経由せず、JPCommon で構築された内部ポインタを追って
    g2p を行うため、他の Open JTalk バインディング実装より若干高速です。
    また、他のバインディングにない以下の関数が実装されています。
    - `g2p_per_word`: テキストを単語ごとに区切られた音素リストに変換します。
    - `g2p_mapping`: テキストを解析し、単語と音素のマッピング情報を返します。


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
        ...

    @staticmethod
    def from_dictionary(dict: Dictionary) -> "OpenJTalk":
        """既存の Dictionary オブジェクトを共有してインスタンスを作成します。

        Args:
            dict (Dictionary): `Dictionary` クラスによってロード済みの辞書オブジェクト。

        Returns:
            OpenJTalk: 初期化されたインスタンス。
        """
        ...

    @staticmethod
    def from_path(dict_dir: str, user_dict: Optional[str] = None) -> "OpenJTalk":
        """指定されたパスから辞書をロードしてインスタンスを作成します。

        Args:
            dict_dir (str): システム辞書のディレクトリパス。
            user_dict (Optional[str], optional): ユーザー辞書のファイルパス。

        Returns:
            OpenJTalk: 初期化されたインスタンス。
        """
        ...

    def g2p(self, text: str) -> List[str]:
        """テキストを音素リストに変換します。

        pyopenjtalk のような音素文字列を得るためには、
        `phonemes = " ".join(phonemes)` をしてください。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: 音素記号のリスト (例: `['k', 'o', 'N', ...]`)。
        """
        ...

    def g2p_detailed(self, text: str) -> List[str]:
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
        ...


    def g2p_kana(self, text: str) -> str:
        """テキストをカタカナ読みに変換します。

        pyopenjtalk と同様に、記号や未知語などは元の表記のまま出力されます。

        Args:
            text (str): 入力テキスト。

        Returns:
            str: カタカナ文字列 (例: `"コンニチワ"`)。
        """
        ...

    def g2p_per_word(self, text: str) -> List[List[str]]:
        """テキストを単語ごとに区切られた音素リストに変換します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[List[str]]: 単語ごとの音素リストのリスト。
            (例: `[['k', 'o', 'N', ...], ['pau'], ['s', 'e', 'k', 'a', 'i']]`)
        """
        ...

    def g2p_mapping(self, text: str) -> List[WordPhonemeMap]:
        """テキストを解析し、単語と音素のマッピング情報を返します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[WordPhonemeMap]: 単語と音素のマッピングオブジェクトのリスト。
        """
        ...

    def g2p_mapping_detailed(self, text: str) -> List[WordPhonemeDetail]:
        """入力テキストの形態素ごとの音素マッピングを返します。
        MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。

        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        Args:
            text (str): 入力テキスト。

        Returns:
            List[WordPhonemeDetail]: 単語と音素のマッピングオブジェクトのリスト。
        """
        ...

    def run_frontend(self, text: str) -> List[NjdFeature]:
        """
        Args:
            text (str): 入力テキスト。

        Returns:
            List[NjdFeature]: 特徴量のリスト。
        """
        ...

    def extract_fullcontext(self, text: str) -> List[str]:
        """フルコンテキストラベルを抽出します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: フルコンテキストラベルのリスト。
        """
        ...

    def g2p_batch(self, texts: List[str]) -> List[List[str]]:
        """複数のテキストに対して `g2p` を実行します。

        Python の GIL を解放してバッチ処理を行います。大量のテキストデータセットの前処理などに最適です。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 各テキストに対応する音素リストのリスト。
        """
        ...

    def g2p_detailed_batch(self, texts: List[str]) -> List[List[str]]:
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
        ...

    def g2p_kana_batch(self, texts: List[str]) -> List[str]:
        """複数のテキストをカタカナ読みに変換します。

        Python の GIL を解放してバッチ処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[str]: 各テキストに対応するカタカナ文字列のリスト。
        """
        ...

    def g2p_per_word_batch(self, texts: List[str]) -> List[List[List[str]]]:
        """複数のテキストを単語ごとに区切られた音素リストに変換します。

        Python の GIL を解放してバッチ処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[List[str]]]: 3次元リスト (テキスト -> 単語 -> 音素リスト)。
        """
        ...

    def g2p_mapping_batch(self, texts: List[str]) -> List[List[WordPhonemeMap]]:
        """複数のテキストを解析し、単語と音素のマッピング情報を返します。

        注意:
            Rust 側での解析計算は並列・バッチ化されますが、最終的な Python オブジェクトへの変換は
            メインスレッド (GIL下) で行われるため、オブジェクト数が多い場合は変換コストが発生します。

        Python の GIL を解放してバッチ処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[WordPhonemeMap]]: 各テキストに対応するマッピング情報のリスト。
        """
        ...

    def g2p_mapping_detailed_batch(self, texts: List[str]) -> List[List[WordPhonemeDetail]]:
        """入力テキストの形態素ごとの音素マッピング（詳細版）をバッチ処理で返します。

        MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。

        注意:
            Rust 側での解析計算は並列・バッチ化されますが、最終的な Python オブジェクトへの変換は
            メインスレッド (GIL下) で行われるため、オブジェクト数が多い場合は変換コストが発生します。

        Python の GIL を解放してバッチ処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[WordPhonemeDetail]]: 各テキストに対応する詳細なマッピング情報のリスト。
        """
        ...

    def extract_fullcontext_batch(self, texts: List[str]) -> List[List[str]]:
        """複数のテキストからフルコンテキストラベルを抽出します。

        Python の GIL を解放してバッチ処理を行います。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 各テキストに対応するフルコンテキストラベルのリストのリスト。
        """
        ...

class Haqumei:
    """`OpenJTalk` を拡張した G2P エンジン。

    [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) の辞書を使用しています。

    `g2p_**`の実装において、フルコンテキストラベルを経由せず、JPCommon で構築された内部ポインタを追って
    g2p を行うため、他の Open JTalk バインディング実装より若干高速です。
    また、他のバインディングにない以下の関数が実装されています。
    - `g2p_per_word`: テキストを単語ごとに区切られた音素リストに変換します。
    - `g2p_mapping`: テキストを解析し、単語と音素のマッピング情報を返します。

    [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) に実装されている、
    Rustで実装された以下の処理によって `OpenJTalk` よりも精度の高い読み推定を行います。
    - フィラーアクセントの修正
    - 文脈に応じた `何` の読み分け予測 (NaniPredictor)
    - 長母音、重母音、撥音や品詞「特殊・マス」に関するアクセント核の修正
    - 踊り字（々）と一の字点（ゝ、ゞ、ヽ、ヾ）の読みの修正

    Examples:

    >>> haqumei = Haqumei()
    >>> haqumei.g2p_kana("何を言っても何の問題もありません。")
    'ナニヲイッテモナンノモンダイモアリマセン。'
    """

    def __init__(
        self,
        normalize_unicode: bool = True,
        modify_filler_accent: bool = True,
        predict_nani: bool = False,
        modify_kanji_yomi: bool = False,
        retreat_acc_nuc: bool = True,
        modify_acc_after_chaining: bool = True,
        process_odoriji: bool = True,
    ) -> None:
        """新しい Haqumei インスタンスを初期化します。"""
        ...

    @staticmethod
    def from_dictionary(dict: Dictionary) -> "Haqumei":
        """既存の Dictionary オブジェクトを使用してインスタンスを初期化します。

        Args:
            dict (Dictionary): ロード済みの辞書オブジェクト。

        Returns:
            Haqumei: 初期化されたインスタンス。
        """
        ...

    def g2p(self, text: str) -> List[str]:
        """テキストを音素リストに変換します。

        pyopenjtalk のような音素文字列を得るためには、
        `phonemes = " ".join(phonemes)` をしてください。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: 音素記号のリスト。
        """
        ...

    def g2p_detailed(self, text: str) -> List[str]:
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
        ...

    def g2p_kana(self, text: str) -> str:
        """テキストをカタカナ読みに変換します。

        Args:
            text (str): 入力テキスト。

        Returns:
            str: カタカナ文字列。
        """
        ...

    def g2p_per_word(self, text: str) -> List[List[str]]:
        """テキストを単語ごとに区切られた音素リストに変換します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[List[str]]: 単語ごとの音素リストのリスト。
        """
        ...

    def g2p_mapping(self, text: str) -> List[WordPhonemeMap]:
        """テキストを解析し、単語と音素のマッピング情報を返します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[WordPhonemeMap]: 単語と音素のマッピングオブジェクトのリスト。
        """
        ...

    def g2p_mapping_detailed(self, text: str) -> List[WordPhonemeDetail]:
        """入力テキストの形態素ごとの音素マッピングを返します。
        MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。

        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        Args:
            text (str): 入力テキスト。

        Returns:
            List[WordPhonemeDetail]: 単語と音素のマッピングオブジェクトのリスト。
        """
        ...

    def run_frontend(self, text: str) -> List[NjdFeature]:
        """詳細な特徴量 (NJDFeature) を取得します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[NjdFeature]: 特徴量のリスト。
        """
        ...

    def extract_fullcontext(self, text: str) -> List[str]:
        """フルコンテキストラベルを抽出します。

        Args:
            text (str): 入力テキスト。

        Returns:
            List[str]: フルコンテキストラベルのリスト。
        """
        ...

    def g2p_batch(self, texts: List[str]) -> List[List[str]]:
        """複数のテキストに対して `g2p` を実行します。

        `modify_kanji_yomi` が無効な場合、マルチスレッドで処理を行います。
        有効な場合は、シングルスレッドでの逐次処理にフォールバックします。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 各テキストに対応する音素リストのリスト。
        """
        ...

    def g2p_detailed_batch(self, texts: List[str]) -> List[List[str]]:
        """すべてのトークンを保持する詳細な G2P 変換のバッチ処理。

        - 既知語: 通常の音素列 (読点などは `pau`)
        - 未知語: `unk`
        - 空白等: `sp` (Space)

        `modify_kanji_yomi` が無効な場合、マルチスレッドで処理を行います。
        有効な場合は、シングルスレッドでの逐次処理にフォールバックします。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 各テキストに対応する詳細な音素リストのリスト。
        """
        ...

    def g2p_kana_batch(self, texts: List[str]) -> List[str]:
        """カタカナ変換のバッチ処理。

        `modify_kanji_yomi` が無効な場合、マルチスレッドで処理を行います。
        有効な場合は、シングルスレッドでの逐次処理にフォールバックします。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[str]: 各テキストに対応するカタカナ文字列のリスト。
        """
        ...

    def g2p_per_word_batch(self, texts: List[str]) -> List[List[List[str]]]:
        """単語ごとに分割された音素リストのバッチ処理。

        `modify_kanji_yomi` が無効な場合、マルチスレッドで処理を行います。
        有効な場合は、シングルスレッドでの逐次処理にフォールバックします。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[List[str]]]: 3次元リスト (テキスト -> 単語 -> 音素リスト)。
        """
        ...

    def g2p_mapping_batch(self, texts: List[str]) -> List[List[WordPhonemeMap]]:
        """形態素ごとの音素マッピングのバッチ処理。

        `modify_kanji_yomi` が無効な場合、マルチスレッドで処理を行います。
        有効な場合は、シングルスレッドでの逐次処理にフォールバックします。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[WordPhonemeMap]]: 各テキストに対応するマッピング情報のリスト。
        """
        ...

    def g2p_mapping_detailed_batch(self, texts: List[str]) -> List[List[WordPhonemeDetail]]:
        """形態素ごとの音素マッピング（詳細版）のバッチ処理。

        `modify_kanji_yomi` が無効な場合、マルチスレッドで処理を行います。
        有効な場合は、シングルスレッドでの逐次処理にフォールバックします。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[WordPhonemeDetail]]: 各テキストに対応する詳細なマッピング情報のリスト。
        """
        ...

    def extract_fullcontext_batch(self, texts: List[str]) -> List[List[str]]:
        """フルコンテキストラベル抽出のバッチ処理。

        `modify_kanji_yomi` が無効な場合、マルチスレッドで処理を行います。
        有効な場合は、シングルスレッドでの逐次処理にフォールバックします。

        Args:
            texts (List[str]): 入力テキストのリスト。

        Returns:
            List[List[str]]: 各テキストに対応するフルコンテキストラベルのリストのリスト。
        """
        ...

def update_global_dictionary(dict: Dictionary) -> None:
    """OpenJTalk で使用されるグローバル辞書を更新 (設定) します。

    この関数を呼び出した後、引数なしで `OpenJTalk()` や `Haqumei()` を初期化すると、
    ここで設定した辞書がデフォルトで使用されます。

    既存のインスタンスは、次のメソッド呼び出し時に内部で辞書が更新されます。

    Args:
        dict (Dictionary): 設定する辞書オブジェクト。
    """
    ...

def unset_user_dictionary() -> None:
    """グローバル辞書からユーザー辞書設定を解除します。

    システム辞書のみを使用する状態に戻します。
    """
    ...
