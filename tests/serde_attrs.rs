#![cfg(feature = "serialize")]

extern crate quick_xml;
extern crate regex;
extern crate serde;

use quick_xml::se::to_string;
use regex::Regex;
use serde::Serialize;
use std::borrow::Cow;

#[derive(Serialize)]
#[serde(rename = "classroom")]
struct Classroom {
    pub students: Students,
    pub number: String,
    pub adviser: Person,
}

#[derive(Serialize)]
struct Students {
    #[serde(rename = "person")]
    pub persons: Vec<Person>,
}

#[derive(Serialize)]
struct Person {
    pub name: String,
    pub age: u32,
}

#[derive(Serialize)]
#[serde(rename = "empty")]
struct Empty {}

#[test]
fn test_nested() {
    let s1 = Person {
        name: "sherlock".to_string(),
        age: 20,
    };
    let s2 = Person {
        name: "harry".to_string(),
        age: 19,
    };
    let t = Person {
        name: "albus".to_string(),
        age: 88,
    };
    let doc = Classroom {
        students: Students {
            persons: vec![s1, s2],
        },
        number: "3-1".to_string(),
        adviser: t,
    };
    let xml = quick_xml::se::to_string(&doc).unwrap();

    let str = r#"<classroom number="3-1">
                   <students>
                      <person name="sherlock" age="20"/>
                      <person name="harry" age="19"/>
                   </students>
                   <adviser name="albus" age="88"/>
                 </classroom>"#;
    assert_eq!(xml, inline(str));
}

fn inline(str: &str) -> Cow<str> {
    let regex = Regex::new(r">\s+<").unwrap();
    regex.replace_all(str, "><")
}

#[test]
fn test_empty() {
    let e = Empty {};
    let xml = to_string(&e).unwrap();
    assert_eq!(xml, "<empty/>");
}
