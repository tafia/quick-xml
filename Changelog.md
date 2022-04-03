> Release checklist (minimal list of actions for cutting a release):
> 0. Check if dependencies in child projects (mostly `compare`) need to be updated and update them
> 1. `$env:RUSTDOCFLAGS="--cfg docsrs"; cargo +nightly doc --all-features` and check generated documentation for missing / unclear things
> 2. Update version in `Cargo.toml`
> 3. Update `Changelog.md` with date of release, add new empty Unreleased headings
> 4. Commit changes with message "Release x.y.z"
> 5. `cargo package` for final check
> 6. Push `master` to my fork, wait while CI pass. Repeat with force pushes if necessary
> 7. `cargo publish`
> 8. Create and push tag `vx.y.z` and push `master` to upstream
> 9. Create a Release on GitHub (in GitHub UI)

----------------------------------------------------------------------------------------------------

## Unreleased

### New Features

- [#938]: Add new enumeration `XmlVersion` and typified getter `BytesDecl::xml_version()`.
- [#938]: Add new error variant `IllFormedError::UnknownVersion`.
- [#371]: Add new error variant `EscapeError::TooManyNestedEntities`.
- [#371]: Improved compliance with the XML attribute value normalization process by adding
  - `Attribute::normalized_value()`
  - `Attribute::normalized_value_with()`
  - `Attribute::decoded_and_normalized_value()`
  - `Attribute::decoded_and_normalized_value_with()`

  which ought to be used in place of deprecated
  - `Attribute::unescape_value()`
  - `Attribute::unescape_value_with()`
  - `Attribute::decode_and_unescape_value()`
  - `Attribute::decode_and_unescape_value_with()`

  Deprecated functions now behaves the same as newly added.

### Bug Fixes

- [#938]: Use correct rules for EOL normalization in `Deserializer` when parse XML 1.0 documents.
  Previously XML 1.1. rules was applied.

### Misc Changes

- [#914]: Remove deprecated `.prefixes()`, `.resolve()`, `.resolve_attribute()`, and `.resolve_element()`
  of `NsReader`. Use `.resolver().<...>` methods instead.
- [#938]: Now `BytesText::xml_content`, `BytesCData::xml_content` and `BytesRef::xml_content`
  accepts `XmlVersion` parameter to apply correct EOL normalization rules.

[#371]: https://github.com/tafia/quick-xml/issues/371
[#914]: https://github.com/tafia/quick-xml/pull/914
[#938]: https://github.com/tafia/quick-xml/pull/938


## 0.39.2 -- 2026-02-20

### New Features

- [#483]: Implement `read_text_into()` and `read_text_into_async()`.

### Bug Fixes

- [#939]: Fix parsing error of the tag from buffered reader, when the first byte `<`
  is the last in the `BufRead` internal buffer. This is the regression from [#936].

[#483]: https://github.com/tafia/quick-xml/issues/483
[#936]: https://github.com/tafia/quick-xml/pull/936
[#939]: https://github.com/tafia/quick-xml/issues/939


## 0.39.1 -- 2026-02-15

### New Features

- [#598]: Add method `NamespaceResolver::set_level` which may be helpful in some circumstances.

### Bug Fixes

- [#597]: Fix incorrect processing of namespace scopes in `NsReader::read_to_end`
  `NsReader::read_to_end_into`, `NsReader::read_to_end_into_async` and `NsReader::read_text`.
  The scope started by a start element was not ended after that call.
- [#936]: Fix incorrect result of `.read_text()` when it is called after reading `Text` or `GeneralRef` event.

[#597]: https://github.com/tafia/quick-xml/issues/597
[#598]: https://github.com/tafia/quick-xml/pull/598
[#936]: https://github.com/tafia/quick-xml/pull/936


## 0.39.0 -- 2026-01-11

Added a way to configure `Writer`. Now all configuration is contained in the `writer::Config`
struct and can be applied at once. When `serde-types` feature is enabled, configuration is serializable.

### New Features

- [#846]: Add methods `config()` and `config_mut()` to inspect and change the writer configuration.
- [#846]: Add ability to write space before `/>` in self-closed tags for maximum compatibility with
  XHTML.
- [#846]: Add method `empty_element_handling()` as a more powerful alternative to `expand_empty_elements()`
  in `Serializer`.
- [#929]: Allow to pass list of field names to `impl_deserialize_for_internally_tagged_enum!` macro
  which is required if you enum variants contains `$value` fields.

### Bug Fixes

- [#923]: Implement correct skipping of well-formed DTD.

### Misc Changes

- [#908]: Increase minimal supported `serde` version from 1.0.139 to 1.0.180.
- [#913]: Deprecate `.prefixes()`, `.resolve()`, `.resolve_attribute()`, and `.resolve_element()`
  of `NsReader`. Use `.resolver().bindings()` and `.resolver().resolve()` methods instead.
- [#913]: `Attributes::has_nil` now accepts `NamespaceResolver` instead of `Reader<R>`.
- [#924]: (breaking change) Split `SyntaxError::UnclosedPIOrXmlDecl` into `UnclosedPI` and
  `UnclosedXmlDecl` for more precise error reporting.
- [#924]: (breaking change) `Parser::eof_error` now takes `&self` and content `&[u8]` parameters.
- [#926]: (breaking change) Split `SyntaxError::UnclosedTag` into `UnclosedTag`,
  `UnclosedSingleQuotedAttributeValue` and `UnclosedDoubleQuotedAttributeValue` for more precise error reporting.

[#846]: https://github.com/tafia/quick-xml/issues/846
[#908]: https://github.com/tafia/quick-xml/pull/908
[#913]: https://github.com/tafia/quick-xml/pull/913
[#923]: https://github.com/tafia/quick-xml/issues/923
[#924]: https://github.com/tafia/quick-xml/pull/924
[#926]: https://github.com/tafia/quick-xml/issues/926
[#929]: https://github.com/tafia/quick-xml/pull/929


## 0.38.4 -- 2025-11-11

### New Features

- [#353]: Add ability to serialize textual content as CDATA sections in `Serializer`.
  Everywhere where the text node may be created, a CDATA section(s) could be produced instead.
  See the new [`Serializer::text_format()`] method.

### Bug Fixes

- [#912]: Fix deserialization of numbers, booleans and characters that is space-wrapped, for example
  `<int>  42  </int>`. That space characters are usually indent added during serialization and
  other XML serialization libraries trims them

### Misc Changes

- [#901]: Fix running tests on 32-bit architecture
- [#909]: Avoid some allocations in the `Serializer`

[#353]: https://github.com/tafia/quick-xml/issues/353
[#901]: https://github.com/tafia/quick-xml/pull/901
[#909]: https://github.com/tafia/quick-xml/pull/909
[#912]: https://github.com/tafia/quick-xml/pull/912
[`Serializer::text_format()`]: https://docs.rs/quick-xml/0.38.4/quick_xml/se/struct.Serializer.html#method.text_format


## 0.38.3 -- 2025-08-24

### Bug Fixes

- [#895]: Fix incorrect normalization of `\rX` EOL sequences where `X` is a char which is
  UTF-8 encoded as [c2 xx], except [c2 85].

### Misc Changes

- [#895]: Add new `xml10_content()` and `xml11_content()` methods which behaves the same as
  `html_content()` and `xml_content()` methods, but express intention more clearly.

[#895]: https://github.com/tafia/quick-xml/pull/895


## 0.38.2 -- 2025-08-19

### New Features

- [#893]: Implement `FusedIterator` for `NamespaceBindingsIter`.
- [#893]: Make `NamespaceResolver` public.
- [#893]: Add `NsReader::resolver()` for access to namespace resolver.

### Misc Changes

- [#893]: Rename `PrefixIter` to `NamespaceBindingsIter`.

[#893]: https://github.com/tafia/quick-xml/pull/893


## 0.38.1 -- 2025-08-03

### Important changes

To get text in events according to the XML specification (normalized EOLs) use the
new methods `xml_content()` instead of `decode()`. `Deserializer` uses new method
automatically.

### New Features

- [#882]: Add new methods to create `Deserializer` from existing `NsReader`:
  - `Deserializer::borrowing`
  - `Deserializer::borrowing_with_resolver`
  - `Deserializer::buffering`
  - `Deserializer::buffering_with_resolver`
- [#878]: Add ability to serialize structs in `$value` fields. The struct name will
  be used as a tag name. Previously only enums was allowed there.
- [#806]: Add `BytesText::xml_content`, `BytesCData::xml_content` and `BytesRef::xml_content`
  methods which returns XML EOL normalized strings.
- [#806]: Add `BytesText::html_content`, `BytesCData::html_content` and `BytesRef::html_content`
  methods which returns HTML EOL normalized strings.

### Bug Fixes

- [#806]: Properly normalize EOL characters in `Deserializer`.
- [#888]: Properly split attribute values by items when deserialize attribute into
  list of values and attribute requires decoding.

[#806]: https://github.com/tafia/quick-xml/issues/806
[#878]: https://github.com/tafia/quick-xml/pull/878
[#882]: https://github.com/tafia/quick-xml/pull/882
[#888]: https://github.com/tafia/quick-xml/pull/888


## 0.38.0 -- 2025-06-28

### Significant changes

Now references to entities (as predefined, such as `&lt;`, as user-defined) reported as a new
`Event::GeneralRef`.
Caller can parse the content of the entity and stream events from it as it is required by the
XML specification. See the updated `custom_entities` example!

Implement whitespace behavior in the standard in `Deserializer`, which says string primitive
types should preserve whitespace, while all other primitives have collapse behavior.

### New Features

- [#863]: Add `Attributes::into_map_access(&str)` and `Attributes::into_deserializer()` when `serialize`
  feature is enabled. This will allow do deserialize serde types right from attributes. Both methods
  returns the same type which implements serde's `Deserializer` and `MapAccess` traits.
- [#766]: Allow to parse resolved entities as XML fragments and stream events from them.
- [#766]: Added new event `Event::GeneralRef` with content of [general entity].
- [#766]: Added new configuration option `allow_dangling_amp` which allows to have
  a `&` not followed by `;` in the textual data which is required for some applications
  for compatibility reasons.
- [#285]: Add ability to `quick_xml::de::Text` to access text with trimmed spaces

### Bug Fixes

- [#868]: Allow to have both `$text` and `$value` special fields in one struct. Previously
  any text will be recognized as `$value` field even when `$text` field is also presented.
- [#868]: Skip text events when deserialize a sequence of items overlapped with text (including CDATA).
- [#841]: Do not strip `xml` prefix from the attributes when map them to struct fields in `Deserializer`.

### Misc Changes

- [#863]: Remove `From<QName<'a>> for BytesStart<'a>` because now `BytesStart` stores the
  encoding in which its data is encoded, but `QName` is a simple wrapper around byte slice.
- [#766]: `BytesText::unescape` and `BytesText::unescape_with` replaced by `BytesText::decode`.
  Now Text events does not contain escaped parts which are reported as `Event::GeneralRef`.

[#285]: https://github.com/tafia/quick-xml/issues/285
[#766]: https://github.com/tafia/quick-xml/pull/766
[#841]: https://github.com/tafia/quick-xml/issues/841
[#863]: https://github.com/tafia/quick-xml/pull/863
[#868]: https://github.com/tafia/quick-xml/pull/868
[general entity]: https://www.w3.org/TR/xml11/#gen-entity


## 0.37.5 -- 2025-04-27

### New Features

- [#857]: Add `BytesCData::decode()`.

[#857]: https://github.com/tafia/quick-xml/pull/857


## 0.37.4 -- 2025-04-01

### Misc Changes

- [#852]: Add `Debug` impl for `NsReader` and `Reader` and `Clone` impl for `NsReader`

[#852]: https://github.com/tafia/quick-xml/pull/852


## 0.37.3 -- 2025-03-25

### New Features

- [#850]: Add `Attribute::as_bool()` method to get an attribute value as a boolean.
- [#850]: Add `Attributes::has_nil()` method to check if attributes has `xsi:nil` attribute set to `true`.
- [#497]: Handle `xsi:nil` attribute in serde Deserializer to better process optional fields.

[#497]: https://github.com/tafia/quick-xml/issues/497
[#850]: https://github.com/tafia/quick-xml/pull/850


## 0.37.2 -- 2024-12-29

### New Features

- [#836]: Add `se::to_utf8_io_writer()` helper compatible with `std::io::Write` and restricted to UTF-8 encoding.

[#836]: https://github.com/tafia/quick-xml/pull/836


## 0.37.1 -- 2024-11-17

### New Features

- [#831]: Add `BytesCData::escaped()` fn to construct CDATA events from arbitrary user input.

[#831]: https://github.com/tafia/quick-xml/issues/831


## 0.37.0 -- 2024-10-27

### New Features

- [#826]: Implement `From<String>` and `From<Cow<str>>` for `quick_xml::de::Text`.
- [#826]: Make `SimpleTypeDeserializer` and `SimpleTypeSerializer` public.
- [#826]: Implement `IntoDeserializer` for `&mut Deserializer`.

### Bug Fixes

- [#655]: Do not write indent before and after `$text` fields and those `$value` fields
  that are serialized as a text (for example, `usize` or `String`).
- [#826]: Handle only those boolean representations that are allowed by [Xml Schema]
  which is only `"true"`, `"1"`, `"false"`, and `"0"`. Previously the following values
  also was accepted:
  |`bool` |XML content
  |-------|-------------------------------------------------------------
  |`true` |`"True"`,  `"TRUE"`,  `"t"`, `"Yes"`, `"YES"`, `"yes"`, `"y"`
  |`false`|`"False"`, `"FALSE"`, `"f"`, `"No"`,  `"NO"`,  `"no"`,  `"n"`

### Misc Changes

- [#227]: Split `SeError` from `DeError` in the `serialize` feature.
  Serialize functions and methods now return `SeError`.
- [#810]: Return `std::io::Error` from `Writer` methods.
- [#811]: Split `NamespaceError` and `EncodingError` from `Error`.
- [#811]: Renamed `Error::EscapeError` to `Error::Escape` to match other variants.
- [#811]: Narrow down error return type from `Error` where only one variant is ever returned:
  attribute related methods on `BytesStart` and `BytesDecl` returns `AttrError`
- [#820]: Classify output of the `Serializer` by returning an enumeration with kind of written data
- [#823]: Do not allow serialization of consequent primitives, for example `Vec<usize>` or
  `Vec<String>` in `$value` fields. They cannot be deserialized back with the same result
- [#827]: Make `escape` and it variants take a `impl Into<Cow<str>>` argument and implement
  `From<(&'a str, Cow<'a, str>)>` on `Attribute`
- [#826]: Removed `DeError::InvalidInt`, `DeError::InvalidFloat` and `DeError::InvalidBoolean`.
  Now the responsibility for returning the error lies with the visitor of the type.
  See rationale in https://github.com/serde-rs/serde/pull/2811

[#227]: https://github.com/tafia/quick-xml/issues/227
[#655]: https://github.com/tafia/quick-xml/issues/655
[#810]: https://github.com/tafia/quick-xml/pull/810
[#811]: https://github.com/tafia/quick-xml/pull/811
[#820]: https://github.com/tafia/quick-xml/pull/820
[#823]: https://github.com/tafia/quick-xml/pull/823
[#826]: https://github.com/tafia/quick-xml/pull/826
[#827]: https://github.com/tafia/quick-xml/pull/827
[Xml Schema]: https://www.w3.org/TR/xmlschema11-2/#boolean


## 0.36.2 -- 2024-09-20

### Bug Fixes

- [#533]: Fix incorrect DocType closing bracket detection when parsing with buffered reader

[#533]: https://github.com/tafia/quick-xml/issues/533


## 0.36.1 -- 2024-07-23

### New Features

- [#623]: Added `Reader::stream()` that can be used to read arbitrary data
  from the inner reader while track position for XML reader.

[#623]: https://github.com/tafia/quick-xml/issues/623


## 0.36.0 -- 2024-07-08

### Bug Fixes

- [#781]: Fix conditions to start CDATA section. Only uppercase `<![CDATA[` can start it.
  Previously any case was allowed.
- [#780]: Fixed incorrect `.error_position()` when encountering syntax error for open or self-closed tag.

### Misc Changes

- [#780]: `reader::Parser`, `reader::ElementParser` and `reader::PiParser` moved to the new module `parser`.
- [#776]: Allow to have attributes in the end tag for compatibility reasons with Adobe Flash XML parser.

[#776]: https://github.com/tafia/quick-xml/issues/776
[#780]: https://github.com/tafia/quick-xml/pull/780
[#781]: https://github.com/tafia/quick-xml/pull/781


## 0.35.0 -- 2024-06-29

### New Features

- [#772]: Add `reader::Config::allow_unmatched_ends` to permit dangling end tags

### Bug Fixes

- [#773]: Fixed reporting incorrect end position in `Reader::read_to_end` family
  of methods and trimming of the trailing spaces in `Reader::read_text` when
  `trim_text_start` is set and the last event is not a `Text` event.
- [#771]: Character references now allow any number of leading zeroes as it should.
  As a result, the following variants of `quick_xml::escape::EscapeError` are removed:
  - `TooLongDecimal`
  - `TooLongHexadecimal`
- [#771]: Fixed `Attribute::unescape_value` which does not unescape predefined values since 0.32.0.
- [#774]: Fixed regression since 0.33.0: `Text` event may be skipped in `read_event_into()`
  and `read_event_into_async()` in some circumstances.

### Misc Changes

- [#771]: `EscapeError::UnrecognizedSymbol` renamed to `EscapeError::UnrecognizedEntity`.
- [#771]: Implemented `PartialEq` for `EscapeError`.
- [#771]: Replace the following variants of `EscapeError` by `InvalidCharRef` variant
  with a new `ParseCharRefError` inside:
  - `EntityWithNull`
  - `InvalidDecimal`
  - `InvalidHexadecimal`
  - `InvalidCodepoint`

[#771]: https://github.com/tafia/quick-xml/pull/771
[#772]: https://github.com/tafia/quick-xml/pull/772
[#773]: https://github.com/tafia/quick-xml/pull/773
[#774]: https://github.com/tafia/quick-xml/issues/774


## 0.34.0 -- 2024-06-25

### Bug Fixes

- [#751]: Fix internal overflow when read 4GB+ files on 32-bit targets using `Reader<impl BufRead>` readers.

### Misc Changes

- [#760]: `Attribute::decode_and_unescape_value` and `Attribute::decode_and_unescape_value_with` now
  accepts `Decoder` instead of `Reader`. Use `Reader::decoder()` to get it.
- [#760]: `Writer::write_event` now consumes event. Use `Event::borrow()` if you want to keep ownership.
- [#751]: Type of `Reader::error_position()` and `Reader::buffer_position()` changed from `usize` to `u64`.
- [#751]: Type alias `Span` changed from `Range<usize>` to `Range<u64>`.

[#751]: https://github.com/tafia/quick-xml/issues/751
[#760]: https://github.com/tafia/quick-xml/pull/760


## 0.33.0 -- 2024-06-21

### New Features

- [#758]: Implemented `From<QName>` for `BytesStart` and `BytesEnd`.

### Bug Fixes

- [#755]: Fix incorrect missing of trimming all-space text events when
  `trim_text_start = false` and `trim_text_end = true`.

### Misc Changes

- [#650]: Change the type of `Event::PI` to a new dedicated `BytesPI` type.
- [#759]: Make `const` as much functions as possible:
  - `resolve_html5_entity()`
  - `resolve_predefined_entity()`
  - `resolve_xml_entity()`
  - `Attr::key()`
  - `Attr::value()`
  - `Attributes::html()`
  - `Attributes::new()`
  - `BytesDecl::from_start()`
  - `Decoder::encoding()`
  - `Deserializer::get_ref()`
  - `IoReader::get_ref()`
  - `LocalName::into_inner()`
  - `Namespace::into_inner()`
  - `NsReader::config()`
  - `NsReader::prefixes()`
  - `Prefix::into_inner()`
  - `QName::into_inner()`
  - `Reader::buffer_position()`
  - `Reader::config()`
  - `Reader::decoder()`
  - `Reader::error_position()`
  - `Reader::get_ref()`
  - `SliceReader::get_ref()`
  - `Writer::get_ref()`
  - `Writer::new()`
- [#763]: Hide `quick_xml::escape::resolve_html5_entity` under `escape-html` feature again.
  This function has significant influence to the compilation time (10+ seconds or 5x times)

[#650]: https://github.com/tafia/quick-xml/issues/650
[#755]: https://github.com/tafia/quick-xml/pull/755
[#758]: https://github.com/tafia/quick-xml/pull/758
[#759]: https://github.com/tafia/quick-xml/pull/759
[#763]: https://github.com/tafia/quick-xml/issues/763


## 0.32.0 -- 2024-06-10

The way to configure parser is changed. Now all configuration is contained in the
`Config` struct and can be applied at once. When `serde-types` feature is enabled,
configuration is serializable.

The method of reporting positions of errors has changed - use `error_position()`
to get an offset of the error position. For `SyntaxError`s the range
`error_position()..buffer_position()` also will represent a span of error.

The way of resolve entities with `unescape_with` are changed. Those methods no longer
resolve predefined entities.

### New Features

- [#513]: Allow to continue parsing after getting new `Error::IllFormed`.
- [#677]: Added methods `config()` and `config_mut()` to inspect and change the parser
  configuration. Previous builder methods on `Reader` / `NsReader` was replaced by
  direct access to fields of config using `reader.config_mut().<...>`.
- [#684]: Added a method `Config::enable_all_checks` to turn on or off all
  well-formedness checks.
- [#362]: Added `escape::minimal_escape()` which escapes only `&` and `<`.
- [#362]: Added `BytesCData::minimal_escape()` which escapes only `&` and `<`.
- [#362]: Added `Serializer::set_quote_level()` which allow to set desired level of escaping.
- [#705]: Added `NsReader::prefixes()` to list all the prefixes currently declared.
- [#629]: Added a default case to `impl_deserialize_for_internally_tagged_enum` macro so that
  it can handle every attribute that does not match existing cases within an enum variant.
- [#722]: Allow to pass owned strings to `Writer::create_element`. This is breaking change!
- [#275]: Added `ElementWriter::new_line()` which enables pretty printing elements with multiple attributes.
- [#743]: Added `Deserializer::get_ref()` to get XML Reader from serde Deserializer
- [#734]: Added helper functions to resolve predefined XML and HTML5 entities:
  - `quick_xml::escape::resolve_predefined_entity`
  - `quick_xml::escape::resolve_xml_entity`
  - `quick_xml::escape::resolve_html5_entity`
- [#753]: Added parser for processing instructions: `quick_xml::reader::PiParser`.
- [#754]: Added parser for elements: `quick_xml::reader::ElementParser`.

### Bug Fixes

- [#622]: Fix wrong disregarding of not closed markup, such as lone `<`.
- [#684]: Fix incorrect position reported for `Error::IllFormed(DoubleHyphenInComment)`.
- [#684]: Fix incorrect position reported for `Error::IllFormed(MissingDoctypeName)`.
- [#704]: Fix empty tags with attributes not being expanded when `expand_empty_elements` is set to true.
- [#683]: Use local tag name when check tag name against possible names for field.
- [#753]: Correctly determine end of processing instructions and XML declaration.

### Misc Changes

- [#675]: Minimum supported version of serde raised to 1.0.139
- [#675]: Rework the `quick_xml::Error` type to provide more accurate information:
  - `Error::EndEventMismatch` replaced by `IllFormedError::MismatchedEndTag` in some cases
  - `Error::EndEventMismatch` replaced by `IllFormedError::UnmatchedEndTag` in some cases
  - `Error::TextNotFound` was removed because not used
  - `Error::UnexpectedBang` replaced by `SyntaxError`
  - `Error::UnexpectedEof` replaced by `SyntaxError` in some cases
  - `Error::UnexpectedEof` replaced by `IllFormedError` in some cases
  - `Error::UnexpectedToken` replaced by `IllFormedError::DoubleHyphenInComment`
  - `Error::XmlDeclWithoutVersion` replaced by `IllFormedError::MissingDeclVersion` (in [#684])
  - `Error::EmptyDocType` replaced by `IllFormedError::MissingDoctypeName` (in [#684])
- [#684]: Changed positions reported for `SyntaxError`s: now they are always points
  to the start of markup (i. e. to the `<` character) with error. Use `error_position()`
  for that.
- [#684]: Now `<??>` parsed as `Event::PI` with empty content instead of raising
  syntax error.
- [#684]: Now `<?xml?>` parsed as `Event::Decl` instead of `Event::PI`.
- [#362]: Now default quote level is `QuoteLevel::Partial` when using serde serializer.
- [#689]: `buffer_position()` now always report the position the parser last seen.
  To get an error position use `error_position()`.
- [#738]: Add an example of how to deserialize XML elements into Rust enums using an
  intermediate custom deserializer.
- [#748]: Implement `Clone` for [`DeEvent`], [`PayloadEvent`] and [`Text`].
- [#734]: Rename `NoEntityResolver` to `PredefinedEntityResolver`.
- [#734]: No longer resolve predefined entities (`lt`, `gt`, `apos`, `quot`, `amp`)
  in `unescape_with` family of methods. You should do that by yourself using the methods
  listed above.

[#275]: https://github.com/tafia/quick-xml/issues/275
[#362]: https://github.com/tafia/quick-xml/issues/362
[#513]: https://github.com/tafia/quick-xml/issues/513
[#622]: https://github.com/tafia/quick-xml/issues/622
[#629]: https://github.com/tafia/quick-xml/issues/629
[#675]: https://github.com/tafia/quick-xml/pull/675
[#677]: https://github.com/tafia/quick-xml/pull/677
[#683]: https://github.com/tafia/quick-xml/issues/683
[#684]: https://github.com/tafia/quick-xml/pull/684
[#689]: https://github.com/tafia/quick-xml/pull/689
[#704]: https://github.com/tafia/quick-xml/pull/704
[#705]: https://github.com/tafia/quick-xml/pull/705
[#722]: https://github.com/tafia/quick-xml/pull/722
[#734]: https://github.com/tafia/quick-xml/pull/734
[#738]: https://github.com/tafia/quick-xml/pull/738
[#743]: https://github.com/tafia/quick-xml/pull/743
[#748]: https://github.com/tafia/quick-xml/pull/748
[#753]: https://github.com/tafia/quick-xml/pull/753
[#754]: https://github.com/tafia/quick-xml/pull/754
[`DeEvent`]: https://docs.rs/quick-xml/latest/quick_xml/de/enum.DeEvent.html
[`PayloadEvent`]: https://docs.rs/quick-xml/latest/quick_xml/de/enum.PayloadEvent.html
[`Text`]: https://docs.rs/quick-xml/latest/quick_xml/de/struct.Text.html


## 0.31.0 -- 2023-10-22

MSRV bumped to 1.56! Crate now uses Rust 2021 edition.

Enum representation was changed (it was buggy anyway) to ensure compatibility with
serde >= 1.0.181

### New Features

- [#545]: Resolve well-known namespaces (`xml` and `xmlns`) to their appropriate URIs.
  Also, enforce namespace constraints related to these well-known namespaces.
- [#635]: Add support for async `ElementWriter` operations.

### Bug Fixes

- [#660]: Fixed incorrect deserialization of `xs:list`s from empty tags (`<tag/>`
  or `<tag></tag>`). Previously an `DeError::UnexpectedEof")` was returned in that case
- [#580]: Fixed incorrect deserialization of vectors of newtypes from sequences of tags.
- [#661]: More string handling of serialized primitive values (booleans, numbers, strings,
  unit structs, unit variants). `<int>123<something-else/></int>` is no longer valid
  content. Previously all data after `123` up to closing tag would be silently skipped.
- [#567]: Fixed incorrect deserialization of vectors of enums from sequences of tags.
- [#671]: Fixed deserialization of empty `simpleType`s (for example, attributes) into
  `Option` fields: now they are always deserialized as `Some("")`.

### Misc Changes

- [#643]: Bumped MSRV to 1.56. In practice the previous MSRV was incorrect in many cases.
- [#643]: Adopted Rust 2021 edition.
- [#545]: Added new `Error` variant -- `Error::InvalidPrefixBind`.
- [#651]: Relax requirement for version of `arbitrary` dependency -- we're actually
  compatible with version 1.0.0 and up.
- [#649]: Make features linkable and reference them in the docs.
- [#619]: Allow to raise application errors in `ElementWriter::write_inner_content`
  (and newly added `ElementWriter::write_inner_content_async` of course).
- [#662]: Get rid of some allocations during serde deserialization.
- [#665]: Improve serialization of `xs:list`s when some elements serialized to an empty string.
- [#630]: Fixed compatibility with serde >= 1.0.181

[#545]: https://github.com/tafia/quick-xml/pull/545
[#567]: https://github.com/tafia/quick-xml/issues/567
[#580]: https://github.com/tafia/quick-xml/issues/580
[#619]: https://github.com/tafia/quick-xml/issues/619
[#630]: https://github.com/tafia/quick-xml/issues/630
[#635]: https://github.com/tafia/quick-xml/pull/635
[#643]: https://github.com/tafia/quick-xml/pull/643
[#649]: https://github.com/tafia/quick-xml/pull/646
[#651]: https://github.com/tafia/quick-xml/pull/651
[#660]: https://github.com/tafia/quick-xml/pull/660
[#661]: https://github.com/tafia/quick-xml/pull/661
[#662]: https://github.com/tafia/quick-xml/pull/662
[#665]: https://github.com/tafia/quick-xml/pull/665
[#671]: https://github.com/tafia/quick-xml/issues/671


## 0.30.0 -- 2023-07-23

### New Features

- [#609]: Added `Writer::write_serializable` to provide the capability to serialize
  arbitrary types using serde when using the lower-level `Writer` API.
- [#615]: Added ability to set entity resolver when deserialize using borrowing reader.
- [#617]: Added ability to enforce the expansion of empty elements.

### Bug Fixes

- [#604]: Avoid crashing on wrong comments like `<!-->` when using `read_event_into*` functions.

### Misc Changes

[#604]: https://github.com/tafia/quick-xml/issue/604
[#609]: https://github.com/tafia/quick-xml/pull/609
[#615]: https://github.com/tafia/quick-xml/pull/615
[#617]: https://github.com/tafia/quick-xml/pull/617


## 0.29.0 -- 2023-06-13

### New Features

- [#601]: Add `serde_helper` module to the crate root with some useful utility
  functions and document using of enum's unit variants as a text content of element.
- [#606]: Implement indentation for `AsyncWrite` trait implementations.

### Bug Fixes

- [#603]: Fix a regression from [#581] that an XML comment or a processing
  instruction between a <!DOCTYPE> and the root element in the file broke
  deserialization of structs by returning `DeError::ExpectedStart`
- [#608]: Return a new error `Error::EmptyDocType` on empty doctype instead
  of crashing because of a debug assertion.

### Misc Changes

- [#594]: Add a helper macro to help deserialize internally tagged enums
  with Serde, which doesn't work out-of-the-box due to serde limitations.

[#581]: https://github.com/tafia/quick-xml/pull/581
[#594]: https://github.com/tafia/quick-xml/pull/594
[#601]: https://github.com/tafia/quick-xml/pull/601
[#603]: https://github.com/tafia/quick-xml/pull/603
[#606]: https://github.com/tafia/quick-xml/pull/606
[#608]: https://github.com/tafia/quick-xml/issues/608


## 0.28.2 -- 2023-04-12

### New Features

- [#581]: Allow `Deserializer` to set `quick_xml::de::EntityResolver` for
  resolving unknown entities that would otherwise cause the parser to return
  an [`EscapeError::UnrecognizedSymbol`] error.

### Misc Changes

- [#584]: Export `EscapeError` from the crate
- [#581]: Relax requirements for `unsescape_*` set of functions -- their now use
  `FnMut` instead of `Fn` for `resolve_entity` parameters, like `Iterator::map`
  from `std`.

[#581]: https://github.com/tafia/quick-xml/pull/581
[#584]: https://github.com/tafia/quick-xml/pull/584


## 0.28.1 -- 2023-03-19

### Misc Changes

- [#579]: `ElementWriter.write_inner_content` now uses a `FnOnce` instead of a more restrictive `Fn` closure

[#579]: https://github.com/tafia/quick-xml/pull/579


## 0.28.0 -- 2023-03-13

### New Features

- [#541]: (De)serialize specially named `$text` enum variant in [externally tagged]
  enums to / from textual content
- [#556]: `to_writer` and `to_string` now accept `?Sized` types
- [#556]: Add new `to_writer_with_root` and `to_string_with_root` helper functions
- [#520]: Add methods `BytesText::inplace_trim_start` and `BytesText::inplace_trim_end`
  to trim leading and trailing spaces from text events
- [#565]: Allow deserialize special field names `$value` and `$text` into borrowed
  fields when use serde deserializer
- [#568]: Rename `Writer::inner` into `Writer::get_mut`
- [#568]: Add method `Writer::get_ref`
- [#569]: Rewrite the `Reader::read_event_into_async` as an async fn, making the future `Send` if possible.
- [#571]: Borrow element names (`<element>`) when deserialize with serde.
  This change allow to deserialize into `HashMap<&str, T>`, for example
- [#573]: Add basic support for async byte writers via tokio's `AsyncWrite`.

### Bug Fixes

- [#537]: Restore ability to deserialize attributes that represents XML namespace
  mappings (`xmlns:xxx`) that was broken since [#490]
- [#510]: Fix an error of deserialization of `Option<T>` fields where `T` is some
  sequence type (for example, `Vec` or tuple)
- [#540]: Fix a compilation error (probably a rustc bug) in some circumstances.
  `Serializer::new` and `Serializer::with_root` now accepts only references to `Write`r.
- [#520]: Merge consequent (delimited only by comments and processing instructions)
  texts and CDATA when deserialize using serde deserializer. `DeEvent::Text` and
  `DeEvent::CData` events was replaced by `DeEvent::Text` with merged content.
  The same behavior for the `Reader` does not implemented (yet?) and should be
  implemented manually
- [#562]: Correctly set minimum required version of memchr dependency to 2.1
- [#565]: Correctly set minimum required version of tokio dependency to 1.10
- [#565]: Fix compilation error when build with serde <1.0.139


[externally tagged]: https://serde.rs/enum-representations.html#externally-tagged
[#490]: https://github.com/tafia/quick-xml/pull/490
[#510]: https://github.com/tafia/quick-xml/issues/510
[#520]: https://github.com/tafia/quick-xml/pull/520
[#537]: https://github.com/tafia/quick-xml/issues/537
[#540]: https://github.com/tafia/quick-xml/issues/540
[#541]: https://github.com/tafia/quick-xml/pull/541
[#556]: https://github.com/tafia/quick-xml/pull/556
[#562]: https://github.com/tafia/quick-xml/pull/562
[#565]: https://github.com/tafia/quick-xml/pull/565
[#568]: https://github.com/tafia/quick-xml/pull/568
[#569]: https://github.com/tafia/quick-xml/pull/569
[#571]: https://github.com/tafia/quick-xml/pull/571
[#573]: https://github.com/tafia/quick-xml/pull/573


## 0.27.1 -- 2022-12-28

### Bug Fixes

- [#530]: Fix an infinite loop leading to unbounded memory consumption that occurs when
  skipping events on malformed XML with the `overlapped-lists` feature active.
- [#530]: Fix an error in the `Deserializer::read_to_end` when `overlapped-lists`
  feature is active and malformed XML is parsed

[#530]: https://github.com/tafia/quick-xml/pull/530


## 0.27.0 -- 2022-12-25

### New Features

- [#521]: Implement `Clone` for all error types. This required changing `Error::Io` to contain
  `Arc<std::io::Error>` instead of `std::io::Error` since `std::io::Error` does not implement
  `Clone`.

### Bug Fixes

- [#490]: Ensure that serialization of map keys always produces valid XML names.
  In particular, that means that maps with numeric and numeric-like keys (for
  example, `"42"`) no longer can be serialized because [XML name] cannot start
  from a digit
- [#500]: Fix deserialization of top-level sequences of enums, like
  ```xml
  <?xml version="1.0" encoding="UTF-8"?>
  <!-- list of enum Enum { A, B, С } -->
  <A/>
  <B/>
  <C/>
  ```
- [#514]: Fix wrong reporting `Error::EndEventMismatch` after disabling and enabling
  `.check_end_names`
- [#517]: Fix swapped codes for `\r` and `\n` characters when escaping them
- [#523]: Fix incorrect skipping text and CDATA content before any map-like structures
  in serde deserializer, like
  ```xml
  unwanted text<struct>...</struct>
  ```
- [#523]: Fix incorrect handling of `xs:list`s with encoded spaces: they still
  act as delimiters, which is confirmed also by mature XmlBeans Java library
- [#473]: Fix a hidden requirement to enable serde's `derive` feature to get
  quick-xml's `serialize` feature for `edition = 2021` or `resolver = 2` crates

### Misc Changes

- [#490]: Removed `$unflatten=` special prefix for fields for serde (de)serializer, because:
  - it is useless for deserializer
  - serializer was rewritten and does not require it anymore

  This prefix allowed you to serialize struct field as an XML element and now
  replaced by a more thoughtful system explicitly indicating that a field should
  be serialized as an attribute by prepending `@` character to its name
- [#490]: Removed `$primitive=` prefix. That prefix allowed you to serialize struct
  field as an attribute instead of an element and now replaced by a more thoughtful
  system explicitly indicating that a field should be serialized as an attribute
  by prepending `@` character to its name
- [#490]: In addition to the `$value` special name for a field a new `$text`
  special name was added:
  - `$text` is used if you want to map field to text content only. No markup is
    expected (but text can represent a list as defined by `xs:list` type)
  - `$value` is used if you want to map elements with different names to one field,
    that should be represented either by an `enum`, or by sequence of `enum`s
    (`Vec`, tuple, etc.), or by string. Use it when you want to map field to any
    content of the field, text or markup

  Refer to [documentation] for details.
- [#521]: MSRV bumped to 1.52.
- [#473]: `serde` feature that used to make some types serializable, renamed to `serde-types`
- [#528]: Added documentation for XML to `serde` mapping

[#473]: https://github.com/tafia/quick-xml/issues/473
[#490]: https://github.com/tafia/quick-xml/pull/490
[#500]: https://github.com/tafia/quick-xml/issues/500
[#514]: https://github.com/tafia/quick-xml/issues/514
[#517]: https://github.com/tafia/quick-xml/issues/517
[#521]: https://github.com/tafia/quick-xml/pull/521
[#523]: https://github.com/tafia/quick-xml/pull/523
[#528]: https://github.com/tafia/quick-xml/pull/528
[XML name]: https://www.w3.org/TR/xml11/#NT-Name
[documentation]: https://docs.rs/quick-xml/0.27.0/quick_xml/de/index.html#difference-between-text-and-value-special-names


## 0.26.0 -- 2022-10-23

### Misc Changes

- [#481]: Removed the uses of `const fn` added in version 0.24 in favor of a lower minimum
  supported Rust version (1.46.0).  Minimum supported Rust version is now verified in the CI.
- [#489]: Reduced the size of the package uploaded into the crates.io by excluding
  tests, examples, and benchmarks.

[#481]: https://github.com/tafia/quick-xml/pull/481
[#489]: https://github.com/tafia/quick-xml/pull/489


## 0.25.0 -- 2022-09-10

### Bug Fixes

- [#469]: Fix incorrect parsing of CDATA and comments when using buffered readers

### Misc Changes

- [#468]: Content of `DeError::Unsupported` changed from `&'static str` to `Cow<'static, str>`
- [#468]: Ensure that map keys are restricted to only types that can be serialized as primitives

[#468]: https://github.com/tafia/quick-xml/pull/468
[#469]: https://github.com/tafia/quick-xml/issues/469


## 0.24.1 -- 2022-09-10

### Bug Fixes

- [#469]: Fix incorrect parsing of CDATA and comments when using buffered readers

[#469]: https://github.com/tafia/quick-xml/issues/469


## 0.24.0 -- 2022-08-28

### New Features

- [#387]: Allow overlapping between elements of sequence and other elements
  (using new feature `overlapped-lists`)
- [#393]: New module `name` with `QName`, `LocalName`, `Namespace`, `Prefix`
  and `PrefixDeclaration` wrappers around byte arrays and `ResolveResult` with
  the result of namespace resolution
- [#180]: Make `Decoder` struct public. You already had access to it via the
  `Reader::decoder()` method, but could not name it in the code. Now the preferred
  way to access decoding functionality is via this struct
- [#395]: Add support for XML Schema `xs:list`
- [#324]: `Reader::from_str` / `Deserializer::from_str` / `from_str` now ignore
  the XML declared encoding and always use UTF-8
- [#416]: Add `borrow()` methods in all event structs which allows to get
  a borrowed version of any event
- [#437]: Split out namespace reading functionality to a dedicated `NsReader`, namely:
  |Old function in `Reader`|New function in `NsReader`
  |------------------------|--------------------------
  |                        |`read_event` -- borrow from input
  |                        |`read_resolved_event` -- borrow from input
  |                        |`read_event_into`
  |`read_namespaced_event` |`read_resolved_event_into`
  |                        |`resolve`
  |`event_namespace`       |`resolve_element`
  |`attribute_namespace`   |`resolve_attribute`
- [#439]: Added utilities `detect_encoding()` and `decode()` under the `quick-xml::encoding` namespace.
- [#450]: Added support of asynchronous [tokio](https://tokio.rs/) readers
- [#455]: Change return type of all `read_to_end*` methods to return a span between tags
- [#455]: Added `Reader::read_text` method to return a raw content (including markup) between tags
- [#459]: Added a `Writer::write_bom()` method for inserting a Byte-Order-Mark into the document.
- [#467]: The following functions made `const`:
  - `Attr::key`
  - `Attr::value`
  - `Attributes::html`
  - `Attributes::new`
  - `BytesDecl::from_start`
  - `Decoder::encoding`
  - `LocalName::into_inner`
  - `Namespace::into_inner`
  - `Prefix::into_inner`
  - `QName::into_inner`
  - `Reader::buffer_position`
  - `Reader::decoder`
  - `Reader::get_ref`
  - `Serializer::new`
  - `Serializer::with_root`
  - `Writer::new`

### Bug Fixes

- [#9]: Deserialization erroneously was successful in some cases where error is expected.
  This broke deserialization of untagged enums which rely on error if variant cannot be parsed
- [#387]: Allow to have an ordinary elements together with a `$value` field
- [#387]: Internal deserializer state can be broken when deserializing a map with
  a sequence field (such as `Vec<T>`), where elements of this sequence contains
  another sequence. This error affects only users with the `serialize` feature enabled
- [#393]: Now `event_namespace`, `attribute_namespace` and `read_event_namespaced`
  returns `ResolveResult::Unknown` if prefix was not registered in namespace buffer
- [#393]: Fix breaking processing after encounter an attribute with a reserved name (started with "xmlns")
- [#363]: Do not generate empty `Event::Text` events
- [#412]: Fix using incorrect encoding if `read_to_end` family of methods or `read_text`
  method not found a corresponding end tag and reader has non-UTF-8 encoding
- [#421]: Fix incorrect order of unescape and decode operations for serde deserializer:
  decoding should be first, unescape is the second
- [#421]: Fixed unknown bug in serde deserialization of externally tagged enums
  when an enum variant represented as a `Text` event (i.e. `<xml>tag</xml>`)
  and a document encoding is not an UTF-8
- [#434]: Fixed incorrect error generated in some cases by serde deserializer
- [#445]: Use local name without namespace prefix when selecting enum variants based on element names
  in a serde deserializer

### Misc Changes

- [#8]: Changes in the error type `DeError`:
  |Variant|Change
  |-------|---------------------------------------------------------------------
  |~~`DeError::Text`~~|Removed because never raised
  |~~`DeError::InvalidEnum`~~|Removed because never raised
  |`DeError::Xml`|Renamed to `DeError::InvalidXml` for consistency with `DeError::InvalidBoolean`
  |`DeError::Int`|Renamed to `DeError::InvalidInt` for consistency with `DeError::InvalidBoolean`
  |`DeError::Float`|Renamed to `DeError::InvalidFloat` for consistency with `DeError::InvalidBoolean`
  |`DeError::Start`|Renamed to `DeError::UnexpectedStart` and tag name added to an error
  |`DeError::End`|Renamed to `DeError::UnexpectedEnd` and tag name added to an error
  |`DeEvent::Eof`|Renamed to `DeError::UnexpectedEof`
  |`DeError::EndOfAttributes`|Renamed to `DeError::KeyNotFound`
  |`DeError::ExpectedStart`|Added

- [#391]: Added code coverage

- [#393]: `event_namespace` and `attribute_namespace` now accept `QName`
  and returns `ResolveResult` and `LocalName`, `read_event_namespaced` now
  returns `ResolveResult` instead of `Option<[u8]>`
- [#393]: Types of `Attribute::key` and `Attr::key()` changed to `QName`
- [#393]: Now `BytesStart::name()` and `BytesEnd::name()` returns `QName`, and
  `BytesStart::local_name()` and `BytesEnd::local_name()` returns `LocalName`

- [#191]: Remove unused `reader.decoder().decode_owned()`. If you ever used it,
  use `String::from_utf8` instead (which that function did)
- [#191]: Remove `*_without_bom` methods from the `Attributes` struct because they are useless.
  Use the same-named methods without that suffix instead. Attribute values cannot contain BOM
- [#191]: Remove `Reader::decode()` and `Reader::decode_without_bom()`, they are replaced by
  `Decoder::decode()` and nothing.
  Use `reader.decoder().decode_*(...)` instead of `reader.decode_*(...)` for now.
  `Reader::encoding()` is replaced by `Decoder::encoding()` as well
- [#180]: Eliminated the differences in the decoding API when feature `encoding` enabled and when it is
  disabled. Signatures of functions are now the same regardless of whether or not the feature is
  enabled, and an error will be returned instead of performing replacements for invalid characters
  in both cases.

  Previously, if the `encoding` feature was enabled, decoding functions would return `Result<Cow<&str>>`
  while without this feature they would return `Result<&str>`. With this change, only `Result<Cow<&str>>`
  is returned regardless of the status of the feature.
- [#180]: Error variant `Error::Utf8` replaced by `Error::NonDecodable`

- [#118]: Remove `BytesStart::unescaped*` set of methods because they could return wrong results
  Use methods on `Attribute` instead

- [#403]: Remove deprecated `quick_xml::de::from_bytes` and `Deserializer::from_borrowing_reader`

- [#412]: Rename methods of `Reader`:
  |Old Name                 |New Name
  |-------------------------|---------------------------------------------------
  |`read_event`             |`read_event_into`
  |`read_to_end`            |`read_to_end_into`
  |`read_text`              |`read_text_into`
  |`read_event_unbuffered`  |`read_event`
  |`read_to_end_unbuffered` |`read_to_end`
- [#412]: Change `read_to_end*` and `read_text_into` to accept `QName` instead of `AsRef<[u8]>`

- [#415]: Changed custom entity unescaping API to accept closures rather than a mapping of entity to
  replacement text. This avoids needing to allocate a map and provides the user with more flexibility.
- [#415]: Renamed functions for consistency across the API:
  |Old Name                |New Name
  |------------------------|-------------------------------------------
  |`*_with_custom_entities`|`*_with`
  |`BytesText::unescaped()`|`BytesText::unescape()`
  |`Attribute::unescaped_*`|`Attribute::unescape_*`
- [#329]: Also, that functions now borrow from the input instead of event / attribute

- [#416]: `BytesStart::to_borrowed` renamed to `BytesStart::borrow`, the same method
  added to all events

- [#421]: `decode_and_unescape*` methods now does one less allocation if unescaping is not required
- [#421]: Removed ability to deserialize byte arrays from serde deserializer.
  XML is not able to store binary data directly, you should always use some encoding
  scheme, for example, HEX or Base64
- [#421]: All unescaping functions now accepts and returns strings instead of byte slices

- [#423]: All escaping functions now accepts and returns strings instead of byte slices
- [#423]: Removed `BytesText::from_plain` because it internally did escaping of a byte array,
  but since now escaping works on strings. Use `BytesText::new` instead

- [#428]: Removed `BytesText::escaped()`. Use `.as_ref()` provided by `Deref` impl instead.
- [#428]: Removed `BytesText::from_escaped()`. Use constructors from strings instead,
  because writer anyway works in UTF-8 only
- [#428]: Removed `BytesCData::new()`. Use constructors from strings instead,
  because writer anyway works in UTF-8 only
- [#428]: Changed the event and `Attributes` constructors to accept a `&str` slices instead of `&[u8]` slices.
  Handmade events has always been assumed to store their content UTF-8 encoded.
- [#428]: Removed `Decoder` parameter from `_and_decode` versions of functions for
  `BytesText` (remember, that those functions was renamed in #415).

- [#431]: Changed event constructors:
  |Old names                                         |New name
  |--------------------------------------------------|----------------------------------------------
  |`BytesStart::owned_name(impl Into<Vec<u8>>)`      |`BytesStart::new(impl Into<Cow<str>>)`
  |`BytesStart::borrowed_name(&[u8])`                |_(as above)_
  |`BytesStart::owned(impl Into<Vec<u8>>, usize)`    |`BytesStart::from_content(impl Into<Cow<str>>, usize)`
  |`BytesStart::borrowed(&[u8], usize)`              |_(as above)_
  |`BytesEnd::owned(Vec<u8>)`                        |`BytesEnd::new(impl Into<Cow<str>>)`
  |`BytesEnd::borrowed(&[u8])`                       |_(as above)_
  |`BytesText::from_escaped(impl Into<Cow<[u8]>>)`   |`BytesText::from_escaped(impl Into<Cow<str>>)`
  |`BytesText::from_escaped_str(impl Into<Cow<str>>)`|_(as above)_
  |`BytesText::from_plain(&[u8])`                    |`BytesText::new(&str)`
  |`BytesText::from_plain_str(&str)`                 |_(as above)_
  |`BytesCData::new(impl Into<Cow<[u8]>>)`           |`BytesCData::new(impl Into<Cow<str>>)`
  |`BytesCData::from_str(&str)`                      |_(as above)_

- [#440]: Removed `Deserializer::from_slice` and `quick_xml::de::from_slice` methods because deserializing from a byte
  array cannot guarantee borrowing due to possible copying while decoding.

- [#455]: Removed `Reader::read_text_into` which is just a thin wrapper over match on `Event::Text`

- [#456]: Reader and writer stuff grouped under `reader` and `writer` modules.
  You still can use re-exported definitions from a crate root

- [#459]: Made the `Writer::write()` method non-public as writing random bytes to a document is not generally useful or desirable.
- [#459]: BOM bytes are no longer emitted as `Event::Text`. To write a BOM, use `Writer::write_bom()`.

- [#467]: Removed `Deserializer::new` because it cannot be used outside of the quick-xml crate

### New Tests

- [#9]: Added tests for incorrect nested tags in input
- [#387]: Added a bunch of tests for sequences deserialization
- [#393]: Added more tests for namespace resolver
- [#393]: Added tests for reserved names (started with "xml"i) -- see <https://www.w3.org/TR/xml-names11/#xmlReserved>
- [#363]: Add tests for `Reader::read_event_impl` to ensure that proper events generated for corresponding inputs
- [#407]: Improved benchmark suite to cover whole-document parsing, escaping and unescaping text
- [#418]: Parameterized macrobenchmarks and comparative benchmarks, added throughput measurements via criterion
- [#434]: Added more tests for serde deserializer
- [#443]: Now all documents in `/tests/documents` are checked out with LF eol in working copy (except sample_5_utf16bom.xml)

[#8]: https://github.com/Mingun/fast-xml/pull/8
[#9]: https://github.com/Mingun/fast-xml/pull/9
[#118]: https://github.com/tafia/quick-xml/issues/118
[#180]: https://github.com/tafia/quick-xml/issues/180
[#191]: https://github.com/tafia/quick-xml/issues/191
[#324]: https://github.com/tafia/quick-xml/issues/324
[#329]: https://github.com/tafia/quick-xml/issues/329
[#363]: https://github.com/tafia/quick-xml/issues/363
[#387]: https://github.com/tafia/quick-xml/pull/387
[#391]: https://github.com/tafia/quick-xml/pull/391
[#393]: https://github.com/tafia/quick-xml/pull/393
[#395]: https://github.com/tafia/quick-xml/pull/395
[#403]: https://github.com/tafia/quick-xml/pull/403
[#407]: https://github.com/tafia/quick-xml/pull/407
[#412]: https://github.com/tafia/quick-xml/pull/412
[#415]: https://github.com/tafia/quick-xml/pull/415
[#416]: https://github.com/tafia/quick-xml/pull/416
[#418]: https://github.com/tafia/quick-xml/pull/418
[#421]: https://github.com/tafia/quick-xml/pull/421
[#423]: https://github.com/tafia/quick-xml/pull/423
[#428]: https://github.com/tafia/quick-xml/pull/428
[#431]: https://github.com/tafia/quick-xml/pull/431
[#434]: https://github.com/tafia/quick-xml/pull/434
[#437]: https://github.com/tafia/quick-xml/pull/437
[#439]: https://github.com/tafia/quick-xml/pull/439
[#440]: https://github.com/tafia/quick-xml/pull/440
[#443]: https://github.com/tafia/quick-xml/pull/443
[#445]: https://github.com/tafia/quick-xml/pull/445
[#450]: https://github.com/tafia/quick-xml/pull/450
[#455]: https://github.com/tafia/quick-xml/pull/455
[#456]: https://github.com/tafia/quick-xml/pull/456
[#459]: https://github.com/tafia/quick-xml/pull/459
[#467]: https://github.com/tafia/quick-xml/pull/467

----------------------------------------------------------------------------------------------------

> Legend:
> - feat: A new feature
> - fix: A bug fix
> - docs: Documentation only changes
> - style: White-space, formatting, missing semi-colons, etc
> - refactor: A code change that neither fixes a bug nor adds a feature
> - perf: A code change that improves performance
> - test: Adding missing tests
> - chore: Changes to the build process or auxiliary tools/libraries/documentation


## 0.23.1 -- 2022-09-11

### Bug Fixes

- [#469]: Fix incorrect parsing of CDATA and comments when using buffered readers

[#469]: https://github.com/tafia/quick-xml/issues/469


## 0.23.0 -- 2022-05-08

- feat: add support for `i128` / `u128` in attributes or text/CDATA content
- test: add tests for malformed inputs for serde deserializer
- fix: allow to deserialize `unit`s from any data in attribute values and text nodes
- refactor: unify errors when EOF encountered during serde deserialization
- test: ensure that after deserializing all XML was consumed
- feat: add `Deserializer::from_str`, `Deserializer::from_slice` and `Deserializer::from_reader`
- refactor: deprecate `from_bytes` and `Deserializer::from_borrowing_reader` because
  they are fully equivalent to `from_slice` and `Deserializer::new`
- refactor: reduce number of unnecessary copies when deserialize numbers/booleans/identifiers
  from the attribute and element names and attribute values
- fix: allow to deserialize `unit`s from text and CDATA content.
  `DeError::InvalidUnit` variant is removed, because after fix it is no longer used
- fix: `ElementWriter`, introduced in [#274](https://github.com/tafia/quick-xml/pull/274)
  (0.23.0-alpha2) now available to end users
- fix: allow lowercase `<!doctype >` definition (used in HTML 5) when parse document from `&[u8]`
- test: add tests for consistence behavior of buffered and borrowed readers
- fix: produce consistent error positions in buffered and borrowed readers
- feat: `Error::UnexpectedBang` now provide the byte found
- refactor: unify code for buffered and borrowed readers
- fix: fix internal panic message when parse malformed XML
  ([#344](https://github.com/tafia/quick-xml/issues/344))
- test: add tests for trivial documents (empty / only comment / `<root>...</root>` -- one tag with content)
- fix: CDATA was not handled in many cases where it should
- fix: do not unescape CDATA content because it never escaped by design.
  CDATA event data now represented by its own `BytesCData` type
  ([quick-xml#311](https://github.com/tafia/quick-xml/issues/311))
- feat: add `Reader::get_ref()` and `Reader::get_mut()`, rename
  `Reader::into_underlying_reader()` to `Reader::into_inner()`
- refactor: now `Attributes::next()` returns a new type `AttrError` when attribute parsing failed
  ([#4](https://github.com/Mingun/fast-xml/pull/4))
- test: properly test all paths of attributes parsing ([#4](https://github.com/Mingun/fast-xml/pull/4))
- feat: attribute iterator now implements `FusedIterator` ([#4](https://github.com/Mingun/fast-xml/pull/4))
- fix: fixed many errors in attribute parsing using iterator, returned from `attributes()`
  or `html_attributes()` ([#4](https://github.com/Mingun/fast-xml/pull/4))

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
