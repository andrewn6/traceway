# Fix Proxy preview_string Panic on Multi-Byte UTF-8

**Labels:** `bug`, `backend`
**Difficulty:** Easy
**Priority:** P0 — Critical

## Summary

The `preview_string` function in `crates/proxy/src/lib.rs` truncates strings by byte index, not character boundary. If the truncation point falls in the middle of a multi-byte UTF-8 character (e.g., emoji, CJK, accented characters), Rust panics with "byte index is not a char boundary."

## Impact

Any LLM response containing multi-byte UTF-8 characters near the truncation boundary will crash the proxy, causing a 500 error for the user. This is especially common with non-English content or emoji-heavy outputs.

## Where

`crates/proxy/src/lib.rs` — `preview_string` function. It likely does something like:

```rust
fn preview_string(s: &str, max_len: usize) -> String {
    s[..max_len].to_string()  // PANICS if max_len is mid-character
}
```

## What to do

Replace byte-based truncation with character-aware truncation:

```rust
fn preview_string(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
```

Or if you want byte-level control (for bandwidth reasons), use `floor_char_boundary` (Rust 1.73+):

```rust
fn preview_string(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let truncated = s.floor_char_boundary(max_bytes);
    s[..truncated].to_string()
}
```

## Acceptance criteria

- [ ] `preview_string("Hello 🌍 World", 8)` does not panic
- [ ] `preview_string("日本語テスト", 5)` does not panic
- [ ] Output is valid UTF-8 in all cases
- [ ] `cargo check -p proxy` passes
- [ ] Add a unit test with multi-byte strings
