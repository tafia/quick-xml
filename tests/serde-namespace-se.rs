use quick_xml::se::to_string;
use quick_xml_derived::QuickXml;
use regex::Regex;
use serde::Serialize;
use std::borrow::Cow;

use pretty_assertions::assert_eq;

#[derive(QuickXml, Serialize)]
#[serde(rename = "classroom")]
#[qxml{
    xmlns:B="hello"
    xmlns:C="asdfasdfasdf"
    xmlns="http://google.com"
}]
struct Classroom {
    #[qxml{ pre:B }]
    pub students: Students,
    #[qxml{ pre:B }]
    pub number: String,
    pub adviser: Person,
}

#[derive(QuickXml, Serialize)]
#[qxml{
    xmlns="http://reddit.com"
}]
struct Students {
    //#[serde(rename = "person")]
    #[qxml{ pre:B }]
    pub persons: Vec<Person>,
}

#[derive(QuickXml, Serialize)]
struct Person {
    #[qxml{ pre:B }]
    pub name: String,
    pub age: u32,
}

#[test]
fn struct_test() {
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
    let xml = quick_xml::se::to_string_with_qxml_meta(&doc).unwrap();
    let str = r#"<classroom xmlns:B="hello" xmlns:C="asdfasdfasdf" xmlns="http://google.com" B:number="3-1">
                   <students xmlns="http://reddit.com">
                      <persons B:name="sherlock" age="20"/>
                      <persons B:name="harry" age="19"/>
                   </students>
                   <adviser B:name="albus" age="88"/>
                 </classroom>"#;
    assert_eq!(xml, inline(str));

}

fn inline(str: &str) -> Cow<str> {
    let regex = Regex::new(r">\s+<").unwrap();
    regex.replace_all(str, "><")
}

