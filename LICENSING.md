# License

Morflo is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.

This is the customary dual license of the Rust ecosystem, and it matches the
terms of every crate Morflo compiles. You may take Morflo under whichever of the
two suits you; you do not need to satisfy both.

## Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Morflo by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## What this covers, and what it does not

This license covers Morflo's own source, including its built-in image engine.
Every third-party component compiled into Morflo is permissively licensed and
recorded in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

It does **not** cover FFmpeg. Morflo runs a locally installed `ffmpeg` and
`ffprobe` as external programs when it finds them, and ships no media-engine
binary of any kind. An FFmpeg build carries its own terms — commonly GPL or
LGPL depending on how it was configured — and those terms bind whoever
distributes that build, not Morflo's source. If you package Morflo together with
an FFmpeg binary, that combination is yours to license correctly. See
[docs/legal/media-engine-distribution.md](docs/legal/media-engine-distribution.md).
