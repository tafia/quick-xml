use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event::*;
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use quick_xml::Error;
use std::borrow::Cow;

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

use pretty_assertions::assert_eq;

#[test]
fn test_sample() {
    let src = include_str!("documents/sample_rss.xml");
    let mut r = Reader::from_str(src);
    let mut count = 0;
    loop {
        match r.read_event().unwrap() {
            Start(_) => count += 1,
            Decl(e) => println!("{:?}", e.version()),
            Eof => break,
            _ => (),
        }
    }
    println!("{}", count);
}

#[test]
fn test_attributes_empty() {
    let src = "<a att1='a' att2='b'/>";
    let mut r = Reader::from_str(src);
    r.trim_text(true).expand_empty_elements(false);
    match r.read_event() {
        Ok(Empty(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"att1"),
                    value: Cow::Borrowed(b"a"),
                }))
            );
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"att2"),
                    value: Cow::Borrowed(b"b"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_attribute_equal() {
    let src = "<a att1=\"a=b\"/>";
    let mut r = Reader::from_str(src);
    r.trim_text(true).expand_empty_elements(false);
    match r.read_event() {
        Ok(Empty(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"att1"),
                    value: Cow::Borrowed(b"a=b"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_comment_starting_with_gt() {
    let src = "<a /><!-->-->";
    let mut r = Reader::from_str(src);
    r.trim_text(true).expand_empty_elements(false);
    loop {
        match r.read_event() {
            Ok(Comment(e)) => {
                assert_eq!(e.as_ref(), b">");
                break;
            }
            Ok(Eof) => panic!("Expecting Comment"),
            _ => (),
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
    loop {
        match reader.read_event() {
            Ok(Eof) | Err(..) => break,
            _ => (),
        }
    }
}

#[test]
fn test_no_trim() {
    let mut reader = Reader::from_str(" <tag> text </tag> ");

    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), Start(_)));
    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), End(_)));
    assert!(matches!(reader.read_event().unwrap(), Text(_)));
}

#[test]
fn test_trim_end() {
    let mut reader = Reader::from_str(" <tag> text </tag> ");
    reader.trim_text_end(true);

    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), Start(_)));
    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), End(_)));
}

#[test]
fn test_trim() {
    let mut reader = Reader::from_str(" <tag> text </tag> ");
    reader.trim_text(true);

    assert!(matches!(reader.read_event().unwrap(), Start(_)));
    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), End(_)));
}

#[test]
fn test_clone_reader() {
    let mut reader = Reader::from_str("<tag>text</tag>");
    reader.trim_text(true);

    assert!(matches!(reader.read_event().unwrap(), Start(_)));

    let mut cloned = reader.clone();

    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), End(_)));

    assert!(matches!(cloned.read_event().unwrap(), Text(_)));
    assert!(matches!(cloned.read_event().unwrap(), End(_)));
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

    let res: LineScoreData =
        quick_xml::de::from_str(include_str!("documents/linescore.xml")).unwrap();

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

    let res: Game = quick_xml::de::from_str(include_str!("documents/players.xml")).unwrap();

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

#[test]
fn test_issue299() -> Result<(), Error> {
    let xml = r#"
<?xml version="1.0" encoding="utf8"?>
<MICEX_DOC xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <SECURITY SecurityId="PLZL" ISIN="RU000A0JNAA8" SecShortName="Short Name" PriceType="CASH">
    <RECORDS RecNo="1" TradeNo="1111" TradeDate="2021-07-08" TradeTime="15:00:00" BuySell="S" SettleCode="Y1Dt" Decimals="3" Price="13057.034" Quantity="766" Value="10001688.29" AccInt="0" Amount="10001688.29" Balance="766" TrdAccId="X0011" ClientDetails="2222" CPFirmId="3333" CPFirmShortName="Firm Short Name" Price2="13057.034" RepoPart="2" ReportTime="16:53:27" SettleTime="17:47:06" ClientCode="4444" DueDate="2021-07-09" EarlySettleStatus="N" RepoRate="5.45" RateType="FIX"/>
  </SECURITY>
</MICEX_DOC>
"#;
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event()? {
            Start(e) | Empty(e) => {
                let attr_count = match e.name().as_ref() {
                    b"MICEX_DOC" => 1,
                    b"SECURITY" => 4,
                    b"RECORDS" => 26,
                    _ => unreachable!(),
                };
                assert_eq!(
                    attr_count,
                    e.attributes().filter(Result::is_ok).count(),
                    "mismatch att count on '{:?}'",
                    reader.decoder().decode(e.name().as_ref())
                );
            }
            Eof => break,
            _ => (),
        }
    }
    Ok(())
}

#[cfg(feature = "serialize")]
#[test]
fn test_issue305_unflatten_namespace() -> Result<(), quick_xml::DeError> {
    use quick_xml::de::from_str;

    #[derive(Deserialize, Debug, PartialEq)]
    struct NamespaceBug {
        #[serde(rename = "$unflatten=d:test2")]
        test2: String,
    }

    let namespace_bug: NamespaceBug = from_str(
        r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <d:test xmlns:d="works">
        <d:test2>doesntwork</d:test2>
    </d:test>"#,
    )?;

    assert_eq!(
        namespace_bug,
        NamespaceBug {
            test2: "doesntwork".into(),
        }
    );

    Ok(())
}

#[cfg(feature = "serialize")]
#[test]
fn test_issue305_unflatten_nesting() -> Result<(), quick_xml::DeError> {
    use quick_xml::de::from_str;

    #[derive(Deserialize, Debug, PartialEq)]
    struct InnerNestingBug {}

    #[derive(Deserialize, Debug, PartialEq)]
    struct NestingBug {
        // comment out one of these fields and it works
        #[serde(rename = "$unflatten=outer1")]
        outer1: InnerNestingBug,

        #[serde(rename = "$unflatten=outer2")]
        outer2: String,
    }

    let nesting_bug: NestingBug = from_str::<NestingBug>(
        r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <root>
        <outer1></outer1>
        <outer2></outer2>
    </root>"#,
    )?;

    assert_eq!(
        nesting_bug,
        NestingBug {
            outer1: InnerNestingBug {},
            outer2: "".into(),
        }
    );

    Ok(())
}
