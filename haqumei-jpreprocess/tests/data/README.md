These small dictionary sources are derived from `tests/data` in [jpreprocess](https://github.com/jpreprocess/jpreprocess/tree/54cf9bc2d40a5d6f25144333e9cd03fd3258a126/tests/data), using NAIST-JDIC 0.1.3. Context IDs are compacted consistently across the system dictionary, user dictionary, and matrix. The integration test compiles the sources with Haqumei’s shared compiler and exercises system and user dictionary loading through vibrato-rkyv.

The source code and test fixtures retain the upstream BSD-3-Clause license in `LICENSE`. The dictionary notices are in `LICENSE-naist-jdic`.
