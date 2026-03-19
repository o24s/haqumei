#ifndef FPRINTF_REDIRECT_H
#define FPRINTF_REDIRECT_H

#ifdef __cplusplus
  #include <cstdio>
  #include <stdio.h>
#else
  #include <stdio.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif
    int haqumei_redirect_printf(const char *format, ...);
    int haqumei_redirect_fprintf(FILE *stream, const char *format, ...);
#ifdef __cplusplus
}
#endif

#define printf(...) haqumei_redirect_printf(__VA_ARGS__)
#define fprintf(...) haqumei_redirect_fprintf(__VA_ARGS__)

#endif // FPRINTF_REDIRECT_H
