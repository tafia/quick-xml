extern crate quick_xml_derived;
use quick_xml_derived::QuickXml;
use q_meta::QuickXmlMeta;
#[derive(QuickXml)]
#[qxml{
    xmlns:F="foorun"
    xmlns:B="barurn"
    xmlns="http://this-is-a-default-namespace"
}]
struct Foo {
    #[qxml{ pre:B }]
    id: String
}

fn check_foo_has_qxml_meta() {
    foo = Foo { id: "asdf".to_string()};
}

fn has_qxml_meta<T: QuickXmlMeta>(t: T) {
    t.get_quick_xml_meta();
}

