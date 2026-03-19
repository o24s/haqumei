#include <iostream>
#include <streambuf>
#include <string>

extern "C" void haqumei_rust_print(const char* msg, int is_stderr);

class RustLogBuf : public std::streambuf {
private:
    std::string buffer;
    int is_stderr;

    void flush_to_rust() {
        if (!buffer.empty()) {
            haqumei_rust_print(buffer.c_str(), is_stderr);
            buffer.clear();
        }
    }

protected:
    virtual int_type overflow(int_type c) override {
        if (c != traits_type::eof()) {
            char ch = traits_type::to_char_type(c);
            if (ch == '\n') {
                flush_to_rust();
            } else {
                buffer += ch;
            }
        }
        return c;
    }

    virtual std::streamsize xsputn(const char* s, std::streamsize n) override {
        for (std::streamsize i = 0; i < n; ++i) {
            if (s[i] == '\n') {
                flush_to_rust();
            } else {
                buffer += s[i];
            }
        }
        return n;
    }

    virtual int sync() override {
        return 0;
    }

public:
    RustLogBuf(int is_stderr) : is_stderr(is_stderr) {}
    virtual ~RustLogBuf() {
        flush_to_rust();
    }
};

static std::streambuf* orig_cout_buf = nullptr;
static std::streambuf* orig_cerr_buf = nullptr;
static RustLogBuf* rust_cout_buf = nullptr;
static RustLogBuf* rust_cerr_buf = nullptr;

extern "C" void setup_cpp_redirect() {
    if (!rust_cout_buf) {
        rust_cout_buf = new RustLogBuf(0);
        orig_cout_buf = std::cout.rdbuf(rust_cout_buf);
    }
    if (!rust_cerr_buf) {
        rust_cerr_buf = new RustLogBuf(1);
        orig_cerr_buf = std::cerr.rdbuf(rust_cerr_buf);
    }
}

extern "C" void teardown_cpp_redirect() {
    if (rust_cout_buf) {
        std::cout.rdbuf(orig_cout_buf);
        delete rust_cout_buf;
        rust_cout_buf = nullptr;
    }
    if (rust_cerr_buf) {
        std::cerr.rdbuf(orig_cerr_buf);
        delete rust_cerr_buf;
        rust_cerr_buf = nullptr;
    }
}
