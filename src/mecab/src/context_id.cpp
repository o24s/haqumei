// MeCab -- Yet Another Part-of-Speech and Morphological Analyzer
//
//
//  Copyright(C) 2001-2006 Taku Kudo <taku@chasen.org>
//  Copyright(C) 2004-2006 Nippon Telegraph and Telephone Corporation
#include <fstream>
#include "context_id.h"
#include "iconv_utils.h"
#include "utils.h"

namespace {

using namespace MeCab;

bool open_map(const char *filename,
              std::map<std::string, int> *cmap,
              Iconv *iconv) {
  std::ifstream ifs(WPATH(filename));
  if (!ifs) {
    std::cerr << "no such file or directory: " << filename << std::endl;
    return false;
  }
  cmap->clear();
  char *col[2];
  std::string line;
  while (std::getline(ifs, line)) {
    if (2 != tokenize2(const_cast<char *>(line.c_str()),
                       " \t", col, 2)) {
      std::cerr << "format error: " << line << std::endl;
      return false;
    }
    std::string pos = col[1];
    if (iconv) {
      iconv->convert(&pos);
    }
    cmap->insert(std::pair<std::string, int>
                 (pos, std::atoi(col[0])));
  }
  return true;
}

bool build(std::map<std::string, int> *cmap,
           const std::string &bos) {
  int id = 1;  // for BOS/EOS
  for (std::map<std::string, int>::iterator it = cmap->begin();
       it != cmap->end();
       ++it) it->second = id++;
  cmap->insert(std::make_pair(bos, 0));
  return true;
}

bool save(const char* filename,
          std::map<std::string, int> *cmap) {
  std::ofstream ofs(WPATH(filename));
  if (!ofs) {
    std::cerr << "permission denied: " << filename << std::endl;
    return false;
  }
  for (std::map<std::string, int>::const_iterator it = cmap->begin();
       it != cmap->end(); ++it) {
    ofs << it->second << " " << it->first << std::endl;
  }
  return true;
}
}

namespace MeCab {

void ContextID::clear() {
  left_.clear();
  right_.clear();
  left_bos_.clear();
  right_bos_.clear();
}

void ContextID::add(const char *l, const char *r) {
  left_.insert(std::make_pair(std::string(l), 1));
  right_.insert(std::make_pair(std::string(r), 1));
}

void ContextID::addBOS(const char *l, const char *r) {
  left_bos_ = l;
  right_bos_ = r;
}

bool ContextID::save(const char* lfile,
                     const char* rfile) {
  return (::save(lfile, &left_) && ::save(rfile, &right_));
}

bool ContextID::open(const char *lfile,
                     const char *rfile,
                     Iconv *iconv) {
  return (::open_map(lfile, &left_, iconv) &&
          ::open_map(rfile, &right_, iconv));
}

bool ContextID::build() {
  return (::build(&left_, left_bos_) && ::build(&right_, right_bos_));
}

int ContextID::lid(const char *l) const {
  std::map<std::string, int>::const_iterator it = left_.find(l);
  // NOTE: end() イテレータをデリファレンスする（未定義動作）代わりに -1 を返す
  // 呼び出し元は lid >= 0 を確認してから結果を使用する
  if (it == left_.end()) {
    std::cerr << "cannot find LEFT-ID for " << l << std::endl;
    return -1;
  }
  return it->second;
}

int ContextID::rid(const char *r) const {
  std::map<std::string, int>::const_iterator it = right_.find(r);
  // NOTE: end() イテレータをデリファレンスする（未定義動作）代わりに -1 を返す
  // 呼び出し元は rid >= 0 を確認してから結果を使用する
  if (it == right_.end()) {
    std::cerr << "cannot find RIGHT-ID for " << r << std::endl;
    return -1;
  }
  return it->second;
}
}
