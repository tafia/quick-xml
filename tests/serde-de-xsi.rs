//! Tests for ensure behavior of `xsi:nil` handling.
//!
//! We want to threat element with `xsi:nil="true"` as `None` in optional contexts.
use quick_xml::se::to_string;
use quick_xml::DeError;

use serde::{Deserialize, Serialize};

mod serde_helpers;
use serde_helpers::from_str;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Foo {
    elem: String,
}

macro_rules! assert_error_matches {
    ($res: expr, $err: pat) => {
        assert!(
            matches!($res, Err($err)),
            concat!("Expected `", stringify!($err), "`, but got `{:?}`"),
            $res
        );
    };
}

mod top_level_option {
    use super::*;

    mod empty {
        use super::*;

        /// Without `xsi:nil="true"` tags in optional contexts are always considered as having
        /// `Some` value, but because we do not have `tag` element, deserialization failed
        #[test]
        fn none() {
            let xml = r#"<foo/>"#;
            assert_error_matches!(from_str::<Option<Foo>>(xml), DeError::Custom(_));
        }

        /// When prefix is not defined, attributes not bound to any namespace (unlike elements),
        /// so just `nil="true"` does not mean that `xsi:nil` is set
        mod no_prefix {
            use super::*;

            #[test]
            fn true_() {
                let xml = r#"<foo xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="true"/>"#;
                assert_error_matches!(from_str::<Option<Foo>>(xml), DeError::Custom(_));
            }

            #[test]
            fn false_() {
                let xml = r#"<foo xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="false"/>"#;
                assert_error_matches!(from_str::<Option<Foo>>(xml), DeError::Custom(_));
            }
        }

        /// Check canonical prefix
        mod xsi {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn true_() {
                let xml = r#"<foo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true"/>"#;
                assert_eq!(from_str::<Option<Foo>>(xml).unwrap(), None);
            }

            #[test]
            fn false_() {
                let xml = r#"<foo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="false"/>"#;
                assert_error_matches!(from_str::<Option<Foo>>(xml), DeError::Custom(_));
            }
        }

        /// Check other prefix to be sure that we not process only canonical prefixes
        mod ns0 {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn true_() {
                let xml = r#"<foo xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="true"/>"#;
                assert_eq!(from_str::<Option<Foo>>(xml).unwrap(), None);
            }

            #[test]
            fn false_() {
                let xml = r#"<foo xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="false"/>"#;
                assert_error_matches!(from_str::<Option<Foo>>(xml), DeError::Custom(_));
            }
        }
    }

    /// We have no place to store attribute of the element, so the behavior must be the same
    /// as without attributes.
    mod with_attr {
        use super::*;

        #[test]
        fn none() {
            let xml = r#"<foo attr="value"/>"#;
            assert_error_matches!(from_str::<Option<Foo>>(xml), DeError::Custom(_));
        }

        /// When prefix is not defined, attributes not bound to any namespace (unlike elements),
        /// so just `nil="true"` does not mean that `xsi:nil` is set
        mod no_prefix {
            use super::*;

            #[test]
            fn true_() {
                let xml = r#"<foo xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="true" attr="value"/>"#;
                assert_error_matches!(from_str::<Option<Foo>>(xml), DeError::Custom(_));
            }

            #[test]
            fn false_() {
                let xml = r#"<foo xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="false" attr="value"/>"#;
                assert_error_matches!(from_str::<Option<Foo>>(xml), DeError::Custom(_));
            }
        }

        /// Check canonical prefix
        mod xsi {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn true_() {
                let xml = r#"<foo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true" attr="value"/>"#;
                assert_eq!(from_str::<Option<Foo>>(xml).unwrap(), None);
            }

            #[test]
            fn false_() {
                let xml = r#"<foo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="false" attr="value"/>"#;
                assert_error_matches!(from_str::<Option<Foo>>(xml), DeError::Custom(_));
            }
        }

        /// Check other prefix to be sure that we not process only canonical prefixes
        mod ns0 {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn true_() {
                let xml = r#"<foo xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="true" attr="value"/>"#;
                assert_eq!(from_str::<Option<Foo>>(xml).unwrap(), None);
            }

            #[test]
            fn false_() {
                let xml = r#"<foo xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="false" attr="value"/>"#;
                assert_error_matches!(from_str::<Option<Foo>>(xml), DeError::Custom(_));
            }
        }
    }

    mod with_element {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn none() {
            let xml = r#"<foo><elem>Foo</elem></foo>"#;
            assert_eq!(
                from_str::<Option<Foo>>(xml).unwrap(),
                Some(Foo { elem: "Foo".into() })
            );
        }

