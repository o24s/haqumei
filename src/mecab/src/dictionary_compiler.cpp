//  MeCab -- Yet Another Part-of-Speech and Morphological Analyzer
//
//  Copyright(C) 2001-2006 Taku Kudo <taku@chasen.org>
//  Copyright(C) 2004-2006 Nippon Telegraph and Telephone Corporation
#include <iostream>
#include <map>
#include <vector>
#include <string>
#include "char_property.h"
#include "common.h"
#include "connector.h"
#include "dictionary.h"
#include "dictionary_rewriter.h"
#include "feature_index.h"
#include "mecab.h"
#include "param.h"

#ifdef HAVE_CONFIG_H
#include "config.h"
#endif

namespace MeCab {

bool quiet_mode = false;

class DictionaryComplier {
 public:
  static int run(int argc, char **argv) {
    static const MeCab::Option long_options[] = {
      { "dicdir",   'd',   ".",   "DIR", "set DIR as dic dir (default \".\")" },
      { "outdir",   'o',   ".",   "DIR",
        "set DIR as output dir (default \".\")" },
      { "model",   'm',  0,     "FILE", "use FILE as model file" },
      { "userdic",  'u',   0,   "FILE",   "build user dictionary" },
      { "assign-user-dictionary-costs", 'a', 0, 0,
        "only assign costs/ids to user dictionary" },
      { "build-unknown",  'U',   0,   0,
        "build parameters for unknown words" },
      { "build-model", 'M', 0, 0,   "build model file" },
      { "build-charcategory", 'C', 0, 0,   "build character category maps" },
      { "build-sysdic",  's', 0, 0,   "build system dictionary" },
      { "build-matrix",    'm',  0,   0,   "build connection matrix" },
      { "charset",   'c',  MECAB_DEFAULT_CHARSET, "ENC",
        "make charset of binary dictionary ENC (default "
        MECAB_DEFAULT_CHARSET ")"  },
      { "charset",   't',  MECAB_DEFAULT_CHARSET, "ENC", "alias of -c"  },
      { "dictionary-charset",  'f',  MECAB_DEFAULT_CHARSET,
        "ENC", "assume charset of input CSVs as ENC (default "
        MECAB_DEFAULT_CHARSET ")"  },
      { "wakati",    'w',  0,   0,   "build wakati-gaki only dictionary", },
      { "posid",     'p',  0,   0,   "assign Part-of-speech id" },
      { "node-format", 'F', 0,  "STR",
        "use STR as the user defined node format" },
      { "quiet",     'q',  0,   0,   "don't print progress"  },
      { "version",   'v',  0,   0,   "show the version and exit."  },
      { "help",      'h',  0,   0,   "show this help and exit."  },
      { 0, 0, 0, 0 }
    };

    Param param;

    if (!param.open(argc, argv, long_options)) {
      std::cout << param.what() << "\n\n" <<  COPYRIGHT
                << "\ntry '--help' for more information." << std::endl;
      return -1;
    }

    if (!param.help_version()) {
      return 0;
    }

    const std::string dicdir = param.get<std::string>("dicdir");
    const std::string outdir = param.get<std::string>("outdir");
    bool opt_unknown = param.get<bool>("build-unknown");
    bool opt_matrix = param.get<bool>("build-matrix");
    bool opt_charcategory = param.get<bool>("build-charcategory");
    bool opt_sysdic = param.get<bool>("build-sysdic");
    bool opt_model = param.get<bool>("build-model");
    bool opt_assign_user_dictionary_costs = param.get<bool>
        ("assign-user-dictionary-costs");
    bool opt_quiet = param.get<bool>("quiet");
    const std::string userdic = param.get<std::string>("userdic");

    MeCab::quiet_mode = opt_quiet;

#define DCONF(file) create_filename(dicdir, std::string(file)).c_str()
#define OCONF(file) create_filename(outdir, std::string(file)).c_str()

    if (param.load(DCONF(DICRC)) == false) {
      std::cerr << "no such file or directory: " << DCONF(DICRC) << std::endl;
      return -1;
    }

    std::vector<std::string> dic;
    if (userdic.empty()) {
      enum_csv_dictionaries(dicdir.c_str(), &dic);
    } else {
      dic = param.rest_args();
    }

    // NOTE: Open JTalk ビルドでは die クラスのデストラクタ内の exit() がコメントアウトされている
    // (common.h 参照) ため、CHECK_DIE は失敗時に stderr へ出力するだけでプロセスを終了しない。
    // ユーザー辞書パス (!userdic.empty()) で通るコード内の CHECK_DIE のうち、不正な CSV 入力で
    // 未定義動作やクラッシュを引き起こしうるものは dictionary.cpp / dictionary_rewriter.cpp /
    // context_id.cpp にて適切なエラーリターンに変換済み。不正なエントリは lid/rid >= 0 チェック等で
    // 安全にスキップされる。
    // システム辞書パス (else ブランチ) の Connector::compile / CharProperty::compile 等には
    // CHECK_DIE が残っているが、同梱のシステム辞書ソースファイルが不正でない限り発動しない。
    if (!userdic.empty()) {
      if (dic.size() == 0) {
        std::cerr << "no dictionaries are specified" << std::endl;
        return -1;
      }
      param.set("type", static_cast<int>(MECAB_USR_DIC));
      if (opt_assign_user_dictionary_costs) {
        const bool is_compiled = Dictionary::assignUserDictionaryCosts(
            param,
            dic,
            userdic.c_str());
        if (is_compiled == false) {
          std::cerr << "failed to compile user dictionary: " << userdic << std::endl;
          return -1;
        }
      } else {
        const bool is_compiled = Dictionary::compile(param, dic, userdic.c_str());
        if (is_compiled == false) {
          std::cerr << "failed to compile user dictionary: " << userdic << std::endl;
          return -1;
        }
      }
    } else {
      if (!opt_unknown && !opt_matrix && !opt_charcategory &&
          !opt_sysdic && !opt_model) {
        opt_unknown = opt_matrix = opt_charcategory =
            opt_sysdic = opt_model = true;
      }

      if (opt_charcategory || opt_unknown) {
        const bool is_compiled = CharProperty::compile(DCONF(CHAR_PROPERTY_DEF_FILE),
                                                       DCONF(UNK_DEF_FILE),
                                                       OCONF(CHAR_PROPERTY_FILE));
        if (is_compiled == false) {
          std::cerr << "failed to compile character category" << std::endl;
          return -1;
        }
      }

      if (opt_unknown) {
        std::vector<std::string> tmp;
        tmp.push_back(DCONF(UNK_DEF_FILE));
        param.set("type", static_cast<int>(MECAB_UNK_DIC));
        const bool is_compiled = Dictionary::compile(param, tmp, OCONF(UNK_DIC_FILE));
        if (is_compiled == false) {
          std::cerr << "failed to compile unknown dictionary" << std::endl;
          return -1;
        }
      }

      if (opt_model) {
        if (file_exists(DCONF(MODEL_DEF_FILE))) {
          const bool is_compiled = FeatureIndex::compile(param,
                                                         DCONF(MODEL_DEF_FILE),
                                                         OCONF(MODEL_FILE));
          if (is_compiled == false) {
            std::cerr << "failed to compile model" << std::endl;
            return -1;
          }
        } else {
          if (!opt_quiet) {
            std::cout << DCONF(MODEL_DEF_FILE)
                      << " is not found. skipped." << std::endl;
          }
        }
      }

      if (opt_sysdic) {
        if (dic.size() == 0) {
          std::cerr << "no dictionaries are specified" << std::endl;
          return -1;
        }
        param.set("type", static_cast<int>(MECAB_SYS_DIC));
        const bool is_compiled = Dictionary::compile(param, dic, OCONF(SYS_DIC_FILE));
        if (is_compiled == false) {
          std::cerr << "failed to compile system dictionary" << std::endl;
          return -1;
        }
      }

      if (opt_matrix) {
        const bool is_compiled = Connector::compile(DCONF(MATRIX_DEF_FILE),
                                                    OCONF(MATRIX_FILE));
        if (is_compiled == false) {
          std::cerr << "failed to compile connection matrix" << std::endl;
          return -1;
        }
      }
    }

    if (!opt_quiet) {
      std::cout << "\ndone!\n";
    }

    return 0;
  }
};

#undef DCONF
#undef OCONF
}

int mecab_dict_index(int argc, char **argv) {
  return MeCab::DictionaryComplier::run(argc, argv);
}
