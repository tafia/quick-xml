Run fuzzing with `-O` to avoid false positives at `debug_assert!`, e.g.:

```bash
cargo fuzz run -O -j4 fuzz_target_1
```

See also: https://github.com/rust-fuzz/cargo-fuzz
