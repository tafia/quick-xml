extern crate quick_xml_derived;
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

