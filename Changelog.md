> Legend:
  - feat: A new feature
  - fix: A bug fix
  - docs: Documentation only changes
  - style: White-space, formatting, missing semi-colons, etc
  - refactor: A code change that neither fixes a bug nor adds a feature
  - perf: A code change that improves performance
  - test: Adding missing tests
  - chore: Changes to the build process or auxiliary tools/libraries/documentation

## Unreleased

- test: add tests for malformed inputs for serde deserializer
- fix: allow to deserialize `unit`s from any data in attribute values and text nodes
- refactor: unify errors when EOF encountered during serde deserialization
- test: ensure that after deserializing all XML was consumed
- feat: add `Deserializer::from_str` and `Deserializer::from_bytes`
- refactor: reduce number of unnecessary copies when deserialize numbers/booleans/identifiers
  from the attribute and element names and attribute values
- fix: allow to deserialize `unit`s from text and CDATA content.
  `DeError::InvalidUnit` variant is removed, because after fix it is no longer used

## 0.23.0-alpha3

- fix: use element name (with namespace) when unflattening (serialize feature)

## 0.23.0-alpha2

- fix: failing tests with features

## 0.23.0-alpha1

- style: convert to rust edition 2018
- fix: don't encode multi byte escape characters as big endian
- feat: add `Writer::write_nested_event`
- feat: add `BytesStart::try_get_attribute`
- test: add more test on github actions
- feat: allow unbuffered deserialization (!!)
- style: use edition 2018
- feat: add a function for partially escaping an element
- feat: higher level api to write xmls

## 0.22.0

