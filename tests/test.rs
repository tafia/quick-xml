extern crate quick_xml;
#[cfg(feature = "serialize")]
extern crate serde;

use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event::*;
use quick_xml::Reader;
use std::borrow::Cow;
use std::io::Cursor;

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

#[test]
fn test_sample() {
    let src: &[u8] = include_bytes!("sample_rss.xml");
    let mut buf = Vec::new();
    let mut r = Reader::from_reader(src);
    let mut count = 0;
    loop {
        match r.read_event(&mut buf).unwrap() {
            Start(_) => count += 1,
            Decl(e) => println!("{:?}", e.version()),
            Eof => break,
            _ => (),
        }
        buf.clear();
    }
    println!("{}", count);
}

#[test]
fn test_attributes_empty() {
    let src = b"<a att1='a' att2='b'/>";
    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        Ok(Empty(e)) => {
            let mut atts = e.attributes();
            match atts.next() {
                Some(Ok(Attribute {
                    key: b"att1",
                    value: Cow::Borrowed(b"a"),
                })) => (),
                e => panic!("Expecting att1='a' attribute, found {:?}", e),
            }
            match atts.next() {
                Some(Ok(Attribute {
                    key: b"att2",
                    value: Cow::Borrowed(b"b"),
                })) => (),
                e => panic!("Expecting att2='b' attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        }
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_attribute_equal() {
    let src = b"<a att1=\"a=b\"/>";
    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        Ok(Empty(e)) => {
            let mut atts = e.attributes();
            match atts.next() {
                Some(Ok(Attribute {
                    key: b"att1",
                    value: Cow::Borrowed(b"a=b"),
                })) => (),
                e => panic!("Expecting att1=\"a=b\" attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        }
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_comment_starting_with_gt() {
    let src = b"<a /><!-->-->";
    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    loop {
        match r.read_event(&mut buf) {
            Ok(Comment(ref e)) if &**e == b">" => break,
            Ok(Eof) => panic!("Expecting Comment"),
            _ => (),
        }
    }
}

/// Single empty element with qualified attributes.
/// Empty element expansion: disabled
/// The code path for namespace handling is slightly different for `Empty` vs. `Start+End`.
#[test]
fn test_attributes_empty_ns() {
    let src = b"<a att1='a' r:att2='b' xmlns:r='urn:example:r' />";

    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    let e = match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((None, Empty(e))) => e,
        e => panic!("Expecting Empty event, got {:?}", e),
    };

    let mut atts = e
        .attributes()
        .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
        // we don't care about xmlns attributes for this test
        .filter(|kv| !kv.key.starts_with(b"xmlns"))
        .map(|Attribute { key: name, value }| {
            let (opt_ns, local_name) = r.attribute_namespace(name, &ns_buf);
            (opt_ns, local_name, value)
        });
    match atts.next() {
        Some((None, b"att1", Cow::Borrowed(b"a"))) => (),
        e => panic!("Expecting att1='a' attribute, found {:?}", e),
    }
    match atts.next() {
        Some((Some(ns), b"att2", Cow::Borrowed(b"b"))) => {
            assert_eq!(&ns[..], b"urn:example:r");
        }
        e => panic!(
            "Expecting {{urn:example:r}}att2='b' attribute, found {:?}",
            e
        ),
    }
    match atts.next() {
        None => (),
        e => panic!("Expecting None, found {:?}", e),
    }
}

/// Single empty element with qualified attributes.
/// Empty element expansion: enabled
/// The code path for namespace handling is slightly different for `Empty` vs. `Start+End`.
#[test]
fn test_attributes_empty_ns_expanded() {
    let src = b"<a att1='a' r:att2='b' xmlns:r='urn:example:r' />";

    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(true);
    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();
    {
        let e = match r.read_namespaced_event(&mut buf, &mut ns_buf) {
            Ok((None, Start(e))) => e,
            e => panic!("Expecting Empty event, got {:?}", e),
        };

        let mut atts = e
            .attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| !kv.key.starts_with(b"xmlns"))
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.attribute_namespace(name, &ns_buf);
                (opt_ns, local_name, value)
            });
        match atts.next() {
            Some((None, b"att1", Cow::Borrowed(b"a"))) => (),
            e => panic!("Expecting att1='a' attribute, found {:?}", e),
        }
        match atts.next() {
            Some((Some(ns), b"att2", Cow::Borrowed(b"b"))) => {
                assert_eq!(&ns[..], b"urn:example:r");
            }
            e => panic!(
                "Expecting {{urn:example:r}}att2='b' attribute, found {:?}",
                e
            ),
        }
        match atts.next() {
            None => (),
            e => panic!("Expecting None, found {:?}", e),
        }
    }

    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((None, End(e))) => assert_eq!(b"a", e.name()),
        e => panic!("Expecting End event, got {:?}", e),
    }
}

#[test]
fn test_default_ns_shadowing_empty() {
    let src = b"<e xmlns='urn:example:o'><e att1='a' xmlns='urn:example:i' /></e>";

    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    // <outer xmlns='urn:example:o'>
    {
        match r.read_namespaced_event(&mut buf, &mut ns_buf) {
            Ok((Some(ns), Start(e))) => {
                assert_eq!(&ns[..], b"urn:example:o");
                assert_eq!(e.name(), b"e");
            }
            e => panic!("Expected Start event (<outer>), got {:?}", e),
        }
    }

    // <inner att1='a' xmlns='urn:example:i' />
    {
        let e = match r.read_namespaced_event(&mut buf, &mut ns_buf) {
            Ok((Some(ns), Empty(e))) => {
                assert_eq!(::std::str::from_utf8(ns).unwrap(), "urn:example:i");
                assert_eq!(e.name(), b"e");
                e
            }
            e => panic!("Expecting Empty event, got {:?}", e),
        };

        let mut atts = e
            .attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| !kv.key.starts_with(b"xmlns"))
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.attribute_namespace(name, &ns_buf);
                (opt_ns, local_name, value)
            });
        // the attribute should _not_ have a namespace name. The default namespace does not
        // apply to attributes.
        match atts.next() {
            Some((None, b"att1", Cow::Borrowed(b"a"))) => (),
            e => panic!("Expecting att1='a' attribute, found {:?}", e),
        }
        match atts.next() {
            None => (),
            e => panic!("Expecting None, found {:?}", e),
        }
    }

    // </outer>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((Some(ns), End(e))) => {
            assert_eq!(&ns[..], b"urn:example:o");
            assert_eq!(e.name(), b"e");
        }
        e => panic!("Expected End event (<outer>), got {:?}", e),
    }
}

