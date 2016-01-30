use error::{Error, Result};

pub struct Attributes<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Attributes<'a> {
    pub fn new(buf: &'a [u8], pos: usize) -> Attributes<'a> {
        Attributes {
            bytes: buf,
            position: pos,
        }
    }
}

impl<'a> Iterator for Attributes<'a> {
    type Item = Result<(&'a[u8], &'a str)>;
    fn next(&mut self) -> Option<Self::Item> {
        
        let len = self.bytes.len();
        let p = self.position;
        let mut iter = self.bytes[p..].iter().cloned().enumerate();

        let start_key = {
            let mut found_space = false;
            let p: usize;
            loop {
                match iter.next() {
                    Some((_, b' '))
                        | Some((_, b'\r')) 
                        | Some((_, b'\n'))
                        | Some((_, b'\t')) => if !found_space { found_space = true; },
                    Some((i, _)) => if found_space { 
                        p = i;
                        break;
                    },
                    None => {
                        self.position = len;
                        return None;
                    }
                }
            }
            p
        };

        let mut has_equal = false;
        let mut end_key = None;
        let mut start_val = None;
        let mut end_val = None;
        loop {
            match iter.next() {
                Some((i, b' '))
                    | Some((i, b'\r')) 
                    | Some((i, b'\n'))
                    | Some((i, b'\t')) => {
                    if end_key.is_none() { end_key = Some(i); }
                },
                Some((i, b'=')) => {
                    if has_equal {
                        debug!("has_equal x2 !");
                        return None; // TODO: return error instead
                    }
                    has_equal = true;
                    if end_key.is_none() {
                        end_key = Some(i);
                    }
                },
                Some((i, b'"')) => {
                    if !has_equal {
                        return Some(Err(Error::Malformed("Unexpected quote before '='".to_owned())));
                    }
                    if start_val.is_none() {
                        start_val = Some(i + 1);
                    } else if end_val.is_none() {
                        end_val = Some(i);
                        break;
                    }
                },
                Some((_, _)) => (),
                None => {
                    self.position = len;
                    return None;
                }
            }
        }
        self.position = end_val.unwrap() + 1;

        match ::std::str::from_utf8(&self.bytes[(p + start_val.unwrap())..(p + end_val.unwrap())]) {
            Ok(s) => Some(Ok((&self.bytes[(p + start_key)..(p + end_key.unwrap())], s))),
            Err(e) => Some(Err(Error::from(e))),
        }
    }
}
