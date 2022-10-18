extern crate quick_xml_derived;
use quick_xml_derived::QuickXml;
use q_meta::QuickXmlMeta;
#[qxml{xmlns:F = "foorun"
xmlns:B = "barurn"
xmlns = "http://this-is-a-default-namespace"}]
struct Foo {
    #[qxml{pre:B}]
    id: String,
}
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    use phf::phf_map;
    #[automatically_derived]
    impl Foo {
        fn get_quick_xml_meta() -> &'static QuickXmlItemMeta {
            &QuickXmlItemMeta {
                namespace_declarations: &[
                    ("C", "http://asdfasdf"),
                    ("B", "asdf"),
                    ("", ""),
                ],
                identifier_prefix_map: phf::Map {
                    key: 12913932095322966823u64,
                    disps: &[(0u32, 0u32)],
                    entries: &[("jax", "B"), ("foo", "C"), ("bar", "B")],
                },
            }
        }
    }
};
fn check_foo_has_qxml_meta() {
    foo = Foo { id: "asdf".to_string() };
}
fn has_qxml_meta<T: QuickXmlMeta>(t: T) {
    t.get_quick_xml_meta();
}