#[test]
fn test_default_ns_shadowing_expanded() {
    let src = b"<e xmlns='urn:example:o'><e att1='a' xmlns='urn:example:i' /></e>";

    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(true);
    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    // <outer xmlns='urn:example:o'>
    {
        match r.read_namespaced_event(&mut buf, &mut ns_buf) {
            Ok((Some(ns), Start(e))) => {
                assert_eq!(&ns[..], b"urn:example:o");
                assert_eq!(e.name(), b"e");
            }
            e => panic!("Expected Start event (<outer>), got {:?}", e),
        }
    }
    buf.clear();

    // <inner att1='a' xmlns='urn:example:i' />
    {
        let e = match r.read_namespaced_event(&mut buf, &mut ns_buf) {
            Ok((Some(ns), Start(e))) => {
                assert_eq!(&ns[..], b"urn:example:i");
                assert_eq!(e.name(), b"e");
                e
            }
            e => panic!("Expecting Start event (<inner>), got {:?}", e),
        };
        let mut atts = e
            .attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| !kv.key.starts_with(b"xmlns"))
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.attribute_namespace(name, &ns_buf);
                (opt_ns, local_name, value)
            });
        // the attribute should _not_ have a namespace name. The default namespace does not
        // apply to attributes.
        match atts.next() {
            Some((None, b"att1", Cow::Borrowed(b"a"))) => (),
            e => panic!("Expecting att1='a' attribute, found {:?}", e),
        }
        match atts.next() {
            None => (),
            e => panic!("Expecting None, found {:?}", e),
        }
    }

    // virtual </inner>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((Some(ns), End(e))) => {
            assert_eq!(&ns[..], b"urn:example:i");
            assert_eq!(e.name(), b"e");
        }
        e => panic!("Expected End event (</inner>), got {:?}", e),
    }
    // </outer>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((Some(ns), End(e))) => {
            assert_eq!(&ns[..], b"urn:example:o");
            assert_eq!(e.name(), b"e");
        }
        e => panic!("Expected End event (</outer>), got {:?}", e),
    }
}

