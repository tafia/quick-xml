//! Tests of deserialization of XML documents into various sequential types

use quick_xml::DeError;
use serde::Deserialize;

mod serde_helpers;
use serde_helpers::from_str;

/// Check that top-level sequences can be deserialized from the multi-root XML documents
mod top_level {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn simple() {
        from_str::<[(); 3]>("<root/><root>42</root><root>answer</root>").unwrap();

        let data: Vec<()> = from_str("<root/><root>42</root><root>answer</root>").unwrap();
        assert_eq!(data, vec![(), (), ()]);
    }

    /// Special case: empty sequence
    #[test]
    fn empty() {
        from_str::<[(); 0]>("").unwrap();

        let data: Vec<()> = from_str("").unwrap();
        assert_eq!(data, vec![]);
    }

    /// Special case: one-element sequence
    #[test]
    fn one_element() {
        from_str::<[(); 1]>("<root/>").unwrap();
        from_str::<[(); 1]>("<root>42</root>").unwrap();
        from_str::<[(); 1]>("text").unwrap();
        from_str::<[(); 1]>("<![CDATA[cdata]]>").unwrap();

        let data: Vec<()> = from_str("<root/>").unwrap();
        assert_eq!(data, vec![()]);

        let data: Vec<()> = from_str("<root>42</root>").unwrap();
        assert_eq!(data, vec![()]);

        let data: Vec<()> = from_str("text").unwrap();
        assert_eq!(data, vec![()]);

        let data: Vec<()> = from_str("<![CDATA[cdata]]>").unwrap();
        assert_eq!(data, vec![()]);
    }