        /// When prefix is not defined, attributes not bound to any namespace (unlike elements),
        /// so just `nil="true"` does not mean that `xsi:nil` is set
        mod no_prefix {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn true_() {
                let xml = r#"<foo xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="true"><elem>Foo</elem></foo>"#;
                assert_eq!(
                    from_str::<Option<Foo>>(xml).unwrap(),
                    Some(Foo { elem: "Foo".into() })
                );
            }

            #[test]
            fn false_() {
                let xml = r#"<foo xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="false"><elem>Foo</elem></foo>"#;
                assert_eq!(
                    from_str::<Option<Foo>>(xml).unwrap(),
                    Some(Foo { elem: "Foo".into() })
                );
            }
        }

        /// Check canonical prefix
        mod xsi {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn true_() {
                let xml = r#"<foo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true"><elem>Foo</elem></foo>"#;
                assert_eq!(from_str::<Option<Foo>>(xml).unwrap(), None);
            }

            #[test]
            fn false_() {
                let xml = r#"<foo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="false"><elem>Foo</elem></foo>"#;
                assert_eq!(
                    from_str::<Option<Foo>>(xml).unwrap(),
                    Some(Foo { elem: "Foo".into() })
                );
            }
        }

        /// Check other prefix to be sure that we not process only canonical prefixes
        mod ns0 {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn true_() {
                let xml = r#"<foo xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="true"><elem>Foo</elem></foo>"#;
                assert_eq!(from_str::<Option<Foo>>(xml).unwrap(), None);
            }

            #[test]
            fn false_() {
                let xml = r#"<foo xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="false"><elem>Foo</elem></foo>"#;
                assert_eq!(
                    from_str::<Option<Foo>>(xml).unwrap(),
                    Some(Foo { elem: "Foo".into() })
                );
            }
        }
    }
}

mod as_field {
    use super::*;

