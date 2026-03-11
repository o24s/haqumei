#include <stdio.h>
#include <stdarg.h>

#ifdef _WIN32
  #include <io.h>
  #define GET_FILENO _fileno
#else
  #define GET_FILENO fileno
#endif

extern void haqumei_rust_print(const char *msg, int is_stderr);

static int redirect_to_rust(const char* format, va_list args, int is_stderr) {
    char buffer[1024];
    int ret = vsnprintf(buffer, sizeof(buffer), format, args);

    if (ret < 0) {
        return ret;
    }

    if (ret >= (int)sizeof(buffer)) {
        buffer[sizeof(buffer) - 2] = '.'; // Last printable char
        buffer[sizeof(buffer) - 3] = '.';
        buffer[sizeof(buffer) - 4] = '.';
    }

    haqumei_rust_print(buffer, is_stderr);

    return ret;
}

int haqumei_redirect_fprintf(FILE *stream, const char *format, ...) {
    int fd = GET_FILENO(stream);
    int is_stderr = (fd == 2);
    int is_stdout = (fd == 1);

    if (!is_stderr && !is_stdout) {
        va_list args;
        va_start(args, format);
        int ret = vfprintf(stream, format, args);
        va_end(args);
        return ret;
    }

    va_list args;
    va_start(args, format);
    int ret = redirect_to_rust(format, args, is_stderr);
    va_end(args);
    return ret;
}
