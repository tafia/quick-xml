#![cfg(feature = "serialize")]

extern crate quick_xml;
extern crate serde;

use quick_xml::{de::from_str, se::to_string};
use serde::{Deserialize, Serialize};

type Err = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct GetBucketTaggingOutput {
    #[serde(rename = "TagSet")]
    tag_set: TagSet,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TagSet {
    #[serde(rename = "Tag")]
    tags: Vec<Tag>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Tag {
    /// Name of the tag.
    #[serde(rename = "Key")]
    key: String,
    /// Value of the tag.
    #[serde(rename = "Value")]
    value: String,
}

#[test]
fn get_bucket_tagging() -> Result<(), Err> {
    let src = " 
    <GetBucketTaggingOutput>
       <TagSet>
          <Tag>
             <Key>string</Key>
             <Value>string</Value>
          </Tag>
       </TagSet>
    </GetBucketTaggingOutput>";

    let bucket_tagging: GetBucketTaggingOutput = from_str(src).unwrap();
    assert_eq!(
        bucket_tagging,
        GetBucketTaggingOutput {
            tag_set: TagSet {
                tags: vec![Tag {
                    key: "string".to_string(),
                    value: "string".to_string(),
                }],
            }
        }
    );
    // roundtrip
    to_string(&from_str(src)?)?;
    Ok(())
}
