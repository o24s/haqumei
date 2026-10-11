/* glibc < 2.38 compat shims for __isoc23_* and __libc_single_threaded */

#ifndef _GNU_SOURCE
#define _GNU_SOURCE 1
#endif

#include <ctype.h>
#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <locale.h>
#include <stdint.h>
#include <stdlib.h>

#if defined(HAQUMEI_FORCE_GLIBC_COMPAT) || \
    (defined(__GLIBC__) && __GLIBC__ == 2 && __GLIBC_MINOR__ < 38)

#if !defined(__GNUC__)
#error "GCC or Clang is required (weak/hidden attributes, asm labels)"
#endif

/* Drop header redirect macros if present */
#undef strtol
#undef strtoul
#undef strtoll
#undef strtoull
#undef strtoimax
#undef strtoumax

#ifndef HAQUMEI_C23_PREFIX
#define HAQUMEI_C23_PREFIX __isoc23_
#define HAQUMEI_C23_PRODUCTION 1
#endif
#define C23_CAT_(a, b) a##b
#define C23_CAT(a, b) C23_CAT_(a, b)
#define C23_SYM(name) C23_CAT(HAQUMEI_C23_PREFIX, name)

#define COMPAT __attribute__((weak, visibility("hidden")))

/* Always report multi-threaded (0 is the safe fallback for glibc < 2.32) */
#ifdef HAQUMEI_C23_PRODUCTION
COMPAT extern char __libc_single_threaded;
COMPAT char __libc_single_threaded = 0;
#endif

typedef char c23_uintmax_fits[(sizeof(uintmax_t) <= sizeof(unsigned long long)) ? 1 : -1];

struct c23_bin {
  unsigned long long mag;
  int neg;
  int ovf;
};

static int c23_isspace(unsigned char c, locale_t loc) {
  return loc ? isspace_l(c, loc) : isspace(c);
}

/* Parse "0b" / "0B" prefix; returns 0 if input should fallback to standard libc functions */
static int c23_binary(const char *nptr, char **endptr, int base, locale_t loc,
                      struct c23_bin *r) {
  const char *s = nptr;
  unsigned long long v = 0;
  int neg = 0;
  int ovf = 0;

  if (base != 0 && base != 2)
    return 0;
  while (c23_isspace((unsigned char)*s, loc))
    ++s;
  if (*s == '-') {
    neg = 1;
    ++s;
  } else if (*s == '+') {
    ++s;
  }
  if (s[0] != '0' || (s[1] != 'b' && s[1] != 'B') ||
      (s[2] != '0' && s[2] != '1'))
    return 0;
  s += 2;
  for (; *s == '0' || *s == '1'; ++s) {
    if (v > (ULLONG_MAX >> 1))
      ovf = 1;
    else
      v = (v << 1) | (unsigned long long)(*s - '0');
  }
  r->mag = v;
  r->neg = neg;
  r->ovf = ovf;
  if (endptr)
    *endptr = (char *)s;
  return 1;
}

static long long c23_signed(const struct c23_bin *r, long long min,
                            long long max) {
  unsigned long long limit =
      r->neg ? 0ULL - (unsigned long long)min : (unsigned long long)max;
  if (r->ovf || r->mag > limit) {
    errno = ERANGE;
    return r->neg ? min : max;
  }
  return r->neg ? (long long)(0ULL - r->mag) : (long long)r->mag;
}

static unsigned long long c23_unsigned(const struct c23_bin *r,
                                       unsigned long long max) {
  if (r->ovf || r->mag > max) {
    errno = ERANGE;
    return max;
  }
  return r->neg ? 0ULL - r->mag : r->mag;
}

/* asm labels call the classic libc symbol directly to avoid infinite recursion */
#define C23_DEF_S(NAME, RET, MIN, MAX)                                         \
  extern RET real_##NAME(const char *, char **, int) __asm__(#NAME);           \
  COMPAT RET C23_SYM(NAME)(const char *, char **, int);                        \
  COMPAT RET C23_SYM(NAME)(const char *nptr, char **endptr, int base) {        \
    struct c23_bin r;                                                          \
    if (!c23_binary(nptr, endptr, base, NULL, &r))                             \
      return real_##NAME(nptr, endptr, base);                                  \
    return (RET)c23_signed(&r, MIN, MAX);                                      \
  }

#define C23_DEF_U(NAME, RET, MAX)                                              \
  extern RET real_##NAME(const char *, char **, int) __asm__(#NAME);           \
  COMPAT RET C23_SYM(NAME)(const char *, char **, int);                        \
  COMPAT RET C23_SYM(NAME)(const char *nptr, char **endptr, int base) {        \
    struct c23_bin r;                                                          \
    if (!c23_binary(nptr, endptr, base, NULL, &r))                             \
      return real_##NAME(nptr, endptr, base);                                  \
    return (RET)c23_unsigned(&r, MAX);                                         \
  }

#define C23_DEF_SL(NAME, RET, MIN, MAX)                                        \
  extern RET real_##NAME(const char *, char **, int, locale_t)                 \
      __asm__(#NAME);                                                          \
  COMPAT RET C23_SYM(NAME)(const char *, char **, int, locale_t);              \
  COMPAT RET C23_SYM(NAME)(const char *nptr, char **endptr, int base,          \
                           locale_t loc) {                                     \
    struct c23_bin r;                                                          \
    if (!c23_binary(nptr, endptr, base, loc, &r))                              \
      return real_##NAME(nptr, endptr, base, loc);                             \
    return (RET)c23_signed(&r, MIN, MAX);                                      \
  }

#define C23_DEF_UL(NAME, RET, MAX)                                             \
  extern RET real_##NAME(const char *, char **, int, locale_t)                 \
      __asm__(#NAME);                                                          \
  COMPAT RET C23_SYM(NAME)(const char *, char **, int, locale_t);              \
  COMPAT RET C23_SYM(NAME)(const char *nptr, char **endptr, int base,          \
                           locale_t loc) {                                     \
    struct c23_bin r;                                                          \
    if (!c23_binary(nptr, endptr, base, loc, &r))                              \
      return real_##NAME(nptr, endptr, base, loc);                             \
    return (RET)c23_unsigned(&r, MAX);                                         \
  }

C23_DEF_S(strtol, long, LONG_MIN, LONG_MAX)
C23_DEF_S(strtoll, long long, LLONG_MIN, LLONG_MAX)
C23_DEF_S(strtoimax, intmax_t, INTMAX_MIN, INTMAX_MAX)
C23_DEF_U(strtoul, unsigned long, ULONG_MAX)
C23_DEF_U(strtoull, unsigned long long, ULLONG_MAX)
C23_DEF_U(strtoumax, uintmax_t, UINTMAX_MAX)
C23_DEF_SL(strtol_l, long, LONG_MIN, LONG_MAX)
C23_DEF_SL(strtoll_l, long long, LLONG_MIN, LLONG_MAX)
C23_DEF_UL(strtoul_l, unsigned long, ULONG_MAX)
C23_DEF_UL(strtoull_l, unsigned long long, ULLONG_MAX)

#endif