    #[test]
    fn excess_attribute() {
        from_str::<[(); 3]>(r#"<root/><root excess="attribute">42</root><root>answer</root>"#)
            .unwrap();

        let data: Vec<()> =
            from_str(r#"<root/><root excess="attribute">42</root><root>answer</root>"#).unwrap();
        assert_eq!(data, vec![(), (), ()]);
    }

    #[test]
    fn mixed_content() {
        // Text and CDATA represents a one logical text item
        from_str::<[(); 2]>(
            r#"
            <element/>
            text
            <![CDATA[cdata]]>
            "#,
        )
        .unwrap();

        let data: Vec<()> = from_str(
            r#"
            <element/>
            text
            <![CDATA[cdata]]>
            "#,
        )
        .unwrap();
        // Text and CDATA represents a one logical text item
        assert_eq!(data, vec![(), ()]);
    }

    /// This test ensures that composition of deserializer building blocks plays well
    #[test]
    fn list_of_struct() {
        #[derive(Debug, PartialEq, Default, Deserialize)]
        #[serde(default)]
        struct Struct {
            #[serde(rename = "@attribute")]
            attribute: Option<String>,
            element: Option<String>,
        }

        let data: Vec<Struct> = from_str(
            r#"
            <struct/>
            <struct attribute="value"/>
            <struct>
                <element>value</element>
            </struct>
            <struct attribute="value">
                <element>value</element>
            </struct>
            "#,
        )
        .unwrap();
        assert_eq!(
            data,
            vec![
                Struct {
                    attribute: None,
                    element: None,
                },
                Struct {
                    attribute: Some("value".to_string()),
                    element: None,
                },
                Struct {
                    attribute: None,
                    element: Some("value".to_string()),
                },
                Struct {
                    attribute: Some("value".to_string()),
                    element: Some("value".to_string()),
                },
            ]
        );
    }

    /// Test for https://github.com/tafia/quick-xml/issues/500
    #[test]
    fn list_of_enum() {
        #[derive(Debug, PartialEq, Deserialize)]
        enum Enum {
            One,
            Two,
        }

        let data: Vec<Enum> = from_str(
            r#"
            <One/>
            <Two/>
            <One/>
            "#,
        )
        .unwrap();
        assert_eq!(data, vec![Enum::One, Enum::Two, Enum::One]);
    }
}

/// Tests where each sequence item have an identical name in an XML.
/// That explicitly means that `enum`s as list elements are not supported
/// in that case, because enum requires different tags.
///
/// (by `enums` we mean [externally tagged enums] is serde terminology)
///
/// [externally tagged enums]: https://serde.rs/enum-representations.html#externally-tagged
mod fixed_name {
    use super::*;

    /// This module contains tests where size of the list have a compile-time size
    mod fixed_size {
        use super::*;
        use pretty_assertions::assert_eq;

        #[derive(Debug, PartialEq, Deserialize)]
        struct List<T = ()> {
            item: [T; 3],
        }

        /// Simple case: count of elements matches expected size of sequence,
        /// each element has the same name. Successful deserialization expected
        #[test]
        fn simple() {
            from_str::<List>(
                r#"
                <root>
                    <item/>
                    <item/>
                    <item/>
                </root>
                "#,
            )
            .unwrap();
        }

        /// Special case: empty sequence
        #[test]
        #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
        fn empty() {
            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                item: [(); 0],
            }

            from_str::<List>(r#"<root></root>"#).unwrap();
            from_str::<List>(r#"<root/>"#).unwrap();
        }

        /// Special case: one-element sequence
        #[test]
        fn one_element() {
            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                item: [(); 1],
            }

            from_str::<List>(
                r#"
                <root>
                    <item/>
                </root>
                "#,
            )
            .unwrap();
        }

        /// Fever elements than expected size of sequence, each element has
        /// the same name. Failure expected
        #[test]
        fn fever_elements() {
            let data = from_str::<List>(
                r#"
                <root>
                    <item/>
                    <item/>
                </root>
                "#,
            );

            match data {
                Err(DeError::Custom(e)) => {
                    assert_eq!(e, "invalid length 2, expected an array of length 3")
                }
                e => panic!(
                    r#"Expected `Err(Custom("invalid length 2, expected an array of length 3"))`, but got `{:?}`"#,
                    e
                ),
            }
        }

        /// More elements than expected size of sequence, each element has
        /// the same name. Failure expected. If you wish to ignore excess
        /// elements, use the special type, that consume as much elements
        /// as possible, but ignores excess elements
        #[test]
        fn excess_elements() {
            let data = from_str::<List>(
                r#"
                <root>
                    <item/>
                    <item/>
                    <item/>
                    <item/>
                </root>
                "#,
            );

            match data {
                Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `item`"),
                e => panic!(
                    r#"Expected `Err(Custom("duplicate field `item`"))`, but got `{:?}`"#,
                    e
                ),
            }
        }

        /// Mixed content assumes, that some elements will have an internal
        /// name `$text` or `$value`, so, unless field named the same, it is expected
        /// to fail
        #[test]
        fn mixed_content() {
            let data = from_str::<List>(
                r#"
                <root>
                    <element/>
                    text
                    <![CDATA[cdata]]>
                </root>
                "#,
            );

            match data {
                Err(DeError::Custom(e)) => assert_eq!(e, "missing field `item`"),
                e => panic!(
                    r#"Expected `Err(Custom("missing field `item`"))`, but got `{:?}`"#,
                    e
                ),
            }
        }

        /// In those tests sequence should be deserialized from an XML
        /// with additional elements that is not defined in the struct.
        /// That fields should be skipped during deserialization
        mod unknown_items {
            use super::*;
            #[cfg(not(feature = "overlapped-lists"))]
            use pretty_assertions::assert_eq;

            #[test]
            fn before() {
                from_str::<List>(
                    r#"
                    <root>
                        <unknown/>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();
            }

            #[test]
            fn after() {
                from_str::<List>(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                        <unknown/>
                    </root>
                    "#,
                )
                .unwrap();
            }

            #[test]
            fn overlapped() {
                let data = from_str::<List>(
                    r#"
                    <root>
                        <item/>
                        <unknown/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                data.unwrap();

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            /// Test for https://github.com/tafia/quick-xml/issues/435
            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    outer: [List; 3],
                }

                let data = from_str::<Root>(
                    r#"
                    <root>
                        <outer><item/><item/><item/></outer>
                        <unknown/>
                        <outer><item/><item/><item/></outer>
                        <outer><item/><item/><item/></outer>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                data.unwrap();

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// In those tests non-sequential field is defined in the struct
        /// before sequential, so it will be deserialized before the list.
        /// That struct should be deserialized from an XML where these
        /// fields comes in an arbitrary order
        mod field_before_list {
            use super::*;
            #[cfg(not(feature = "overlapped-lists"))]
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct Root {
                node: (),
                item: [(); 3],
            }

            #[test]
            fn before() {
                from_str::<Root>(
                    r#"
                    <root>
                        <node/>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();
            }

            #[test]
            fn after() {
                from_str::<Root>(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                        <node/>
                    </root>
                    "#,
                )
                .unwrap();
            }

            #[test]
            fn overlapped() {
                let data = from_str::<Root>(
                    r#"
                    <root>
                        <item/>
                        <node/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                data.unwrap();

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            /// Test for https://github.com/tafia/quick-xml/issues/435
            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    node: (),
                    outer: [List; 3],
                }

                let data = from_str::<Root>(
                    r#"
                    <root>
                        <outer><item/><item/><item/></outer>
                        <node/>
                        <outer><item/><item/><item/></outer>
                        <outer><item/><item/><item/></outer>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                data.unwrap();

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// In those tests non-sequential field is defined in the struct
        /// after sequential, so it will be deserialized after the list.
        /// That struct should be deserialized from an XML where these
        /// fields comes in an arbitrary order
        mod field_after_list {
            use super::*;
            #[cfg(not(feature = "overlapped-lists"))]
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct Root {
                item: [(); 3],
                node: (),
            }

            #[test]
            fn before() {
                from_str::<Root>(
                    r#"
                    <root>
                        <node/>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();
            }

            #[test]
            fn after() {
                from_str::<Root>(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                        <node/>
                    </root>
                    "#,
                )
                .unwrap();
            }

            #[test]
            fn overlapped() {
                let data = from_str::<Root>(
                    r#"
                    <root>
                        <item/>
                        <node/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                data.unwrap();

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            /// Test for https://github.com/tafia/quick-xml/issues/435
            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    outer: [List; 3],
                    node: (),
                }

                let data = from_str::<Root>(
                    r#"
                    <root>
                        <outer><item/><item/><item/></outer>
                        <node/>
                        <outer><item/><item/><item/></outer>
                        <outer><item/><item/><item/></outer>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                data.unwrap();

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// In those tests two lists are deserialized simultaneously.
        /// Lists should be deserialized even when them overlaps
        mod two_lists {
            use super::*;
            #[cfg(not(feature = "overlapped-lists"))]
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct Pair {
                item: [(); 3],
                element: [(); 2],
            }

            #[test]
            fn splitted() {
                from_str::<Pair>(
                    r#"
                    <root>
                        <element/>
                        <element/>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();
            }

            #[test]
            fn overlapped() {
                let data = from_str::<Pair>(
                    r#"
                    <root>
                        <item/>
                        <element/>
                        <item/>
                        <element/>
                        <item/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                data.unwrap();

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            /// Test for https://github.com/tafia/quick-xml/issues/435
            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    outer: [List; 3],
                    element: [(); 2],
                }

                let data = from_str::<Pair>(
                    r#"
                    <root>
                        <outer><item/><item/><item/></outer>
                        <element/>
                        <outer><item/><item/><item/></outer>
                        <element/>
                        <outer><item/><item/><item/></outer>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                data.unwrap();

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// Deserialization of primitives slightly differs from deserialization
        /// of complex types, so need to check this separately
        #[test]
        fn primitives() {
            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                item: [usize; 3],
            }

            let data: List = from_str(
                r#"
                <root>
                    <item>41</item>
                    <item>42</item>
                    <item>43</item>
                </root>
                "#,
            )
            .unwrap();
            assert_eq!(data, List { item: [41, 42, 43] });

            from_str::<List>(
                r#"
                <root>
                    <item>41</item>
                    <item><item>42</item></item>
                    <item>43</item>
                </root>
                "#,
            )
            .unwrap_err();
        }

        /// This test ensures that composition of deserializer building blocks
        /// plays well
        #[test]
        fn list_of_struct() {
            #[derive(Debug, PartialEq, Default, Deserialize)]
            #[serde(default)]
            struct Struct {
                #[serde(rename = "@attribute")]
                attribute: Option<String>,
                element: Option<String>,
            }

            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                item: [Struct; 4],
            }

            let data: List = from_str(
                r#"
                <root>
                    <item/>
                    <item attribute="value"/>
                    <item>
                        <element>value</element>
                    </item>
                    <item attribute="value">
                        <element>value</element>
                    </item>
                </root>
                "#,
            )
            .unwrap();
            assert_eq!(
                data,
                List {
                    item: [
                        Struct {
                            attribute: None,
                            element: None,
                        },
                        Struct {
                            attribute: Some("value".to_string()),
                            element: None,
                        },
                        Struct {
                            attribute: None,
                            element: Some("value".to_string()),
                        },
                        Struct {
                            attribute: Some("value".to_string()),
                            element: Some("value".to_string()),
                        },
                    ],
                }
            );
        }

        #[test]
        fn list_of_list() {
            let data: List<Vec<String>> = from_str(
                r#"
                <root>
                    <item>first item</item>
                    <item>second item</item>
                    <item>third item</item>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(
                data,
                List {
                    item: [
                        vec!["first".to_string(), "item".to_string()],
                        vec!["second".to_string(), "item".to_string()],
                        vec!["third".to_string(), "item".to_string()],
                    ],
                }
            );
        }

        /// Checks that sequences represented by elements can contain sequences,
        /// represented by [`xs:list`s](https://www.w3schools.com/xml/el_list.asp)
        mod xs_list {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, Deserialize, PartialEq)]
            struct List {
                /// Outer list mapped to elements, inner -- to `xs:list`.
                ///
                /// `#[serde(default)]` is not required, because correct
                /// XML will always contains at least 1 element.
                item: [Vec<String>; 1],
            }

            /// Special case: zero elements
            #[test]
            fn zero() {
                #[derive(Debug, Deserialize, PartialEq)]
                struct List {
                    /// Outer list mapped to elements, inner -- to `xs:list`.
                    ///
                    /// `#[serde(default)]` is required to correctly deserialize
                    /// empty sequence, because without elements the field
                    /// also is missing and derived `Deserialize` implementation
                    /// would complain about that unless field is marked as
                    /// `default`.
                    #[serde(default)]
                    item: [Vec<String>; 0],
                }

                let data: List = from_str(
                    r#"
                    <root>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(data, List { item: [] });
            }

            /// Special case: one element
            #[test]
            fn one() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item>first list</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: [vec!["first".to_string(), "list".to_string()]]
                    }
                );
            }

            /// Special case: empty `xs:list`
            #[test]
            fn empty() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(data, List { item: [vec![]] });
            }

            /// Special case: outer list is always mapped to an elements sequence,
            /// not to an `xs:list`
            #[test]
            fn element() {
                #[derive(Debug, Deserialize, PartialEq)]
                struct List {
                    /// Outer list mapped to elements, inner -- to `xs:list`.
                    ///
                    /// `#[serde(default)]` is not required, because correct
                    /// XML will always contains at least 1 element.
                    item: [String; 1],
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item>first item</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: ["first item".to_string()]
                    }
                );
            }

            /// This tests demonstrates, that for `$value` field (`list`) actual
            /// name of XML element (`item`) does not matter. That allows list
            /// item to be an enum, where tag name determines enum variant
            #[test]
            fn many() {
                #[derive(Debug, Deserialize, PartialEq)]
                struct List {
                    /// Outer list mapped to elements, inner -- to `xs:list`.
                    ///
                    /// `#[serde(default)]` is not required, because correct
                    /// XML will always contains at least 1 element.
                    item: [Vec<String>; 2],
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item>first list</item>
                        <item>second list</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: [
                            vec!["first".to_string(), "list".to_string()],
                            vec!["second".to_string(), "list".to_string()],
                        ]
                    }
                );
            }
        }
    }

    /// This module contains tests where size of the list have an unspecified size
    mod variable_size {
        use super::*;
        use pretty_assertions::assert_eq;

        #[derive(Debug, PartialEq, Deserialize)]
        struct List {
            item: Vec<()>,
        }

        /// Simple case: count of elements matches expected size of sequence,
        /// each element has the same name. Successful deserialization expected
        #[test]
        fn simple() {
            let data: List = from_str(
                r#"
                <root>
                    <item/>
                    <item/>
                    <item/>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(
                data,
                List {
                    item: vec![(), (), ()],
                }
            );
        }

        /// Special case: empty sequence
        #[test]
        #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
        fn empty() {
            let data: List = from_str(r#"<root></root>"#).unwrap();
            assert_eq!(data, List { item: vec![] });

            let data: List = from_str(r#"<root/>"#).unwrap();
            assert_eq!(data, List { item: vec![] });
        }

        /// Special case: one-element sequence
        #[test]
        fn one_element() {
            let data: List = from_str(
                r#"
                <root>
                    <item/>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(data, List { item: vec![()] });
        }

        /// Mixed content assumes, that some elements will have an internal
        /// name `$text` or `$value`, so, unless field named the same, it is expected
        /// to fail
        #[test]
        fn mixed_content() {
            let data = from_str::<List>(
                r#"
                <root>
                    <element/>
                    text
                    <![CDATA[cdata]]>
                </root>
                "#,
            );

            match data {
                Err(DeError::Custom(e)) => assert_eq!(e, "missing field `item`"),
                e => panic!(
                    r#"Expected `Err(Custom("missing field `item`"))`, but got `{:?}`"#,
                    e
                ),
            }
        }

        /// In those tests sequence should be deserialized from the XML
        /// with additional elements that is not defined in the struct.
        /// That fields should be skipped during deserialization
        mod unknown_items {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn before() {
                let data: List = from_str(
                    r#"
                    <root>
                        <unknown/>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![(), (), ()],
                    }
                );
            }

            #[test]
            fn after() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                        <unknown/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![(), (), ()],
                    }
                );
            }

            #[test]
            fn overlapped() {
                let data = from_str::<List>(
                    r#"
                    <root>
                        <item/>
                        <unknown/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    List {
                        item: vec![(), (), ()],
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `item`"),
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `item`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            /// Test for https://github.com/tafia/quick-xml/issues/435
            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    outer: Vec<List>,
                }

                let data = from_str::<Root>(
                    r#"
                    <root>
                        <outer><item/></outer>
                        <unknown/>
                        <outer><item/></outer>
                        <outer><item/></outer>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        outer: vec![
                            List { item: vec![()] },
                            List { item: vec![()] },
                            List { item: vec![()] },
                        ],
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `outer`"),
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `outer`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// In those tests non-sequential field is defined in the struct
        /// before sequential, so it will be deserialized before the list.
        /// That struct should be deserialized from the XML where these
        /// fields comes in an arbitrary order
        mod field_before_list {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Default, Deserialize)]
            struct Root {
                node: (),
                item: Vec<()>,
            }

            #[test]
            fn before() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <node/>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        node: (),
                        item: vec![(), (), ()],
                    }
                );
            }

            #[test]
            fn after() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                        <node/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        node: (),
                        item: vec![(), (), ()],
                    }
                );
            }

            #[test]
            fn overlapped() {
                let data = from_str::<Root>(
                    r#"
                    <root>
                        <item/>
                        <node/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        node: (),
                        item: vec![(), (), ()],
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "duplicate field `item`")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `item`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            /// Test for https://github.com/tafia/quick-xml/issues/435
            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    node: (),
                    outer: Vec<List>,
                }

                let data = from_str::<Root>(
                    r#"
                    <root>
                        <outer><item/></outer>
                        <node/>
                        <outer><item/></outer>
                        <outer><item/></outer>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        node: (),
                        outer: vec![
                            List { item: vec![()] },
                            List { item: vec![()] },
                            List { item: vec![()] },
                        ],
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `outer`"),
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `outer`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// In those tests non-sequential field is defined in the struct
        /// after sequential, so it will be deserialized after the list.
        /// That struct should be deserialized from the XML where these
        /// fields comes in an arbitrary order
        mod field_after_list {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Default, Deserialize)]
            struct Root {
                item: Vec<()>,
                node: (),
            }

            #[test]
            fn before() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <node/>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        item: vec![(), (), ()],
                        node: (),
                    }
                );
            }

            #[test]
            fn after() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                        <node/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        item: vec![(), (), ()],
                        node: (),
                    }
                );
            }

            #[test]
            fn overlapped() {
                let data = from_str::<Root>(
                    r#"
                    <root>
                        <item/>
                        <node/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        item: vec![(), (), ()],
                        node: (),
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "duplicate field `item`")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `item`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            /// Test for https://github.com/tafia/quick-xml/issues/435
            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    outer: Vec<List>,
                    node: (),
                }

                let data = from_str::<Root>(
                    r#"
                    <root>
                        <outer><item/></outer>
                        <node/>
                        <outer><item/></outer>
                        <outer><item/></outer>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        outer: vec![
                            List { item: vec![()] },
                            List { item: vec![()] },
                            List { item: vec![()] },
                        ],
                        node: (),
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `outer`"),
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `outer`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// In those tests two lists are deserialized simultaneously.
        /// Lists should be deserialized even when them overlaps
        mod two_lists {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct Pair {
                item: Vec<()>,
                element: Vec<()>,
            }

            #[test]
            fn splitted() {
                let data: Pair = from_str(
                    r#"
                    <root>
                        <element/>
                        <element/>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Pair {
                        item: vec![(), (), ()],
                        element: vec![(), ()],
                    }
                );
            }

            #[test]
            fn overlapped() {
                let data = from_str::<Pair>(
                    r#"
                    <root>
                        <item/>
                        <element/>
                        <item/>
                        <element/>
                        <item/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Pair {
                        item: vec![(), (), ()],
                        element: vec![(), ()],
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `item`"),
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `item`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    outer: Vec<List>,
                    element: Vec<()>,
                }

                let data = from_str::<Pair>(
                    r#"
                    <root>
                        <outer><item/></outer>
                        <element/>
                        <outer><item/></outer>
                        <outer><item/></outer>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Pair {
                        outer: vec![
                            List { item: vec![()] },
                            List { item: vec![()] },
                            List { item: vec![()] },
                        ],
                        element: vec![()],
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `outer`"),
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `outer`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// Deserialization of primitives slightly differs from deserialization
        /// of complex types, so need to check this separately
        #[test]
        fn primitives() {
            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                item: Vec<usize>,
            }

            let data: List = from_str(
                r#"
                <root>
                    <item>41</item>
                    <item>42</item>
                    <item>43</item>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(
                data,
                List {
                    item: vec![41, 42, 43],
                }
            );

            from_str::<List>(
                r#"
                <root>
                    <item>41</item>
                    <item><item>42</item></item>
                    <item>43</item>
                </root>
                "#,
            )
            .unwrap_err();
        }

        /// This test ensures that composition of deserializer building blocks
        /// plays well
        #[test]
        fn list_of_struct() {
            #[derive(Debug, PartialEq, Default, Deserialize)]
            #[serde(default)]
            struct Struct {
                #[serde(rename = "@attribute")]
                attribute: Option<String>,
                element: Option<String>,
            }

            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                item: Vec<Struct>,
            }

            let data: List = from_str(
                r#"
                <root>
                    <item/>
                    <item attribute="value"/>
                    <item>
                        <element>value</element>
                    </item>
                    <item attribute="value">
                        <element>value</element>
                    </item>
                </root>
                "#,
            )
            .unwrap();
            assert_eq!(
                data,
                List {
                    item: vec![
                        Struct {
                            attribute: None,
                            element: None,
                        },
                        Struct {
                            attribute: Some("value".to_string()),
                            element: None,
                        },
                        Struct {
                            attribute: None,
                            element: Some("value".to_string()),
                        },
                        Struct {
                            attribute: Some("value".to_string()),
                            element: Some("value".to_string()),
                        },
                    ],
                }
            );
        }

        /// Checks that sequences represented by elements can contain sequences,
        /// represented by `xs:list`s
        mod xs_list {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, Deserialize, PartialEq)]
            struct List {
                /// Outer list mapped to elements, inner -- to `xs:list`.
                ///
                /// `#[serde(default)]` is required to correctly deserialize
                /// empty sequence, because without elements the field
                /// also is missing and derived `Deserialize` implementation
                /// would complain about that unless field is marked as
                /// `default`.
                #[serde(default)]
                item: Vec<Vec<String>>,
            }

            /// Special case: zero elements
            #[test]
            fn zero() {
                let data: List = from_str(
                    r#"
                    <root>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(data, List { item: vec![] });
            }

            /// Special case: one element
            #[test]
            fn one() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item>first list</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![vec!["first".to_string(), "list".to_string()]]
                    }
                );
            }

            /// Special case: empty `xs:list`
            #[test]
            fn empty() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(data, List { item: vec![vec![]] });
            }

            /// Special case: outer list is always mapped to an elements sequence,
            /// not to an `xs:list`
            #[test]
            fn element() {
                #[derive(Debug, Deserialize, PartialEq)]
                struct List {
                    /// List mapped to elements, inner -- to `xs:list`.
                    ///
                    /// `#[serde(default)]` is not required, because correct
                    /// XML will always contains at least 1 element.
                    item: Vec<String>,
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item>first item</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec!["first item".to_string()]
                    }
                );
            }

            /// This tests demonstrates, that for `$value` field (`list`) actual
            /// name of XML element (`item`) does not matter. That allows list
            /// item to be an enum, where tag name determines enum variant
            #[test]
            fn many() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item>first list</item>
                        <item>second list</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![
                            vec!["first".to_string(), "list".to_string()],
                            vec!["second".to_string(), "list".to_string()],
                        ]
                    }
                );
            }
        }
    }
}