    /// According to the [specification], `xsi:nil` controls only ability to (not) have nested
    /// elements, but it does not applied to attributes. Due to that we ensure, that attributes
    /// are still can be accessed.
    ///
    /// [specification]: https://www.w3.org/TR/xmlschema11-1/#Instance_Document_Constructions
    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct AnyName {
        #[serde(rename = "@attr")]
        attr: Option<String>,

        elem: Option<String>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Root {
        foo: AnyName,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Bar {
        foo: Option<Foo>,
    }

    macro_rules! check {
        (
            $name:ident,
            $true_xml:literal,
            $false_xml:literal,
            $se_xml:literal,
            $attr:expr,
        ) => {
            mod $name {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn true_() {
                    let value = AnyName {
                        attr: $attr,
                        elem: None,
                    };

                    assert_eq!(to_string(&value).unwrap(), $se_xml);
                    assert_eq!(from_str::<AnyName>($true_xml).unwrap(), value);
                }

                #[test]
                fn false_() {
                    let value = AnyName {
                        attr: $attr,
                        elem: None,
                    };

                    assert_eq!(to_string(&value).unwrap(), $se_xml);
                    assert_eq!(from_str::<AnyName>($false_xml).unwrap(), value);
                }
            }
        };
    }

    mod empty {
        use super::*;
        use pretty_assertions::assert_eq;

        /// Without `xsi:nil="true"` tags in optional contexts are always considered as having
        /// `Some` value, but because we do not have `tag` element, deserialization failed
        #[test]
        fn none() {
            let value = AnyName {
                attr: None,
                elem: None,
            };

            assert_eq!(
                to_string(&value).unwrap(),
                r#"<AnyName attr=""><elem/></AnyName>"#
            );
            assert_eq!(
                from_str::<AnyName>("<AnyName/>").unwrap(),
                AnyName {
                    attr: None,
                    elem: None,
                }
            );
        }

        // When prefix is not defined, attributes not bound to any namespace (unlike elements),
        // so just `nil="true"` does not mean that `xsi:nil` is set. But because `AnyName` is empty
        // there anyway nothing inside, so all fields will be set to `None`
        check!(
            no_prefix,
            r#"<AnyName xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="true"/>"#,
            r#"<AnyName xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="false"/>"#,
            r#"<AnyName attr=""><elem/></AnyName>"#,
            None,
        );

        // Check canonical prefix
        check!(
            xsi,
            r#"<AnyName xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true"/>"#,
            r#"<AnyName xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="false"/>"#,
            r#"<AnyName attr=""><elem/></AnyName>"#,
            None,
        );

        // Check other prefix to be sure that we do not process only canonical prefixes
        check!(
            ns0,
            r#"<AnyName xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="true"/>"#,
            r#"<AnyName xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="false"/>"#,
            r#"<AnyName attr=""><elem/></AnyName>"#,
            None,
        );

        mod nested {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn none() {
                let xml =
                    r#"<bar xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><foo/></bar>"#;

                assert_error_matches!(from_str::<Bar>(xml), DeError::Custom(_));
                assert_eq!(
                    from_str::<Root>(xml).unwrap(),
                    Root {
                        foo: AnyName {
                            attr: None,
                            elem: None,
                        },
                    }
                );
            }

            #[test]
            fn true_() {
                let xml = r#"<bar xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><foo xsi:nil="true"/></bar>"#;

                assert_eq!(from_str::<Bar>(xml).unwrap(), Bar { foo: None });
                assert_eq!(
                    from_str::<Root>(xml).unwrap(),
                    Root {
                        foo: AnyName {
                            attr: None,
                            elem: None,
                        },
                    }
                );
            }

            #[test]
            fn false_() {
                let xml = r#"<bar xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><foo xsi:nil="false"/></bar>"#;

                assert_error_matches!(from_str::<Bar>(xml), DeError::Custom(_));
                assert_eq!(
                    from_str::<Root>(xml).unwrap(),
                    Root {
                        foo: AnyName {
                            attr: None,
                            elem: None,
                        },
                    }
                );
            }
        }
    }

    /// We have no place to store attribute of the element, so the behavior must be the same
    /// as without attributes.
    mod with_attr {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn none() {
            let value = AnyName {
                attr: Some("value".into()),
                elem: None,
            };

            assert_eq!(
                to_string(&value).unwrap(),
                r#"<AnyName attr="value"><elem/></AnyName>"#
            );
            assert_eq!(
                from_str::<AnyName>(r#"<AnyName attr="value"/>"#).unwrap(),
                value
            );
        }

        // When prefix is not defined, attributes not bound to any namespace (unlike elements),
        // so just `nil="true"` does not mean that `xsi:nil` is set. But because `AnyName` is empty
        // there anyway nothing inside, so all element fields will be set to `None`
        check!(
            no_prefix,
            r#"<AnyName xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="true" attr="value"/>"#,
            r#"<AnyName xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="false" attr="value"/>"#,
            r#"<AnyName attr="value"><elem/></AnyName>"#,
            Some("value".into()),
        );

        // Check canonical prefix
        check!(
            xsi,
            r#"<AnyName xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true" attr="value"/>"#,
            r#"<AnyName xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="false" attr="value"/>"#,
            r#"<AnyName attr="value"><elem/></AnyName>"#,
            Some("value".into()),
        );

        // Check other prefix to be sure that we do not process only canonical prefixes
        check!(
            ns0,
            r#"<AnyName xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="true" attr="value"/>"#,
            r#"<AnyName xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="false" attr="value"/>"#,
            r#"<AnyName attr="value"><elem/></AnyName>"#,
            Some("value".into()),
        );

        mod nested {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn none() {
                let xml = r#"<bar xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><foo attr="value"/></bar>"#;

                // Without `xsi:nil="true"` <foo> is mapped to `foo` field,
                // but failed to deserialzie because of missing required <elem> tag
                assert_error_matches!(from_str::<Bar>(xml), DeError::Custom(_));
                assert_eq!(
                    from_str::<Root>(xml).unwrap(),
                    Root {
                        foo: AnyName {
                            attr: Some("value".into()),
                            elem: None,
                        },
                    }
                );
            }

            #[test]
            fn true_() {
                let xml = r#"<bar xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><foo xsi:nil="true" attr="value"/></bar>"#;

                assert_eq!(from_str::<Bar>(xml).unwrap(), Bar { foo: None });
                assert_eq!(
                    from_str::<Root>(xml).unwrap(),
                    Root {
                        foo: AnyName {
                            attr: Some("value".into()),
                            elem: None,
                        },
                    }
                );
            }

            #[test]
            fn false_() {
                let xml = r#"<bar xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><foo xsi:nil="false" attr="value"/></bar>"#;

                // With `xsi:nil="false"` <foo> is mapped to `foo` field,
                // but failed to deserialzie because of missing required <elem> tag
                assert_error_matches!(from_str::<Bar>(xml), DeError::Custom(_));
                assert_eq!(
                    from_str::<Root>(xml).unwrap(),
                    Root {
                        foo: AnyName {
                            attr: Some("value".into()),
                            elem: None,
                        },
                    }
                );
            }
        }
    }

    mod with_element {
        use super::*;
        use pretty_assertions::assert_eq;

        macro_rules! check {
            (
                $name:ident,

                $de_true_xml:literal,
                $se_true_xml:literal,

                $de_false_xml:literal,
                $se_false_xml:literal,
            ) => {
                mod $name {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn true_() {
                        let value = AnyName {
                            attr: None,
                            // Becase `nil=true``, element deserialized as `None`
                            elem: None,
                        };

                        assert_eq!(to_string(&value).unwrap(), $se_true_xml);
                        assert_eq!(from_str::<AnyName>($de_true_xml).unwrap(), value);
                    }

                    #[test]
                    fn false_() {
                        let value = AnyName {
                            attr: None,
                            elem: Some("Foo".into()),
                        };

                        assert_eq!(to_string(&value).unwrap(), $se_false_xml);
                        assert_eq!(from_str::<AnyName>($de_false_xml).unwrap(), value);
                    }
                }
            };
        }

        #[test]
        fn none() {
            let value = AnyName {
                attr: None,
                elem: Some("Foo".into()),
            };

            assert_eq!(
                to_string(&value).unwrap(),
                r#"<AnyName attr=""><elem>Foo</elem></AnyName>"#
            );
            assert_eq!(
                from_str::<AnyName>(r#"<AnyName><elem>Foo</elem></AnyName>"#).unwrap(),
                value
            );
        }

        /// When prefix is not defined, attributes not bound to any namespace (unlike elements),
        /// so just `nil="true"` does not mean that `xsi:nil` is set
        mod no_prefix {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn true_() {
                let se_xml = r#"<AnyName attr=""><elem>Foo</elem></AnyName>"#;
                let de_xml = r#"<AnyName xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="true"><elem>Foo</elem></AnyName>"#;

                let value = AnyName {
                    attr: None,
                    elem: Some("Foo".into()),
                };

                assert_eq!(to_string(&value).unwrap(), se_xml);
                assert_eq!(from_str::<AnyName>(de_xml).unwrap(), value);
            }

            #[test]
            fn false_() {
                let se_xml = r#"<AnyName attr=""><elem>Foo</elem></AnyName>"#;
                let de_xml = r#"<AnyName xmlns="http://www.w3.org/2001/XMLSchema-instance" nil="false"><elem>Foo</elem></AnyName>"#;

                let value = AnyName {
                    attr: None,
                    elem: Some("Foo".into()),
                };

                assert_eq!(to_string(&value).unwrap(), se_xml);
                assert_eq!(from_str::<AnyName>(de_xml).unwrap(), value);
            }
        }

        // Check canonical prefix
        check!(
            xsi,
            r#"<AnyName xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true"><elem>Foo</elem></AnyName>"#,
            r#"<AnyName attr=""><elem/></AnyName>"#,
            r#"<AnyName xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="false"><elem>Foo</elem></AnyName>"#,
            r#"<AnyName attr=""><elem>Foo</elem></AnyName>"#,
        );

        // Check other prefix to be sure that we do not process only canonical prefixes
        check!(
            ns0,
            r#"<AnyName xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="true"><elem>Foo</elem></AnyName>"#,
            r#"<AnyName attr=""><elem/></AnyName>"#,
            r#"<AnyName xmlns:ns0="http://www.w3.org/2001/XMLSchema-instance" ns0:nil="false"><elem>Foo</elem></AnyName>"#,
            r#"<AnyName attr=""><elem>Foo</elem></AnyName>"#,
        );

        mod nested {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn none() {
                let xml = r#"<bar xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><foo><elem>Foo</elem></foo></bar>"#;

                assert_eq!(
                    from_str::<Bar>(xml).unwrap(),
                    Bar {
                        foo: Some(Foo { elem: "Foo".into() }),
                    }
                );
                assert_eq!(
                    from_str::<Root>(xml).unwrap(),
                    Root {
                        foo: AnyName {
                            attr: None,
                            elem: Some("Foo".into()),
                        },
                    }
                );
            }

            #[test]
            fn true_() {
                let xml = r#"<bar xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><foo xsi:nil="true"><elem>Foo</elem></foo></bar>"#;

                assert_eq!(from_str::<Bar>(xml).unwrap(), Bar { foo: None });
                assert_eq!(
                    from_str::<Root>(xml).unwrap(),
                    Root {
                        foo: AnyName {
                            attr: None,
                            elem: None,
                        },
                    }
                );
            }

            #[test]
            fn false_() {
                let xml = r#"<bar xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><foo xsi:nil="false"><elem>Foo</elem></foo></bar>"#;

                assert_eq!(
                    from_str::<Bar>(xml).unwrap(),
                    Bar {
                        foo: Some(Foo { elem: "Foo".into() }),
                    }
                );
                assert_eq!(
                    from_str::<Root>(xml).unwrap(),
                    Root {
                        foo: AnyName {
                            attr: None,
                            elem: Some("Foo".into()),
                        },
                    }
                );
            }
        }
    }
}
