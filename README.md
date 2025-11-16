# Haqumei

Haqumei is a Japanese Grapheme-to-Phoneme (G2P) library.

## License

The Rust code of `haqumei` is distributed under the terms of the Apache License 2.0. See the `LICENSE` file in the repository root for details.

### Licenses and Origins of Bundled Software

`haqumei` includes C/C++ source code from a modified version of Open JTalk to provide its Grapheme-to-Phoneme (G2P) functionality. The origins and licenses of this bundled code are as follows:

- Bundled Open JTalk Source Code
  - Origin: The code contained in the `vendor/open_jtalk` directory is based on the
    [tsukumijima/open_jtalk](https://github.com/tsukumijima/open_jtalk) repository, which integrates
    improvements from various community forks (including those from the VOICEVOX project) into an enhanced
    version of Open JTalk.
  - License: The bundled Open JTalk source code is licensed under the Modified BSD License. This license applies
    only to the code located in `vendor/open_jtalk`, and does not apply to the rest of this project. In accordance
    with redistribution requirements, the full text of the Modified BSD License is included in
    `vendor/open_jtalk/src/COPYING`.

## Acknowledgements

The overall design and API of `haqumei` are inspired by `pyopenjtalk` and its highly improved fork, `pyopenjtalk-plus`.

- pyopenjtalk: Copyright (c) 2018 Ryuichi Yamamoto (MIT License)
- pyopenjtalk-plus: Copyright (c) 2023 tsukumijima (MIT License)

We are deeply grateful to the authors and contributors of these foundational projects.
