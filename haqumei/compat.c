#include <stdlib.h>

#if defined(__GNUC__) || defined(__clang__)

__attribute__((weak))
long __isoc23_strtol(const char *nptr, char **endptr, int base) {
    return strtol(nptr, endptr, base);
}

__attribute__((weak))
long long __isoc23_strtoll(const char *nptr, char **endptr, int base) {
    return strtoll(nptr, endptr, base);
}

__attribute__((weak))
unsigned long long __isoc23_strtoull(
    const char *nptr,
    char **endptr,
    int base
) {
    return strtoull(nptr, endptr, base);
}

#endif