#[test]
#[cfg(feature = "encoding_rs")]
fn test_koi8_r_encoding() {
    let src: &[u8] = include_bytes!("documents/opennews_all.rss");
    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    loop {
        match r.read_event(&mut buf) {
            Ok(Text(e)) => {
                e.unescape_and_decode(&r).unwrap();
            }
            Ok(Eof) => break,
            _ => (),
        }
    }
}

#[test]
fn fuzz_53() {
    let data: &[u8] = b"\xe9\x00\x00\x00\x00\x00\x00\x00\x00\
\x00\x00\x00\x00\n(\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\
\x00<>\x00\x08\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00<<\x00\x00\x00";
    let cursor = Cursor::new(data);
    let mut reader = Reader::from_reader(cursor);
    let mut buf = vec![];
    loop {
        match reader.read_event(&mut buf) {
            Ok(quick_xml::events::Event::Eof) | Err(..) => break,
            _ => buf.clear(),
        }
    }
}

#[test]
fn test_issue94() {
    let data = br#"<Run>
<!B>
</Run>"#;
    let mut reader = Reader::from_reader(&data[..]);
    reader.trim_text(true);
    let mut buf = vec![];
    loop {
        match reader.read_event(&mut buf) {
            Ok(quick_xml::events::Event::Eof) | Err(..) => break,
            _ => buf.clear(),
        }
        buf.clear();
    }
}

#[test]
fn fuzz_101() {
    let data: &[u8] = b"\x00\x00<\x00\x00\x0a>&#44444444401?#\x0a413518\
                       #\x0a\x0a\x0a;<:<)(<:\x0a\x0a\x0a\x0a;<:\x0a\x0a\
                       <:\x0a\x0a\x0a\x0a\x0a<\x00*\x00\x00\x00\x00";
    let cursor = Cursor::new(data);
    let mut reader = Reader::from_reader(cursor);
    let mut buf = vec![];
    loop {
        match reader.read_event(&mut buf) {
            Ok(Start(ref e)) | Ok(Empty(ref e)) => {
                if e.unescaped().is_err() {
                    break;
                }
                for a in e.attributes() {
                    if a.ok().map_or(true, |a| a.unescaped_value().is_err()) {
                        break;
                    }
                }
            }
            Ok(Text(ref e)) => {
                if e.unescaped().is_err() {
                    break;
                }
            }
            Ok(Eof) | Err(..) => break,
            _ => (),
        }
        buf.clear();
    }
}

#[test]
fn test_default_namespace() {
    let mut r = Reader::from_str("<a ><b xmlns=\"www1\"></b></a>");
    r.trim_text(true);

    // <a>
    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();
    if let Ok((None, Start(_))) = r.read_namespaced_event(&mut buf, &mut ns_buf) {
    } else {
        panic!("expecting outer start element with no namespace");
    }

    // <b>
    {
        let event = match r.read_namespaced_event(&mut buf, &mut ns_buf) {
            Ok((Some(b"www1"), Start(event))) => event,
            Ok((Some(_), Start(_))) => panic!("expecting namespace to resolve to 'www1'"),
            _ => panic!("expecting namespace resolution"),
        };

        //We check if the resolve_namespace method also work properly
        match r.event_namespace(event.name(), &mut ns_buf) {
            (Some(b"www1"), _) => (),
            (Some(_), _) => panic!("expecting namespace to resolve to 'www1'"),
            ns => panic!(
                "expecting namespace resolution by the resolve_nemespace method {:?}",
                ns
            ),
        }
    }

    // </b>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((Some(b"www1"), End(_))) => (),
        Ok((Some(_), End(_))) => panic!("expecting namespace to resolve to 'www1'"),
        _ => panic!("expecting namespace resolution"),
    }

    // </a> very important: a should not be in any namespace. The default namespace only applies to
    // the sub-document it is defined on.
    if let Ok((None, End(_))) = r.read_namespaced_event(&mut buf, &mut ns_buf) {
    } else {
        panic!("expecting outer end element with no namespace");
    }
}

