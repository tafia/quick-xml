# quick-xml examples

There are many ways to use quick-xml, this guide is intended to help you choose
*which* of its APIs to use. There are two, from highest-level to lowest:

- **serde** (the `serialize` feature) — map XML directly onto
  `#[derive(Serialize, Deserialize)]` structs. Best when your XML corresponds
  cleanly to Rust types.
- **the pull `Reader` and push `Writer`** — work with a stream of `Event`s
  (`Start`, `Text`, `End`, ...). Best when you need streaming, partial parsing,
  or to transform a document. The `Writer` additionally offers a high-level
  builder and a low-level event API; see [Writing](#writing) below.

For simple jobs, it's best to start with serde and drop to the `Reader`/`Writer`
only when it doesn't fit. For parsing especially large or complex documents however,
using the lower-level APIs is likely to be a better approach.

## Which approach should I use?

### Reading

The rule of thumb, in order of preference:

1. **serde** — if the document is reasonably sized, performance is not critical,
   and the shape maps onto structs, this is the least code by far. You describe
   the types once and get parsing for free. See [`serde_roundtrip.rs`](serde_roundtrip.rs)

2. **A state machine** — when the items you want are simple and not too deeply
   nested, a hand-written `Reader` loop that tracks its position in an explicit
   `enum` is fast, allocation-light, and makes the grammar you accept obvious.
   Matching on `(state, event)` pairs shows exactly what you expect where, and a
   single read buffer serves the whole document. See [`reader_patterns.rs`](reader_patterns.rs)

3. **Nested readers / split functions** — when the document has many levels,
   handing each subtree to its own function reads more naturally than one giant
   state enum. The trade-off is that it is harder to reuse a single read buffer
   across levels, so deeply nested parsing can allocate more.
   See [`reader_patterns.rs`](reader_patterns.rs), [`nested_readers.rs`](nested_readers.rs)

In practice case 1 is far more common than 2, which is far more common than 3.
Reach for the lower-level options mainly when serde can't express the mapping,
when you are streaming data too large to hold in memory, or when parsing is hot
enough that avoiding the intermediate structs matters.

If more than one part of your program needs the same document, or if you want the
parsing logic to live apart from what consumes it — layer a **visitor** over any
of the above: one driver walks the document and calls back into a trait, and
each consumer implements only the callbacks it needs. This is a powerful design
pattern that can greatly simplify code re-use in the future. See [`visitor.rs`](visitor.rs)

### Writing

1. **serde** — if you already have the data in Rust structs, serializing them
   directly is the least effort. See [`serde_roundtrip.rs`](serde_roundtrip.rs)

2. **The high-level `Writer::create_element` builder** — an extension of the
   standard `Writer` API which handles writing the opening and closing element
   tags and makes adding attributes more natural. A closure is provided for
   writing the inner content of the element. See [`writer.rs`](writer.rs)

3. **Low-level `Writer::write_event`** — emit each `Start`/`Text`/`End` event
   yourself. Most verbose and easiest to unbalance, but gives total control and
   is ideal when *transforming* a document (read an event, tweak it, write it
   back out). Sees [`writer.rs`](writer.rs), and the transform example in the
   crate's [README](../README.md)

## Buffered vs. borrowed reading

- `Reader::from_str` / `Reader::from_reader` over a `&[u8]` can return events
  that **borrow** from the input, so you call `read_event()` with no buffer.
- Streaming sources (files, sockets) use `read_event_into(&mut buf)` and write
  into a `Vec<u8>` you supply. Reusing (and `clear()`-ing) that one buffer
  across the loop keeps allocations low. See [`read_buffered.rs`](read_buffered.rs)

## The examples

### Start here
- [`getting_started.rs`](getting_started.rs) — the canonical pull-reader loop:
  create a reader, match events, pull out attributes and text. **Read this first.**

### Reading
- [`reader_patterns.rs`](reader_patterns.rs) — state machine vs. nested readers,
  side by side, parsing the same document into the same result.
- [`visitor.rs`](visitor.rs) — the visitor pattern: one parser driver, a trait of
  callbacks, and several consumers that each override only what they need.
- [`read_buffered.rs`](read_buffered.rs) — streaming with a reusable buffer.
- [`nested_readers.rs`](nested_readers.rs) — walking several levels to extract
  data from a real-world (ECMA-376) document.
- [`read_nodes.rs`](read_nodes.rs) — dispatching on top-level nodes by hand.
- [`custom_entities.rs`](custom_entities.rs) — resolving custom `&entity;` definitions.
- [`read_utf16.rs`](read_utf16.rs) — non-UTF-8 input via `DecodingReader`
  (needs `--features encoding`).

### Writing
- [`writer.rs`](writer.rs) — the high-level builder and the low-level event API,
  producing identical output.

### serde
- [`serde_roundtrip.rs`](serde_roundtrip.rs) — mapping XML to structs and back:
  attributes (`@`), text (`$text`), nesting, sequences, and type conversion.
  **Read this to learn the serde field-naming rules** (needs `--features serialize`).
- [`read_nodes_serde.rs`](read_nodes_serde.rs) — the serde counterpart to
  `read_nodes.rs` (needs `--features serialize`).
- [`flattened_enum.rs`](flattened_enum.rs) — choosing an enum variant from an
  attribute with a custom (de)serializer (needs `--features serialize`).

## Running an example

```console
cargo run --example getting_started
cargo run --example serde_roundtrip --features serialize
cargo run --example read_utf16      --features encoding
```
