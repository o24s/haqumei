#include <stdio.h>
#include <stdarg.h>
#include <string.h>

extern void rust_log_redirect(const char *msg, int is_stderr);

int custom_fprintf(FILE *stream, const char *format, ...) {
    char buffer[4096];
    va_list args;

    va_start(args, format);
    vsnprintf(buffer, sizeof(buffer), format, args);
    va_end(args);

    rust_log_redirect(buffer, stream == stderr);

    return 0;
}