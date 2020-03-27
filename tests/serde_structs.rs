#![cfg(feature = "serialize")]

extern crate quick_xml;
extern crate serde;

use quick_xml::{
    de::from_str,
    se
};
use serde::{Deserialize, Serialize};


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_struct_complex_outer()
    {
        #[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
        #[serde(rename = "inner", default)]
        struct Inner {
            in_dummy: u32
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
        struct Test1 {
            dummy1: u32,
            dummy2: String,
            inner: Inner,
        }

        const TEST_XML_1: &str =
            r#"<Test1 dummy1="10" dummy2="bar"><inner in_dummy="30"></inner></Test1>"#;

        println!{};
        println!{"original XML {}", TEST_XML_1};
        println!{};

        let outer: Test1 = match from_str(TEST_XML_1) {
            Ok(foo) => foo,
            Err(e) => {
                println!{"deserialize error {:?}", e};
                assert!{false};
                return;
            }
        };

        println!{"outer {:?}", &outer};
        println!{};

        let xml2 = match se::to_string(&outer) {
            Ok(xml2) => xml2,
            Err(e) => {
                println!{"serialize error {:?}", e};
                assert!{false};
                return;
            }
        };
        println!{"serialized {:?}", &xml2};
        println!{};

        assert_eq!(TEST_XML_1, &xml2)
    }

    #[test]
    fn test_serialize_deserialize_struct_nested()
    {
        #[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
        #[serde(rename = "inner", default)]
        struct Inner {
            in_dummy: u32
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
        struct Test2 {
            inner: Inner,
        }
        const TEST_XML_2: &str =
            r#"<Test2><inner in_dummy="30"></inner></Test2>"#;

        println!{};
        println!{"original XML {}", TEST_XML_2};
        println!{};

        let outer: Test2 = match from_str(TEST_XML_2) {
            Ok(foo) => foo,
            Err(e) => {
                println!{"deserialize error {:?}", e};
                assert!{false};
                return;
            }
        };

        println!{"outer {:?}", &outer};
        println!{};

        let xml2 = match se::to_string(&outer) {
            Ok(xml2) => xml2,
            Err(e) => {
                println!{"serialize error {:?}", e};
                assert!{false};
                return;
            }
        };
        println!{"serialized {:?}", &xml2};
        println!{};

        assert_eq!(TEST_XML_2, &xml2)
    }
}
