These small dictionary fixtures are copied from `tests/data` in [jpreprocess](https://github.com/jpreprocess/jpreprocess/tree/54cf9bc2d40a5d6f25144333e9cd03fd3258a126/tests/data). They exercise Lindera system and user dictionary loading in the API examples.

The binaries have been rebuilt for Lindera 6. `min-dict-src` contains the corresponding CSV, character definitions, unknown-word entries, and connection matrix, derived from the same upstream NAIST-JDIC 0.1.3 data. Context IDs are compacted consistently across the system dictionary, user dictionary, and matrix. The integration test builds an rkyv dictionary from these sources and compares its NJD and labels against the plain Lindera fixtures.

The source code and test fixtures retain the upstream BSD-3-Clause license in `LICENSE`. The dictionary notices are in `LICENSE-naist-jdic`.