#[cfg(feature = "serialize")]
#[test]
fn line_score() {
    #[derive(Debug, PartialEq, Deserialize)]
    struct LineScoreData {
        game_pk: u32,
        game_type: char,
        venue: String,
        venue_w_chan_loc: String,
        venue_id: u32,
        time: String,
        time_zone: String,
        ampm: String,
        home_team_id: u32,
        home_team_city: String,
        home_team_name: String,
        home_league_id: u32,
        away_team_id: u32,
        away_team_city: String,
        away_team_name: String,
        away_league_id: u32,
        #[serde(rename = "linescore", skip_serializing)]
        innings: Vec<LineScore>,
    }
    #[derive(Debug, PartialEq, Deserialize)]
    struct LineScore {
        #[serde(rename = "away_inning_runs")]
        away_runs: u32,
        #[serde(rename = "home_inning_runs")]
        //needs to be an Option, since home team doesn't always bat.
        home_runs: Option<u32>,
        // Keeping the inning as a string, since we'll need it to construct URLs later
        inning: String,
    }

    let res: LineScoreData = quick_xml::de::from_str(include_str!("linescore.xml")).unwrap();

    let expected = LineScoreData {
        game_pk: 239575,
        game_type: 'R',
        venue: "Generic".to_owned(),
        venue_w_chan_loc: "USNY0996".to_owned(),
        venue_id: 401,
        time: "Gm 2".to_owned(),
        time_zone: "ET".to_owned(),
        ampm: "AM".to_owned(),
        home_team_id: 611,
        home_team_city: "DSL Dodgers".to_owned(),
        home_team_name: "DSL Dodgers".to_owned(),
        home_league_id: 130,
        away_team_id: 604,
        away_team_city: "DSL Blue Jays1".to_owned(),
        away_team_name: "DSL Blue Jays1".to_owned(),
        away_league_id: 130,
        innings: vec![
            LineScore {
                away_runs: 1,
                home_runs: Some(0),
                inning: "1".to_owned(),
            },
            LineScore {
                away_runs: 0,
                home_runs: Some(0),
                inning: "2".to_owned(),
            },
            LineScore {
                away_runs: 1,
                home_runs: Some(1),
                inning: "3".to_owned(),
            },
            LineScore {
                away_runs: 2,
                home_runs: Some(0),
                inning: "4".to_owned(),
            },
            LineScore {
                away_runs: 0,
                home_runs: Some(0),
                inning: "5".to_owned(),
            },
            LineScore {
                away_runs: 0,
                home_runs: Some(0),
                inning: "6".to_owned(),
            },
            LineScore {
                away_runs: 0,
                home_runs: Some(0),
                inning: "7".to_owned(),
            },
        ],
    };
    assert_eq!(res, expected);
}

