use quick_xml_derived::QuickXml;
use q_meta::QuickXmlItemMeta;

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

fn main() {
    let foo = Foo{
        id: "asdf".to_string()
    };
    println!("{:?}", Foo::get_quick_xml_meta());
}