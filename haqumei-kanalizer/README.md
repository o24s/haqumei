# haqumei-kanalizer

An ONNX-based English-to-Katakana translator for the Haqumei Japanese G2P library.

## Usage

```rust
let mut kanalizer = haqumei_kanalizer::Kanalizer::new().unwrap();
let kana = kanalizer.convert("kanalizer").unwrap();

assert_eq!(kana, "カナライザー");
```

## License

This crate (including both the Rust code and the bundled ONNX models) is distributed under the terms of the **MIT License**.

- The models are derived from the [VOICEVOX/kanalizer](https://github.com/VOICEVOX/kanalizer) project (model weights from [VOICEVOX/kanalizer-model](https://huggingface.co/VOICEVOX/kanalizer-model)), which is also licensed under the MIT License.
- The ONNX conversion was performed using [o24s/kanalizer-onnx](https://github.com/o24s/kanalizer-onnx).