#[cfg(feature = "serialize")]
#[test]
fn players() {
    #[derive(PartialEq, Deserialize, Serialize, Debug)]
    struct Game {
        #[serde(rename = "team")]
        teams: Vec<Team>,
        //umpires: Umpires
    }

    #[derive(PartialEq, Deserialize, Serialize, Debug)]
    struct Team {
        #[serde(rename = "type")]
        home_away: HomeAway,
        id: String,
        name: String,
        #[serde(rename = "player")]
        players: Vec<Player>,
        #[serde(rename = "coach")]
        coaches: Vec<Coach>,
    }

    #[derive(PartialEq, Deserialize, Serialize, Debug)]
    enum HomeAway {
        #[serde(rename = "home")]
        Home,
        #[serde(rename = "away")]
        Away,
    }

    #[derive(PartialEq, Deserialize, Serialize, Debug, Clone)]
    struct Player {
        id: u32,
        #[serde(rename = "first")]
        name_first: String,
        #[serde(rename = "last")]
        name_last: String,
        game_position: Option<String>,
        bat_order: Option<u8>,
        position: String,
    }

    #[derive(PartialEq, Deserialize, Serialize, Debug)]
    struct Coach {
        position: String,
        #[serde(rename = "first")]
        name_first: String,
        #[serde(rename = "last")]
        name_last: String,
        id: u32,
    }

    let res: Game = quick_xml::de::from_str(include_str!("players.xml")).unwrap();

    let expected = Game {
        teams: vec![
            Team {
                home_away: HomeAway::Away,
                id: "CIN".to_owned(),
                name: "Cincinnati Reds".to_owned(),
                players: vec![
                    Player {
                        id: 115135,
                        name_first: "Ken".to_owned(),
                        name_last: "Griffey".to_owned(),
                        game_position: Some("RF".to_owned()),
                        bat_order: Some(3),
                        position: "RF".to_owned(),
                    },
                    Player {
                        id: 115608,
                        name_first: "Scott".to_owned(),
                        name_last: "Hatteberg".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "1B".to_owned(),
                    },
                    Player {
                        id: 118967,
                        name_first: "Kent".to_owned(),
                        name_last: "Mercker".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 136460,
                        name_first: "Alex".to_owned(),
                        name_last: "Gonzalez".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "SS".to_owned(),
                    },
                    Player {
                        id: 150020,
                        name_first: "Jerry".to_owned(),
                        name_last: "Hairston".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "SS".to_owned(),
                    },
                    Player {
                        id: 150188,
                        name_first: "Francisco".to_owned(),
                        name_last: "Cordero".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 150221,
                        name_first: "Mike".to_owned(),
                        name_last: "Lincoln".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 150319,
                        name_first: "Josh".to_owned(),
                        name_last: "Fogg".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 150472,
                        name_first: "Ryan".to_owned(),
                        name_last: "Freel".to_owned(),
                        game_position: Some("LF".to_owned()),
                        bat_order: Some(2),
                        position: "CF".to_owned(),
                    },
                    Player {
                        id: 276520,
                        name_first: "Bronson".to_owned(),
                        name_last: "Arroyo".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 279571,
                        name_first: "Matt".to_owned(),
                        name_last: "Belisle".to_owned(),
                        game_position: Some("P".to_owned()),
                        bat_order: Some(9),
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 279913,
                        name_first: "Corey".to_owned(),
                        name_last: "Patterson".to_owned(),
                        game_position: Some("CF".to_owned()),
                        bat_order: Some(1),
                        position: "CF".to_owned(),
                    },
                    Player {
                        id: 346793,
                        name_first: "Jeremy".to_owned(),
                        name_last: "Affeldt".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 408252,
                        name_first: "Brandon".to_owned(),
                        name_last: "Phillips".to_owned(),
                        game_position: Some("2B".to_owned()),
                        bat_order: Some(4),
                        position: "2B".to_owned(),
                    },
                    Player {
                        id: 421685,
                        name_first: "Aaron".to_owned(),
                        name_last: "Harang".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 424325,
                        name_first: "David".to_owned(),
                        name_last: "Ross".to_owned(),
                        game_position: Some("C".to_owned()),
                        bat_order: Some(8),
                        position: "C".to_owned(),
                    },
                    Player {
                        id: 429665,
                        name_first: "Edwin".to_owned(),
                        name_last: "Encarnacion".to_owned(),
                        game_position: Some("3B".to_owned()),
                        bat_order: Some(6),
                        position: "3B".to_owned(),
                    },
                    Player {
                        id: 433898,
                        name_first: "Jeff".to_owned(),
                        name_last: "Keppinger".to_owned(),
                        game_position: Some("SS".to_owned()),
                        bat_order: Some(7),
                        position: "SS".to_owned(),
                    },
                    Player {
                        id: 435538,
                        name_first: "Bill".to_owned(),
                        name_last: "Bray".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 440361,
                        name_first: "Norris".to_owned(),
                        name_last: "Hopper".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "O".to_owned(),
                    },
                    Player {
                        id: 450172,
                        name_first: "Edinson".to_owned(),
                        name_last: "Volquez".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 454537,
                        name_first: "Jared".to_owned(),
                        name_last: "Burton".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 455751,
                        name_first: "Bobby".to_owned(),
                        name_last: "Livingston".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 456501,
                        name_first: "Johnny".to_owned(),
                        name_last: "Cueto".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 458015,
                        name_first: "Joey".to_owned(),
                        name_last: "Votto".to_owned(),
                        game_position: Some("1B".to_owned()),
                        bat_order: Some(5),
                        position: "1B".to_owned(),
                    },
                ],
                coaches: vec![
                    Coach {
                        position: "manager".to_owned(),
                        name_first: "Dusty".to_owned(),
                        name_last: "Baker".to_owned(),
                        id: 110481,
                    },
                    Coach {
                        position: "batting_coach".to_owned(),
                        name_first: "Brook".to_owned(),
                        name_last: "Jacoby".to_owned(),
                        id: 116461,
                    },
                    Coach {
                        position: "pitching_coach".to_owned(),
                        name_first: "Dick".to_owned(),
                        name_last: "Pole".to_owned(),
                        id: 120649,
                    },
                    Coach {
                        position: "first_base_coach".to_owned(),
                        name_first: "Billy".to_owned(),
                        name_last: "Hatcher".to_owned(),
                        id: 115602,
                    },
                    Coach {
                        position: "third_base_coach".to_owned(),
                        name_first: "Mark".to_owned(),
                        name_last: "Berry".to_owned(),
                        id: 427028,
                    },
                    Coach {
                        position: "bench_coach".to_owned(),
                        name_first: "Chris".to_owned(),
                        name_last: "Speier".to_owned(),
                        id: 122573,
                    },
                    Coach {
                        position: "bullpen_coach".to_owned(),
                        name_first: "Juan".to_owned(),
                        name_last: "Lopez".to_owned(),
                        id: 427306,
                    },
                    Coach {
                        position: "bullpen_catcher".to_owned(),
                        name_first: "Mike".to_owned(),
                        name_last: "Stefanski".to_owned(),
                        id: 150464,
                    },
                ],
            },
            Team {
                home_away: HomeAway::Home,
                id: "NYM".to_owned(),
                name: "New York Mets".to_owned(),
                players: vec![
                    Player {
                        id: 110189,
                        name_first: "Moises".to_owned(),
                        name_last: "Alou".to_owned(),
                        game_position: Some("LF".to_owned()),
                        bat_order: Some(6),
                        position: "LF".to_owned(),
                    },
                    Player {
                        id: 112116,
                        name_first: "Luis".to_owned(),
                        name_last: "Castillo".to_owned(),
                        game_position: Some("2B".to_owned()),
                        bat_order: Some(2),
                        position: "2B".to_owned(),
                    },
                    Player {
                        id: 113232,
                        name_first: "Carlos".to_owned(),
                        name_last: "Delgado".to_owned(),
                        game_position: Some("1B".to_owned()),
                        bat_order: Some(7),
                        position: "1B".to_owned(),
                    },
                    Player {
                        id: 113702,
                        name_first: "Damion".to_owned(),
                        name_last: "Easley".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "2B".to_owned(),
                    },
                    Player {
                        id: 118377,
                        name_first: "Pedro".to_owned(),
                        name_last: "Martinez".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 123790,
                        name_first: "Billy".to_owned(),
                        name_last: "Wagner".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 133340,
                        name_first: "Orlando".to_owned(),
                        name_last: "Hernandez".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 135783,
                        name_first: "Ramon".to_owned(),
                        name_last: "Castro".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "C".to_owned(),
                    },
                    Player {
                        id: 136724,
                        name_first: "Marlon".to_owned(),
                        name_last: "Anderson".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "LF".to_owned(),
                    },
                    Player {
                        id: 136860,
                        name_first: "Carlos".to_owned(),
                        name_last: "Beltran".to_owned(),
                        game_position: Some("CF".to_owned()),
                        bat_order: Some(4),
                        position: "CF".to_owned(),
                    },
                    Player {
                        id: 150411,
                        name_first: "Brian".to_owned(),
                        name_last: "Schneider".to_owned(),
                        game_position: Some("C".to_owned()),
                        bat_order: Some(8),
                        position: "C".to_owned(),
                    },
                    Player {
                        id: 276371,
                        name_first: "Johan".to_owned(),
                        name_last: "Santana".to_owned(),
                        game_position: Some("P".to_owned()),
                        bat_order: Some(9),
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 277184,
                        name_first: "Matt".to_owned(),
                        name_last: "Wise".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 346795,
                        name_first: "Endy".to_owned(),
                        name_last: "Chavez".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "RF".to_owned(),
                    },
                    Player {
                        id: 407901,
                        name_first: "Jorge".to_owned(),
                        name_last: "Sosa".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 408230,
                        name_first: "Pedro".to_owned(),
                        name_last: "Feliciano".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 408310,
                        name_first: "Aaron".to_owned(),
                        name_last: "Heilman".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 408314,
                        name_first: "Jose".to_owned(),
                        name_last: "Reyes".to_owned(),
                        game_position: Some("SS".to_owned()),
                        bat_order: Some(1),
                        position: "SS".to_owned(),
                    },
                    Player {
                        id: 425508,
                        name_first: "Ryan".to_owned(),
                        name_last: "Church".to_owned(),
                        game_position: Some("RF".to_owned()),
                        bat_order: Some(5),
                        position: "RF".to_owned(),
                    },
                    Player {
                        id: 429720,
                        name_first: "John".to_owned(),
                        name_last: "Maine".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 431151,
                        name_first: "David".to_owned(),
                        name_last: "Wright".to_owned(),
                        game_position: Some("3B".to_owned()),
                        bat_order: Some(3),
                        position: "3B".to_owned(),
                    },
                    Player {
                        id: 434586,
                        name_first: "Ambiorix".to_owned(),
                        name_last: "Burgos".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 434636,
                        name_first: "Angel".to_owned(),
                        name_last: "Pagan".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "LF".to_owned(),
                    },
                    Player {
                        id: 450306,
                        name_first: "Jason".to_owned(),
                        name_last: "Vargas".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                    Player {
                        id: 460059,
                        name_first: "Mike".to_owned(),
                        name_last: "Pelfrey".to_owned(),
                        game_position: None,
                        bat_order: None,
                        position: "P".to_owned(),
                    },
                ],
                coaches: vec![
                    Coach {
                        position: "manager".to_owned(),
                        name_first: "Willie".to_owned(),
                        name_last: "Randolph".to_owned(),
                        id: 120927,
                    },
                    Coach {
                        position: "batting_coach".to_owned(),
                        name_first: "Howard".to_owned(),
                        name_last: "Johnson".to_owned(),
                        id: 116593,
                    },
                    Coach {
                        position: "pitching_coach".to_owned(),
                        name_first: "Rick".to_owned(),
                        name_last: "Peterson".to_owned(),
                        id: 427395,
                    },
                    Coach {
                        position: "first_base_coach".to_owned(),
                        name_first: "Tom".to_owned(),
                        name_last: "Nieto".to_owned(),
                        id: 119796,
                    },
                    Coach {
                        position: "third_base_coach".to_owned(),
                        name_first: "Sandy".to_owned(),
                        name_last: "Alomar".to_owned(),
                        id: 110185,
                    },
                    Coach {
                        position: "bench_coach".to_owned(),
                        name_first: "Jerry".to_owned(),
                        name_last: "Manuel".to_owned(),
                        id: 118262,
                    },
                    Coach {
                        position: "bullpen_coach".to_owned(),
                        name_first: "Guy".to_owned(),
                        name_last: "Conti".to_owned(),
                        id: 434699,
                    },
                    Coach {
                        position: "bullpen_catcher".to_owned(),
                        name_first: "Dave".to_owned(),
                        name_last: "Racaniello".to_owned(),
                        id: 534948,
                    },
                    Coach {
                        position: "coach".to_owned(),
                        name_first: "Sandy".to_owned(),
                        name_last: "Alomar".to_owned(),
                        id: 110184,
                    },
                    Coach {
                        position: "coach".to_owned(),
                        name_first: "Juan".to_owned(),
                        name_last: "Lopez".to_owned(),
                        id: 495390,
                    },
                ],
            },
        ],
    };

    assert_eq!(res, expected);
}
