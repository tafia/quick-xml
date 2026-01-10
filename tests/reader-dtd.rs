use quick_xml::errors::{Error, SyntaxError};
use quick_xml::events::{BytesText, Event};
use quick_xml::reader::{NsReader, Reader};

// For event_ok and syntax_err macros
mod helpers;

macro_rules! ok {
    ($test:ident : $pos:literal $xml:literal $event:literal) => {
        event_ok!($test ($xml) => $pos : Event::DocType(BytesText::from_escaped($event)));
    };
}

mod without_internal_subset {
    use super::*;

    ok!(simple: 15
        "<!DOCTYPE root>"
                  "root"
    );
    ok!(with_external_id_1: 21
        r#"<!DOCTYPE root ">['">"#
                  r#"root ">['""#
    );
    ok!(with_external_id_2: 21
        r#"<!DOCTYPE root '>["'>"#
                  r#"root '>["'"#
    );
    ok!(with_external_id_3: 22
        r#"<!DOCTYPE root ">['" >"#
                  r#"root ">['" "#
    );
    ok!(with_external_id_4: 22
        r#"<!DOCTYPE root '>["' >"#
                  r#"root '>["' "#
    );
}

ok!(with_external_id_1: 23
    r#"<!DOCTYPE root ">['"[]>"#
              r#"root ">['"[]"#
);
ok!(with_external_id_2: 23
    r#"<!DOCTYPE root '>["'[]>"#
              r#"root '>["'[]"#
);
ok!(with_external_id_3: 25
    r#"<!DOCTYPE root ">['" [] >"#
              r#"root ">['" [] "#
);
ok!(with_external_id_4: 25
    r#"<!DOCTYPE root '>["' [] >"#
              r#"root '>["' [] "#
);

ok!(entity_1: 35
    r#"<!DOCTYPE root [<!ENTITY ent ">">]>"#
              r#"root [<!ENTITY ent ">">]"#
);
ok!(entity_2: 35
    r#"<!DOCTYPE root [<!ENTITY ent "<">]>"#
              r#"root [<!ENTITY ent "<">]"#
);

ok!(attlist: 86
    r#"<!DOCTYPE root [<!ATTLIST root att "3 > 2 is true" att2 #FIXED '>>> in other quote'>]>"#
              r#"root [<!ATTLIST root att "3 > 2 is true" att2 #FIXED '>>> in other quote'>]"#
);
ok!(notation: 80
    r#"<!DOCTYPE root [<!NOTATION nota PUBLIC "some_public_id" '">>>some_system_id"'>]>"#
              r#"root [<!NOTATION nota PUBLIC "some_public_id" '">>>some_system_id"'>]"#
);
ok!(comment: 51
    r#"<!DOCTYPE root [<!-- < --><!-- >> --><!-- <<< -->]>"#
              r#"root [<!-- < --><!-- >> --><!-- <<< -->]"#
);
ok!(pi: 37
    r#"<!DOCTYPE root [<?pi <<>><<>><><>?>]>"#
              r#"root [<?pi <<>><<>><><>?>]"#
);
ok!(all_together: 164
    "<!DOCTYPE e [
        <!ELEMENT e ANY>
        <!ATTLIST a>
        <!ENTITY ent '>'>
        <!NOTATION n SYSTEM '>'>
        <!-->-->
        <?pi >?>
    ]
    >"
    "e [
        <!ELEMENT e ANY>
        <!ATTLIST a>
        <!ENTITY ent '>'>
        <!NOTATION n SYSTEM '>'>
        <!-->-->
        <?pi >?>
    ]
    "
);

ok!(unknown_dtd_markup: 34
    "<!DOCTYPE e [ <!unknown e ANY> ] >"
              "e [ <!unknown e ANY> ] "
);

mod unclosed {
    use super::*;

    syntax_err!(doctype_1(".<!DOCTYPE root [ ] ") => SyntaxError::UnclosedDoctype);
    syntax_err!(doctype_2(".<!DOCTYPE root \">['\" [ ] ") => SyntaxError::UnclosedDoctype);
    syntax_err!(doctype_3(".<!DOCTYPE root '>[\"' [ ] ") => SyntaxError::UnclosedDoctype);

    syntax_err!(external_id_1(".<!DOCTYPE root '>[\" ") => SyntaxError::UnclosedDoctype);
    syntax_err!(external_id_2(".<!DOCTYPE root \">[' ") => SyntaxError::UnclosedDoctype);

    syntax_err!(internal_subset_1(".<!DOCTYPE root [ ") => SyntaxError::UnclosedDoctype);
    syntax_err!(internal_subset_2(".<!DOCTYPE root \">['\" [ ") => SyntaxError::UnclosedDoctype);
    syntax_err!(internal_subset_3(".<!DOCTYPE root '>[\"' [ ") => SyntaxError::UnclosedDoctype);

    syntax_err!(element(".<!DOCTYPE root [<!ELEMENT ") => SyntaxError::UnclosedDoctype);
    syntax_err!(attlist(".<!DOCTYPE root [<!ATTLIST ") => SyntaxError::UnclosedDoctype);

    syntax_err!(entity_1(".<!DOCTYPE root [<!ENTITY ") => SyntaxError::UnclosedDoctype);
    syntax_err!(entity_2(".<!DOCTYPE root [<!ENTITY ent \">' ") => SyntaxError::UnclosedDoctype);
    syntax_err!(entity_3(".<!DOCTYPE root [<!ENTITY ent '>\" ") => SyntaxError::UnclosedDoctype);

    syntax_err!(notation_1(".<!DOCTYPE root [<!NOTATION ") => SyntaxError::UnclosedDoctype);
    syntax_err!(notation_2(".<!DOCTYPE root [<!NOTATION n SYSTEM \">' ") => SyntaxError::UnclosedDoctype);
    syntax_err!(notation_3(".<!DOCTYPE root [<!NOTATION n SYSTEM '>\" ") => SyntaxError::UnclosedDoctype);

    syntax_err!(comment_1(".<!DOCTYPE root [<!-- ") => SyntaxError::UnclosedDoctype);
    syntax_err!(comment_2(".<!DOCTYPE root [<!--> ") => SyntaxError::UnclosedDoctype);

    syntax_err!(pi_1(".<!DOCTYPE root [<? ") => SyntaxError::UnclosedDoctype);
    syntax_err!(pi_2(".<!DOCTYPE root [<?pi > ") => SyntaxError::UnclosedDoctype);
}
