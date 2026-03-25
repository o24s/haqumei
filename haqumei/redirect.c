#include <stdio.h>
#include <stdarg.h>

extern void haqumei_rust_print(const char* msg, int is_stderr);

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

#ifdef __GNUC__
    __attribute__((format(printf, 1, 2)))
#endif
int haqumei_redirect_printf(const char *format, ...) {
    va_list args;
    va_start(args, format);
    int ret = redirect_to_rust(format, args, 0);
    va_end(args);

    return ret;
}

#ifdef __GNUC__
__attribute__((format(printf, 2, 3)))
#endif
int haqumei_redirect_fprintf(FILE *stream, const char *format, ...) {
    int is_stdout = (stream == stdout);
    int is_stderr = (stream == stderr);

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
