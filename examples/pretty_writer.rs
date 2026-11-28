//! This example demonstrates the `PrettyWriter` for content-preserving pretty-printing.
//!
//! Unlike the streaming `Writer` which makes layout decisions on-the-fly, `PrettyWriter`
//! buffers events to decide formatting based on complete element content. This matches
//! the behavior of lxml/libxml2 pretty-printing.
//!
//! Key formatting rule: elements are indented only if they contain NO text children.
//! This preserves text content exactly while providing clean indentation for structure.

use quick_xml::events::{BytesComment, BytesEnd, BytesRef, BytesStart, BytesText, Event};
use quick_xml::writer::{PrettyWriter, Writer};

fn main() {
    println!("=== PrettyWriter Examples ===\n");

    // Example 1: Element-only content gets indented
    println!("1. Element-only content (indented):");
    example_element_only();
    println!();

    // Example 2: Mixed content stays inline to preserve text
    println!("2. Mixed content (inline to preserve text):");
    example_mixed_content();
    println!();

    // Example 3: Comment without text is indented
    println!("3. Comment-only element (indented):");
    example_comment_only();
    println!();

    // Example 4: Nested structure
    println!("4. Nested document:");
    example_nested();
    println!();

    // Example 5: Comparison with streaming Writer
    println!("5. Comparison: PrettyWriter vs streaming Writer:");
    comparison_streaming_vs_buffered();
    println!();

    // Example 6: Entity references are inline (text-like)
    println!("6. Entity references (inline like text):");
    example_entity_refs();
    println!();

    // Example 7: Configuration options
    println!("7. Configuration options:");
    example_config();
}

fn example_element_only() {
    let mut writer = PrettyWriter::new(Vec::new());

    writer
        .write_event(Event::Start(BytesStart::new("catalog")))
        .unwrap();
    writer
        .write_event(Event::Empty(
            BytesStart::new("book")
                .with_attributes(vec![("id", "1"), ("title", "Rust Programming")]),
        ))
        .unwrap();
    writer
        .write_event(Event::Empty(
            BytesStart::new("book").with_attributes(vec![("id", "2"), ("title", "XML Processing")]),
        ))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("catalog")))
        .unwrap();

    let result = writer.into_inner().unwrap();
    let output = std::str::from_utf8(&result).unwrap();
    println!("{}", output);
    println!("  → Elements with no text children are indented");
}

fn example_mixed_content() {
    let mut writer = PrettyWriter::new(Vec::new());

    writer
        .write_event(Event::Start(BytesStart::new("paragraph")))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new("This is ")))
        .unwrap();
    writer
        .write_event(Event::Start(BytesStart::new("strong")))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new("important")))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("strong")))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new(" text.")))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("paragraph")))
        .unwrap();

    let result = writer.into_inner().unwrap();
    let output = std::str::from_utf8(&result).unwrap();
    println!("{}", output);
    println!("  → Elements with text children stay inline (preserves exact text content)");
}

fn example_comment_only() {
    let mut writer = PrettyWriter::new(Vec::new());

    writer
        .write_event(Event::Start(BytesStart::new("config")))
        .unwrap();
    writer
        .write_event(Event::Comment(
            BytesComment::new("Database settings").unwrap(),
        ))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("config")))
        .unwrap();

    let result = writer.into_inner().unwrap();
    let output = std::str::from_utf8(&result).unwrap();
    println!("{}", output);
    println!("  → Comments are block-level, so element is indented");
}

fn example_nested() {
    let mut writer = PrettyWriter::new(Vec::new());

    // Root element
    writer
        .write_event(Event::Start(BytesStart::new("document")))
        .unwrap();

    // Section with mixed content (inline)
    writer
        .write_event(Event::Start(BytesStart::new("section")))
        .unwrap();
    writer
        .write_event(Event::Start(BytesStart::new("title")))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new("Introduction")))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("title")))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("section")))
        .unwrap();

    // Section with element-only content (indented)
    writer
        .write_event(Event::Start(BytesStart::new("section")))
        .unwrap();
    writer
        .write_event(Event::Empty(BytesStart::new("placeholder")))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("section")))
        .unwrap();

    writer
        .write_event(Event::End(BytesEnd::new("document")))
        .unwrap();

    let result = writer.into_inner().unwrap();
    let output = std::str::from_utf8(&result).unwrap();
    println!("{}", output);
    println!("  → Mixed: first section has text (inline), second is element-only (indented)");
}

fn comparison_streaming_vs_buffered() {
    let events = vec![
        Event::Start(BytesStart::new("root")),
        Event::Start(BytesStart::new("item")),
        Event::Comment(BytesComment::new("A comment").unwrap()),
        Event::Text(BytesText::new("Some text")),
        Event::End(BytesEnd::new("item")),
        Event::End(BytesEnd::new("root")),
    ];

    // Streaming Writer (best-effort indentation)
    let mut streaming = Writer::new_with_indent(Vec::new(), ' ', 2);
    for event in events.iter() {
        streaming.write_event(event.borrow()).unwrap();
    }
    let streaming_result = streaming.into_inner();

    println!("Streaming Writer output:");
    println!("{}", std::str::from_utf8(&streaming_result).unwrap());
    println!("  Problem: text content altered from \"Some text\" to \"\\n  Some text\\n\"");
    println!();

    // PrettyWriter (content-preserving)
    let mut pretty = PrettyWriter::new(Vec::new());
    for event in events {
        pretty.write_event(event).unwrap();
    }
    let pretty_result = pretty.into_inner().unwrap();

    println!("PrettyWriter output:");
    println!("{}", std::str::from_utf8(&pretty_result).unwrap());
    println!("  Solution: text preserved exactly as \"Some text\", structure indented");
}

fn example_entity_refs() {
    let mut writer = PrettyWriter::new(Vec::new());

    writer
        .write_event(Event::Start(BytesStart::new("message")))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new("Cost: 5")))
        .unwrap();
    writer
        .write_event(Event::GeneralRef(BytesRef::new("lt")))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new("10 ")))
        .unwrap();
    writer
        .write_event(Event::GeneralRef(BytesRef::new("amp")))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new(" available")))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("message")))
        .unwrap();

    let result = writer.into_inner().unwrap();
    let output = std::str::from_utf8(&result).unwrap();
    println!("{}", output);
    println!("  → Entity references (&name;) are treated like text, so element stays inline");
}

fn example_config() {
    let mut writer = PrettyWriter::new_with_indent(Vec::new(), '\t', 1);
    writer.config_mut().add_space_before_slash_in_empty_elements = true;

    writer
        .write_event(Event::Start(BytesStart::new("root")))
        .unwrap();
    writer
        .write_event(Event::Empty(BytesStart::new("item")))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("root")))
        .unwrap();

    let result = writer.into_inner().unwrap();
    let output = std::str::from_utf8(&result).unwrap();
    println!("{}", output);
    println!("  → Custom indent (tabs) and space before slash in empty elements");
}