/// Check that sequences inside element can be deserialized.
/// In terms of serde this is a sequence flatten into the struct:
///
/// ```ignore
/// struct Root {
///   #[serde(flatten)]
///   items: Vec<T>,
/// }
/// ```
/// except that fact that this is not supported nowadays
/// (https://github.com/serde-rs/serde/issues/1905)
///
/// Because this is very frequently used pattern in the XML, quick-xml
/// have a workaround for this. If a field will have a special name `$value`
/// then any `xs:element`s in the `xs:sequence` / `xs:all`, except that
/// which name matches the struct name, will be associated with this field:
///
/// ```ignore
/// struct Root {
///   field: U,
///   #[serde(rename = "$value")]
///   items: Vec<Enum>,
/// }
/// ```
/// In this example `<field>` tag will be associated with a `field` field,
/// but all other tags will be associated with an `items` field. Disadvantages
/// of this approach that you can have only one field, but usually you don't
/// want more
mod variable_name {
    use super::*;
    use serde::de::{Deserializer, EnumAccess, VariantAccess, Visitor};
    use std::fmt::{self, Formatter};

    // NOTE: Derive could be possible once https://github.com/serde-rs/serde/issues/2126 is resolved
    macro_rules! impl_deserialize_choice {
        ($name:ident : $(($field:ident, $field_name:literal)),*) => {
            impl<'de> Deserialize<'de> for $name {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    #[derive(Deserialize)]
                    #[serde(field_identifier)]
                    #[serde(rename_all = "kebab-case")]
                    enum Tag {
                        $($field,)*
                        Other(String),
                    }

                    struct EnumVisitor;
                    impl<'de> Visitor<'de> for EnumVisitor {
                        type Value = $name;

                        fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                            f.write_str("enum ")?;
                            f.write_str(stringify!($name))
                        }

                        fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
                        where
                            A: EnumAccess<'de>,
                        {
                            match data.variant()? {
                                $(
                                    (Tag::$field, variant) => variant.unit_variant().map(|_| $name::$field),
                                )*
                                (Tag::Other(t), v) => v.unit_variant().map(|_| $name::Other(t)),
                            }
                        }
                    }

                    const VARIANTS: &'static [&'static str] = &[
                        $($field_name,)*
                        "<any other tag>"
                    ];
                    deserializer.deserialize_enum(stringify!($name), VARIANTS, EnumVisitor)
                }
            }
        };
    }

    /// Type that can be deserialized from `<one>`, `<two>`, or any other element
    #[derive(Debug, PartialEq)]
    enum Choice {
        One,
        Two,
        /// Any other tag name except `One` or `Two`, name of tag stored inside variant
        Other(String),
    }
    impl_deserialize_choice!(Choice: (One, "one"), (Two, "two"));

    /// Type that can be deserialized from `<first>`, `<second>`, or any other element
    #[derive(Debug, PartialEq)]
    enum Choice2 {
        First,
        Second,
        /// Any other tag name except `First` or `Second`, name of tag stored inside variant
        Other(String),
    }
    impl_deserialize_choice!(Choice2: (First, "first"), (Second, "second"));

    /// Type that can be deserialized from `<one>`, `<two>`, or any other element.
    /// Used for `primitives` tests
    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum Choice3 {
        One(usize),
        Two(String),
        #[serde(other)]
        Other,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum Choice4 {
        One {
            inner: [(); 1],
        },
        Two {
            inner: [(); 1],
        },
        #[serde(other)]
        Other,
    }

    /// This module contains tests where size of the list have a compile-time size
    mod fixed_size {
        use super::*;
        use pretty_assertions::assert_eq;

        #[derive(Debug, PartialEq, Deserialize)]
        struct List<T = Choice> {
            #[serde(rename = "$value")]
            item: [T; 3],
        }

        /// Simple case: count of elements matches expected size of sequence,
        /// each element has the same name. Successful deserialization expected
        #[test]
        fn simple() {
            let data: List = from_str(
                r#"
                <root>
                    <one/>
                    <two/>
                    <three/>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(
                data,
                List {
                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                }
            );
        }

        /// Special case: empty sequence
        #[test]
        #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
        fn empty() {
            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                #[serde(rename = "$value")]
                item: [Choice; 0],
            }

            from_str::<List>(r#"<root></root>"#).unwrap();
            from_str::<List>(r#"<root/>"#).unwrap();
        }

        /// Special case: one-element sequence
        #[test]
        fn one_element() {
            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                #[serde(rename = "$value")]
                item: [Choice; 1],
            }

            let data: List = from_str(
                r#"
                <root>
                    <one/>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(
                data,
                List {
                    item: [Choice::One],
                }
            );
        }

        /// Fever elements than expected size of sequence, each element has
        /// the same name. Failure expected
        #[test]
        fn fever_elements() {
            from_str::<List>(
                r#"
                <root>
                    <one/>
                    <two/>
                </root>
                "#,
            )
            .unwrap_err();
        }

        /// More elements than expected size of sequence, each element has
        /// the same name. Failure expected. If you wish to ignore excess
        /// elements, use the special type, that consume as much elements
        /// as possible, but ignores excess elements
        #[test]
        fn excess_elements() {
            from_str::<List>(
                r#"
                <root>
                    <one/>
                    <two/>
                    <three/>
                    <four/>
                </root>
                "#,
            )
            .unwrap_err();
        }

        #[test]
        fn mixed_content() {
            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                /// Text and CDATA represents a one logical text item
                #[serde(rename = "$value")]
                item: [(); 2],
            }

            from_str::<List>(
                r#"
                <root>
                    <element/>
                    text
                    <![CDATA[cdata]]>
                </root>
                "#,
            )
            .unwrap();
        }

        // There cannot be unknown items, because any tag name is accepted

        /// In those tests non-sequential field is defined in the struct
        /// before sequential, so it will be deserialized before the list.
        /// That struct should be deserialized from the XML where these
        /// fields comes in an arbitrary order
        mod field_before_list {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct Root {
                node: (),
                #[serde(rename = "$value")]
                item: [Choice; 3],
            }

            #[test]
            fn before() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <node/>
                        <one/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        node: (),
                        item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                    }
                );
            }

            #[test]
            fn after() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <one/>
                        <two/>
                        <three/>
                        <node/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        node: (),
                        item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                    }
                );
            }

            #[test]
            fn overlapped() {
                let data = from_str::<Root>(
                    r#"
                    <root>
                        <one/>
                        <node/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        node: (),
                        item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            /// Test for https://github.com/tafia/quick-xml/issues/435
            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    node: (),
                    #[serde(rename = "$value")]
                    item: [Choice4; 3],
                }

                let data = from_str::<Root>(
                    r#"
                    <root>
                        <one><inner/></one>
                        <node/>
                        <two><inner/></two>
                        <three><inner/></three>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        node: (),
                        item: [
                            Choice4::One { inner: [()] },
                            Choice4::Two { inner: [()] },
                            Choice4::Other,
                        ],
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// In those tests non-sequential field is defined in the struct
        /// after sequential, so it will be deserialized after the list.
        /// That struct should be deserialized from the XML where these
        /// fields comes in an arbitrary order
        mod field_after_list {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct Root {
                #[serde(rename = "$value")]
                item: [Choice; 3],
                node: (),
            }

            #[test]
            fn before() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <node/>
                        <one/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                        node: (),
                    }
                );
            }

            #[test]
            fn after() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <one/>
                        <two/>
                        <three/>
                        <node/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                        node: (),
                    }
                );
            }

            #[test]
            fn overlapped() {
                let data = from_str::<Root>(
                    r#"
                    <root>
                        <one/>
                        <node/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                        node: (),
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            /// Test for https://github.com/tafia/quick-xml/issues/435
            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    #[serde(rename = "$value")]
                    item: [Choice4; 3],
                    node: (),
                }

                let data = from_str::<Root>(
                    r#"
                    <root>
                        <one><inner/></one>
                        <node/>
                        <two><inner/></two>
                        <three><inner/></three>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        item: [
                            Choice4::One { inner: [()] },
                            Choice4::Two { inner: [()] },
                            Choice4::Other,
                        ],
                        node: (),
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 1, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// In those tests two lists are deserialized simultaneously.
        /// Lists should be deserialized even when them overlaps
        mod two_lists {
            use super::*;

            /// A field with a variable-name items defined before a field with a fixed-name
            /// items
            mod choice_and_fixed {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    #[serde(rename = "$value")]
                    item: [Choice; 3],
                    element: [(); 2],
                }

                /// A list with fixed-name elements located before a list with variable-name
                /// elements in an XML
                #[test]
                fn fixed_before() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <element/>
                            <element/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            element: [(), ()],
                        }
                    );
                }

                /// A list with fixed-name elements located after a list with variable-name
                /// elements in an XML
                #[test]
                fn fixed_after() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <element/>
                            <element/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            element: [(), ()],
                        }
                    );
                }

                mod overlapped {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        #[serde(rename = "$value")]
                        item: [Choice4; 3],
                        element: [(); 2],
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a fixed-name one
                    #[test]
                    fn fixed_before() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <element/>
                                <one/>
                                <two/>
                                <element/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 2")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("invalid length 1, expected an array of length 2"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a variable-name one
                    #[test]
                    fn fixed_after() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <element/>
                                <two/>
                                <three/>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 3")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// Test for https://github.com/tafia/quick-xml/issues/435
                    #[test]
                    fn with_nested_list_fixed_before() {
                        let data = from_str::<Root>(
                            r#"
                            <root>
                                <element/>
                                <one><inner/></one>
                                <two><inner/></two>
                                <element/>
                                <three><inner/></three>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Root {
                                item: [
                                    Choice4::One { inner: [()] },
                                    Choice4::Two { inner: [()] },
                                    Choice4::Other,
                                ],
                                element: [(); 2],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 2")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("invalid length 1, expected an array of length 2"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// Test for https://github.com/tafia/quick-xml/issues/435
                    #[test]
                    fn with_nested_list_fixed_after() {
                        let data = from_str::<Root>(
                            r#"
                            <root>
                                <one><inner/></one>
                                <element/>
                                <two><inner/></two>
                                <three><inner/></three>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Root {
                                item: [
                                    Choice4::One { inner: [()] },
                                    Choice4::Two { inner: [()] },
                                    Choice4::Other,
                                ],
                                element: [(); 2],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 3")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }
                }
            }

            /// A field with a variable-name items defined after a field with a fixed-name
            /// items
            mod fixed_and_choice {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    element: [(); 2],
                    #[serde(rename = "$value")]
                    item: [Choice; 3],
                }

                /// A list with fixed-name elements located before a list with variable-name
                /// elements in an XML
                #[test]
                fn fixed_before() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <element/>
                            <element/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            element: [(), ()],
                        }
                    );
                }

                /// A list with fixed-name elements located after a list with variable-name
                /// elements in an XML
                #[test]
                fn fixed_after() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <element/>
                            <element/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            element: [(), ()],
                        }
                    );
                }

                mod overlapped {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        element: [(); 2],
                        #[serde(rename = "$value")]
                        item: [Choice4; 3],
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a fixed-name one
                    #[test]
                    fn fixed_before() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <element/>
                                <one/>
                                <two/>
                                <element/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 2")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("invalid length 1, expected an array of length 2"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a variable-name one
                    #[test]
                    fn fixed_after() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <element/>
                                <two/>
                                <three/>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 3")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// Test for https://github.com/tafia/quick-xml/issues/435
                    #[test]
                    fn with_nested_list_fixed_before() {
                        let data = from_str::<Root>(
                            r#"
                            <root>
                                <element/>
                                <one><inner/></one>
                                <two><inner/></two>
                                <element/>
                                <three><inner/></three>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Root {
                                element: [(); 2],
                                item: [
                                    Choice4::One { inner: [()] },
                                    Choice4::Two { inner: [()] },
                                    Choice4::Other,
                                ],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 2")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("invalid length 1, expected an array of length 2"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// Test for https://github.com/tafia/quick-xml/issues/435
                    #[test]
                    fn with_nested_list_fixed_after() {
                        let data = from_str::<Root>(
                            r#"
                            <root>
                                <one><inner/></one>
                                <element/>
                                <two><inner/></two>
                                <three><inner/></three>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Root {
                                element: [(); 2],
                                item: [
                                    Choice4::One { inner: [()] },
                                    Choice4::Two { inner: [()] },
                                    Choice4::Other,
                                ],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 3")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }
                }
            }

            /// Tests are ignored, but exists to show a problem.
            /// May be it will be solved in the future
            mod choice_and_choice {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    #[serde(rename = "$value")]
                    item: [Choice; 3],
                    // Actually, we cannot rename both fields to `$value`, which is now
                    // required to indicate, that field accepts elements with any name
                    #[serde(rename = "$value")]
                    element: [Choice2; 2],
                }

                #[test]
                #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                fn splitted() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <first/>
                            <second/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            element: [Choice2::First, Choice2::Second],
                        }
                    );
                }

                #[test]
                #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                fn overlapped() {
                    let data = from_str::<Pair>(
                        r#"
                        <root>
                            <one/>
                            <first/>
                            <two/>
                            <second/>
                            <three/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Pair {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            element: [Choice2::First, Choice2::Second],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                            e
                        ),
                    }
                }
            }
        }

        /// Deserialization of primitives slightly differs from deserialization
        /// of complex types, so need to check this separately
        #[test]
        fn primitives() {
            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                #[serde(rename = "$value")]
                item: [Choice3; 3],
            }

            let data: List = from_str(
                r#"
                <root>
                    <one>41</one>
                    <two>42</two>
                    <three>43</three>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(
                data,
                List {
                    item: [
                        Choice3::One(41),
                        Choice3::Two("42".to_string()),
                        Choice3::Other,
                    ],
                }
            );

            from_str::<List>(
                r#"
                <root>
                    <one>41</one>
                    <two><item>42</item></two>
                    <three>43</three>
                </root>
                "#,
            )
            .unwrap_err();
        }

        /// Test for https://github.com/tafia/quick-xml/issues/567
        #[test]
        fn list_of_enum() {
            #[derive(Debug, PartialEq, Deserialize)]
            enum Enum {
                Variant(Vec<String>),
            }

            let data: List<Enum> = from_str(
                r#"
                <root>
                    <Variant>first item</Variant>
                    <Variant>second item</Variant>
                    <Variant>third item</Variant>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(
                data,
                List {
                    item: [
                        Enum::Variant(vec!["first".to_string(), "item".to_string()]),
                        Enum::Variant(vec!["second".to_string(), "item".to_string()]),
                        Enum::Variant(vec!["third".to_string(), "item".to_string()]),
                    ],
                }
            );
        }

        /// Checks that sequences represented by elements can contain sequences,
        /// represented by `xs:list`s
        mod xs_list {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, Deserialize, PartialEq)]
            struct List {
                /// Outer list mapped to elements, inner -- to `xs:list`.
                ///
                /// `#[serde(default)]` is not required, because correct
                /// XML will always contains at least 1 element.
                #[serde(rename = "$value")]
                element: [Vec<String>; 1],
            }

            /// Special case: zero elements
            #[test]
            fn zero() {
                #[derive(Debug, Deserialize, PartialEq)]
                struct List {
                    /// Outer list mapped to elements, inner -- to `xs:list`.
                    ///
                    /// `#[serde(default)]` is required to correctly deserialize
                    /// empty sequence, because without elements the field
                    /// also is missing and derived `Deserialize` implementation
                    /// would complain about that unless field is marked as
                    /// `default`.
                    #[serde(default)]
                    #[serde(rename = "$value")]
                    element: [Vec<String>; 0],
                }

                let data: List = from_str(
                    r#"
                    <root>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(data, List { element: [] });
            }

            /// Special case: one element
            #[test]
            fn one() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item>first list</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        element: [vec!["first".to_string(), "list".to_string()]]
                    }
                );
            }

            /// Special case: empty `xs:list`
            #[test]
            fn empty() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(data, List { element: [vec![]] });
            }

            /// Special case: outer list is always mapped to an elements sequence,
            /// not to an `xs:list`
            #[test]
            fn element() {
                #[derive(Debug, Deserialize, PartialEq)]
                struct List {
                    /// List mapped to elements, String -- to `xs:list`.
                    ///
                    /// `#[serde(default)]` is not required, because correct
                    /// XML will always contains at least 1 element.
                    #[serde(rename = "$value")]
                    element: [String; 1],
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item>first item</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        element: ["first item".to_string()]
                    }
                );
            }

            #[test]
            fn many() {
                #[derive(Debug, Deserialize, PartialEq)]
                struct List {
                    /// Outer list mapped to elements, inner -- to `xs:list`
                    #[serde(rename = "$value")]
                    element: [Vec<String>; 2],
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item>first list</item>
                        <item>second list</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        element: [
                            vec!["first".to_string(), "list".to_string()],
                            vec!["second".to_string(), "list".to_string()],
                        ]
                    }
                );
            }
        }
    }

    /// This module contains tests where size of the list have an unspecified size
    mod variable_size {
        use super::*;
        use pretty_assertions::assert_eq;

        #[derive(Debug, PartialEq, Deserialize)]
        struct List {
            #[serde(rename = "$value")]
            item: Vec<Choice>,
        }

        /// Simple case: count of elements matches expected size of sequence,
        /// each element has the same name. Successful deserialization expected
        #[test]
        fn simple() {
            let data: List = from_str(
                r#"
                <root>
                    <one/>
                    <two/>
                    <three/>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(
                data,
                List {
                    item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                }
            );
        }

        /// Special case: empty sequence
        #[test]
        #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
        fn empty() {
            let data = from_str::<List>(r#"<root></root>"#).unwrap();
            assert_eq!(data, List { item: vec![] });

            let data = from_str::<List>(r#"<root/>"#).unwrap();
            assert_eq!(data, List { item: vec![] });
        }

        /// Special case: one-element sequence
        #[test]
        fn one_element() {
            let data: List = from_str(
                r#"
                <root>
                    <one/>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(
                data,
                List {
                    item: vec![Choice::One],
                }
            );
        }

        #[test]
        fn mixed_content() {
            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                #[serde(rename = "$value")]
                item: Vec<()>,
            }

            let data: List = from_str(
                r#"
                <root>
                    <element/>
                    text
                    <![CDATA[cdata]]>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(
                data,
                List {
                    // Text and CDATA represents a one logical text item
                    item: vec![(), ()],
                }
            );
        }

        // There cannot be unknown items, because any tag name is accepted

        /// In those tests non-sequential field is defined in the struct
        /// before sequential, so it will be deserialized before the list.
        /// That struct should be deserialized from the XML where these
        /// fields comes in an arbitrary order
        mod field_before_list {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct Root {
                node: (),
                #[serde(rename = "$value")]
                item: Vec<Choice>,
            }

            #[test]
            fn before() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <node/>
                        <one/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        node: (),
                        item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                    }
                );
            }

            #[test]
            fn after() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <one/>
                        <two/>
                        <three/>
                        <node/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        node: (),
                        item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                    }
                );
            }

            #[test]
            fn overlapped() {
                let data = from_str::<Root>(
                    r#"
                    <root>
                        <one/>
                        <node/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        node: (),
                        item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `$value`"),
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `$value`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            /// Test for https://github.com/tafia/quick-xml/issues/435
            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    node: (),
                    #[serde(rename = "$value")]
                    item: Vec<Choice4>,
                }

                let data = from_str::<Root>(
                    r#"
                    <root>
                        <one><inner/></one>
                        <node/>
                        <two><inner/></two>
                        <three><inner/></three>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        node: (),
                        item: vec![
                            Choice4::One { inner: [()] },
                            Choice4::Two { inner: [()] },
                            Choice4::Other,
                        ],
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "duplicate field `$value`")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `$value`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// In those tests non-sequential field is defined in the struct
        /// after sequential, so it will be deserialized after the list.
        /// That struct should be deserialized from the XML where these
        /// fields comes in an arbitrary order
        mod field_after_list {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct Root {
                #[serde(rename = "$value")]
                item: Vec<Choice>,
                node: (),
            }

            #[test]
            fn before() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <node/>
                        <one/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        node: (),
                    }
                );
            }

            #[test]
            fn after() {
                let data: Root = from_str(
                    r#"
                    <root>
                        <one/>
                        <two/>
                        <three/>
                        <node/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    Root {
                        item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        node: (),
                    }
                );
            }

            #[test]
            fn overlapped() {
                let data = from_str::<Root>(
                    r#"
                    <root>
                        <one/>
                        <node/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        node: (),
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `$value`"),
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `$value`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }

            /// Test for https://github.com/tafia/quick-xml/issues/435
            #[test]
            fn overlapped_with_nested_list() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    #[serde(rename = "$value")]
                    item: Vec<Choice4>,
                    node: (),
                }

                let data = from_str::<Root>(
                    r#"
                    <root>
                        <one><inner/></one>
                        <node/>
                        <two><inner/></two>
                        <three><inner/></three>
                    </root>
                    "#,
                );

                #[cfg(feature = "overlapped-lists")]
                assert_eq!(
                    data.unwrap(),
                    Root {
                        item: vec![
                            Choice4::One { inner: [()] },
                            Choice4::Two { inner: [()] },
                            Choice4::Other,
                        ],
                        node: (),
                    }
                );

                #[cfg(not(feature = "overlapped-lists"))]
                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "duplicate field `$value`")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `$value`"))`, but got `{:?}`"#,
                        e
                    ),
                }
            }
        }

        /// In those tests two lists are deserialized simultaneously.
        /// Lists should be deserialized even when them overlaps
        mod two_lists {
            use super::*;

            /// A field with a variable-name items defined before a field with a fixed-name
            /// items
            mod choice_and_fixed {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    #[serde(rename = "$value")]
                    item: Vec<Choice>,
                    element: Vec<()>,
                }

                /// A list with fixed-name elements located before a list with variable-name
                /// elements in an XML
                #[test]
                fn fixed_before() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <element/>
                            <element/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            element: vec![(), ()],
                        }
                    );
                }

                /// A list with fixed-name elements located after a list with variable-name
                /// elements in an XML
                #[test]
                fn fixed_after() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <element/>
                            <element/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            element: vec![(), ()],
                        }
                    );
                }

                mod overlapped {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        #[serde(rename = "$value")]
                        item: Vec<Choice4>,
                        element: Vec<()>,
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a fixed-name one
                    #[test]
                    fn fixed_before() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <element/>
                                <one/>
                                <two/>
                                <element/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "duplicate field `element`")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("duplicate field `element`"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a variable-name one
                    #[test]
                    fn fixed_after() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <element/>
                                <two/>
                                <three/>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "duplicate field `$value`")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("duplicate field `$value`"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// Test for https://github.com/tafia/quick-xml/issues/435
                    #[test]
                    fn with_nested_list_fixed_before() {
                        let data = from_str::<Root>(
                            r#"
                            <root>
                                <element/>
                                <one><inner/></one>
                                <two><inner/></two>
                                <element/>
                                <three><inner/></three>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Root {
                                item: vec![
                                    Choice4::One { inner: [()] },
                                    Choice4::Two { inner: [()] },
                                    Choice4::Other,
                                ],
                                element: vec![(); 2],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "duplicate field `element`")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("duplicate field `element`"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// Test for https://github.com/tafia/quick-xml/issues/435
                    #[test]
                    fn with_nested_list_fixed_after() {
                        let data = from_str::<Root>(
                            r#"
                            <root>
                                <one><inner/></one>
                                <element/>
                                <two><inner/></two>
                                <three><inner/></three>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Root {
                                item: vec![
                                    Choice4::One { inner: [()] },
                                    Choice4::Two { inner: [()] },
                                    Choice4::Other,
                                ],
                                element: vec![(); 2],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "duplicate field `$value`")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("duplicate field `$value`"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }
                }
            }

            /// A field with a variable-name items defined after a field with a fixed-name
            /// items
            mod fixed_and_choice {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    element: Vec<()>,
                    #[serde(rename = "$value")]
                    item: Vec<Choice>,
                }

                /// A list with fixed-name elements located before a list with variable-name
                /// elements in an XML
                #[test]
                fn fixed_before() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <element/>
                            <element/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            element: vec![(), ()],
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );
                }

                /// A list with fixed-name elements located after a list with variable-name
                /// elements in an XML
                #[test]
                fn fixed_after() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <element/>
                            <element/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            element: vec![(), ()],
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );
                }

                mod overlapped {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        element: Vec<()>,
                        #[serde(rename = "$value")]
                        item: Vec<Choice4>,
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a fixed-name one
                    #[test]
                    fn fixed_before() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <element/>
                                <one/>
                                <two/>
                                <element/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                element: vec![(), ()],
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "duplicate field `element`")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("duplicate field `element`"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a variable-name one
                    #[test]
                    fn fixed_after() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <element/>
                                <two/>
                                <three/>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                element: vec![(), ()],
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "duplicate field `$value`")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("duplicate field `$value`"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// Test for https://github.com/tafia/quick-xml/issues/435
                    #[test]
                    fn with_nested_list_fixed_before() {
                        let data = from_str::<Root>(
                            r#"
                            <root>
                                <element/>
                                <one><inner/></one>
                                <two><inner/></two>
                                <element/>
                                <three><inner/></three>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Root {
                                element: vec![(); 2],
                                item: vec![
                                    Choice4::One { inner: [()] },
                                    Choice4::Two { inner: [()] },
                                    Choice4::Other,
                                ],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "duplicate field `element`")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("duplicate field `element`"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }

                    /// Test for https://github.com/tafia/quick-xml/issues/435
                    #[test]
                    fn with_nested_list_fixed_after() {
                        let data = from_str::<Root>(
                            r#"
                            <root>
                                <one><inner/></one>
                                <element/>
                                <two><inner/></two>
                                <three><inner/></three>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Root {
                                element: vec![(); 2],
                                item: vec![
                                    Choice4::One { inner: [()] },
                                    Choice4::Two { inner: [()] },
                                    Choice4::Other,
                                ],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "duplicate field `$value`")
                            }
                            e => panic!(
                                r#"Expected `Err(Custom("duplicate field `$value`"))`, but got `{:?}`"#,
                                e
                            ),
                        }
                    }
                }
            }

            /// Tests are ignored, but exists to show a problem.
            /// May be it will be solved in the future
            mod choice_and_choice {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    #[serde(rename = "$value")]
                    item: Vec<Choice>,
                    // Actually, we cannot rename both fields to `$value`, which is now
                    // required to indicate, that field accepts elements with any name
                    #[serde(rename = "$value")]
                    element: Vec<Choice2>,
                }

                #[test]
                #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                fn splitted() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <first/>
                            <second/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            element: vec![Choice2::First, Choice2::Second],
                        }
                    );
                }

                #[test]
                #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                fn overlapped() {
                    let data = from_str::<Pair>(
                        r#"
                        <root>
                            <one/>
                            <first/>
                            <two/>
                            <second/>
                            <three/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Pair {
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            element: vec![Choice2::First, Choice2::Second],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected `Err(Custom("invalid length 1, expected an array of length 3"))`, but got `{:?}`"#,
                            e
                        ),
                    }
                }
            }
        }

        /// Deserialization of primitives slightly differs from deserialization
        /// of complex types, so need to check this separately
        #[test]
        fn primitives() {
            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                #[serde(rename = "$value")]
                item: Vec<Choice3>,
            }

            let data: List = from_str(
                r#"
                <root>
                    <one>41</one>
                    <two>42</two>
                    <three>43</three>
                </root>
                "#,
            )
            .unwrap();

            assert_eq!(
                data,
                List {
                    item: vec![
                        Choice3::One(41),
                        Choice3::Two("42".to_string()),
                        Choice3::Other,
                    ],
                }
            );

            from_str::<List>(
                r#"
                <root>
                    <one>41</one>
                    <two><item>42</item></two>
                    <three>43</three>
                </root>
                "#,
            )
            .unwrap_err();
        }

        /// Checks that sequences represented by elements can contain sequences,
        /// represented by `xs:list`s
        mod xs_list {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, Deserialize, PartialEq)]
            struct List {
                /// Outer list mapped to elements, inner -- to `xs:list`.
                ///
                /// `#[serde(default)]` is required to correctly deserialize
                /// empty sequence, because without elements the field
                /// also is missing and derived `Deserialize` implementation
                /// would complain about that unless field is marked as
                /// `default`.
                #[serde(default)]
                #[serde(rename = "$value")]
                element: Vec<Vec<String>>,
            }

            /// Special case: zero elements
            #[test]
            fn zero() {
                let data: List = from_str(
                    r#"
                    <root>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(data, List { element: vec![] });
            }

            /// Special case: one element
            #[test]
            fn one() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item>first list</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        element: vec![vec!["first".to_string(), "list".to_string()]]
                    }
                );
            }

            /// Special case: empty `xs:list`
            #[test]
            fn empty() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        element: vec![vec![]]
                    }
                );
            }

            /// Special case: outer list is always mapped to an elements sequence,
            /// not to an `xs:list`
            #[test]
            fn element() {
                #[derive(Debug, Deserialize, PartialEq)]
                struct List {
                    /// Outer list mapped to elements.
                    #[serde(rename = "$value")]
                    element: Vec<String>,
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item>first item</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        element: vec!["first item".to_string()]
                    }
                );
            }

            /// This tests demonstrates, that for `$value` field (`list`) actual
            /// name of XML element (`item`) does not matter. That allows list
            /// item to be an enum, where tag name determines enum variant
            #[test]
            fn many() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item>first list</item>
                        <item>second list</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        element: vec![
                            vec!["first".to_string(), "list".to_string()],
                            vec!["second".to_string(), "list".to_string()],
                        ]
                    }
                );
            }
        }
    }
}