- feat (breaking): Move html entity escape behind a `'escape-html'` feature to help with compilation
- style: rustfmt
- feat: inline CData when pretty printing
- test: fix tests (Windows and Html5)
- feat (breaking): add `*_with_custom_entities` versions of all `unescape_*\ methods
- test: more robust test for numeric entities
- refactor: add explicit pre-condition about custom_entities

## 0.21.0

- feat: Split text trim into start and end
- fix: `$value` rename should work the same for deserialization and serialization
- docs: README.md: Replace dead benchmark link
- style: Cargo.toml: remove "readme" field
- fix: Parse & in cdata correctly
- style: Fix reader.rs typo
- feat: Accept html5 doctype
- fix: Unescape all existing HTML entities

## 0.20.0
- test: Add tests for indentation
- test: Add complete tests for serde deserialization
- feat: Use self-closed tags when serialize types without nested elements with serde
- feat: Add two new API to the `BytesStart`: `to_borrowed()` and `to_end()`
- feat: Add ability to specify name of the root tag and indentation settings when
  serialize type with serde
- feat: Add support for serialization of
  - unit enums variants
  - newtype structs and enum variants
  - unnamed tuples, tuple structs and enum variants
- fix: More consistent structs serialization
- fix: Deserialization of newtype structs
- fix: `unit` deserialization and newtype and struct deserialization in adjacently tagged enums

## 0.19.0
- docs: Add example for nested parsing
- fix: `buffer_position` not properly set sometimes
- feat: Make escape module public apart from EscapeError
- feat: Nake Reader `Clone`able
- feat: Enable writing manual indentation (and fix underflow on shrink)
- style: Forbid unsafe code
- fix: Use `write_all` instead of `write`
- fix: (Serde) Serialize basic types as attributes (breaking change)
- test: Fix benchmarks on Windows and add trimmed variant
- feat: deserialize bytes

## 0.18.0 - 0.18.1
- feat: add `decode_without_bom` fns for BOM prefixed text fields
- fix: decode then unescape instead of unescape and decode

## 0.17.2
- feat: add Seq to serializer
- docs: update readme with example for `$value`

## 0.17.1
- feat: add new `serialize` feature to support serde serialize/deserialize

## 0.17.0
- perf: speed up (un)escape a little
- feat: remove failure completely (breaking change) and implement `std::error::Error` for `Error`
- feat: improve `Debug`s for `Attribute`, `BytesStart`, `BytesEnd`, `BytesText`

## 0.16.1
- refactor: remove derive_more dependency (used only in 2 structs)
- refactor: move xml-rs bench dependency into another local crate

## 0.16.0
- feat: (breaking change) set failure and encoding_rs crates as optional.
You should now use respectively `use-failure` and `encoding` features to get the old behavior
- perf: improve perf using memchr3 iterator. Reading is 18% better on benches

## 0.15.0
- feat: remove Seek bound
- style: rustfmt

## 0.14.0
- feat: make failure error crate optional. To revert back to old behavior, use the `--failure` feature.

## 0.13.3
- feat: allow changing name without deallocating `BytesStart` buffer
- feat: add standard error type conversion

## 0.13.2
- fix: allow whitespace in End events
- feat: bump dependencies

## 0.13.1
- feat: Add into_underlying_reader method for `Reader<BufRead + Seek>`

## 0.13.0
- feat: rename `resolve_namespace` into `attribute_namespace`
- feat: add a `event_namespace` fn

## 0.12.4
- fix: Fix minor bug for parsing comment tag

## 0.12.3
- feat: add `BytesStart::{owned_name, borrowed_name}`

## 0.12.2
- refactor: bump dependencies
- test: fix travis

## 0.12.1
- feat: enable `into_owned` for all events

## 0.12.0
- feat: rename BytesText fn to better clarify escape intents
- docs: various improvements!

## 0.11.0
- feat: migrate from error-chain to failure
- feat: allow html style attribute iterators
- feat: add optional identation on writer
- refactor: remove unecessary derive impl

## 0.10.1
- fix: overflow possibility when parsing Ascii codes

## 0.10.0
- feat: update dependencies
- doc: add doc for attribute creation functions
- fix: escape attributes
- fix: avoid double escapes

## 0.9.4
- fix: bound tests in `read_bang` fn.

## 0.9.3
- fix: escape was panicking at the 3rd character escaped.

## 0.9.2
- perf: update to encoding_rs 0.7.0, supposedly faster for utf8
- style: rustfmt-nightly

## 0.9.1
- perf: use memchr crate and rewrite some loops with iterators
- docs: remove duplicate `Reader` doc in lib.rs

## 0.9.0
- feat: add getter for encoding to reader
- feat: escape Text events on write (breaking change)

## 0.8.1
- feat: allow `Writer` to borrow `Event` (using `AsRef<Event>`)

## 0.8.0
- fix: make the reader borrow the namespace buffer so it can be used repetitively
- refactor: bump dependencies

## 0.7.3
- fix: fix Event::Text slice always starting at the beginning of the buffer

## 0.7.2
- perf: faster unescape method
- docs: update readme
- refactor bump encoding_rs to 0.6.6

## 0.7.1
- style: rustfmt
- refactor: remove from_ascii crate dependency

## 0.7.0
- style: rustfmt
- fix: {with,extend}_attributes usage
- feat: add naive `local_name` function

## 0.6.2
- fix: another overflow bug found with cargo-fuzz
- refactor: update dependencies

## 0.6.1
- fix: fix an overflow found with cargo-fuzz

## 0.6.0
Major refactoring. Breaks most of existing functionalities
- refactor: replace `XmlReader` with a non allocating `Reader` (uses an external buffer)
- refactor: replace `XmlnsReader` iterator by a simpler `Reader::read_namespaced_event` function
- refactor: replace `UnescapedAttribute` with a new `Attribute` struct with `unescape` functions
- feat: support xml decodings
- refactor: remove the `AsStr` trait: user must use `unescape_and_decode` fns when necessary
  (alternatively, run `unescape` and/or `Reader::decode`)
- refactor: module hierarchies
- refactor: replace `Element`s with several per event structs `BytesStart`
- perf: unescape: use from-ascii crate instead to get ascii codes without string validation
- refactor: rename `XmlWriter` to `Writer` and provide a way to write `&[u8]` directly
- refactor: adds @vandenoever changes to save some namespaces allocations
- refactor: adds error-chain and remove `ResultPos` (user can still use `Reader::buffer_position` if needed)

## 0.5.0
- feat: apply default namespaces (`xmlns="..."`) to unqualified elements
- fix: scope for namespace resolution on empty elements
- fix: parsing of `>` in attribute values

## 0.4.2
- feat: add `into_unescaped_string`
- refactor: remove RustyXML benches
- docs: redirect to docs.rs for documentation
- docs: add examples in lib.rs

## 0.4.1
- feat: add `read_text_unescaped`
- fix: fix tests

## 0.4.0
- fix: fix attributes with `=` character in their value
- perf: inline some local functions

## 0.3.1
- feat: set default to `expand_empty_elements = true`
- fix: fix all broken tests because of `Empty` events

## 0.2.5 - 0.3.0 (yanked)
- feat: Add support for `Empty` event

## 0.2.4
- test: add most tests from xml-rs crate

## 0.2.3
- fix: do not write attributes on `Event::End`

## 0.2.2
- refactor: code refactoring, split largest functions into smaller ones
- refactor: use `Range` instead of `usize`s in `Element` definition
- docs: fix typo

## 0.2.1
- feat: add `Clone` to more structs
- style: apply rustfmt

## 0.2.0
- refactor: change `from_str` into impl `From<&str>`
- feat: support `Event::DocType`
- feat: add `.check_comments` to check for invalid double dashes (`--`) in comments
- fix: check that all attributes are distincts

## v0.1.9
- feat: return more precise index when erroring
- feat: have `Attributes` iterate ResultPos instead of `Result`
- feat: provide functions to unescape `&...;` characters (`.escaped_content` and `.escaped_attributes`)
- fix: have namespace resolution start one level higher

## v0.1.8
- feat: add `XmlnsReader` to iterate event and resolve namespaces!
- docs: better documentation (in particular regarding `Element` structure and design)
- test: add benchmarks, with xml-rs for a reference

## 0.1.7
- feat/fix: add `Event::PI` to manage processing instructions (`<?...?>`)
- test: add test with a sample file

## 0.1.6
- feat: parse `Event::Decl` for xml declaration so we can have `version`, `encoding` ...
- refactor: rename `position` into `buffer_position` because it sometimes conflicted with `Iterator::position`
- test: add test for buffer_position

## 0.1.5
- feat: add buffer position when erroring to help debuging (return `ResultPos` instead of `Result`)
- test: add travis CI
- docs: add merrit badge and travis status

## 0.1.4
- feat: improve Element API with new, with_attributes, push_attribute
- feat: always return raw `&[u8]` and add a `AsStr` trait for conversion

## 0.1.3
- feat: add helper functions
- feat: add `XmlWriter` to write/modify xmls
- feat: use `AsRef<[u8]>` when possible

## 0.1.2 - 0.1.1
- test: add tests
- feat: add `with_check`
