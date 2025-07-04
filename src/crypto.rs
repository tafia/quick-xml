use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use sha2::{Digest as ShaDigest, Sha256, Sha512};
use signature::Signer;
use std::io::Cursor;

use crate::events::Event;
use crate::{Reader, Writer};

/// Supported digest algorithms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigestMethod {
    /// SHA256 digest
    Sha256,
    /// SHA512 digest
    Sha512,
}

/// Trait describing digest capabilities for signers.
pub trait Digest {
    /// Returns the digest algorithm used when signing.
    fn digest_method(&self) -> DigestMethod;
}

#[doc(hidden)]
pub fn canonicalize(xml: &str) -> crate::Result<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let mut elem = e.into_owned();
                let mut attrs: Vec<(Vec<u8>, Vec<u8>)> = elem
                    .attributes()
                    .map(|a| {
                        let a = a?;
                        Ok((a.key.as_ref().to_vec(), a.value.into_owned()))
                    })
                    .collect::<crate::Result<_>>()?;
                attrs.sort_by(|a, b| a.0.cmp(&b.0));
                elem.clear_attributes();
                for (k, v) in attrs {
                    elem.push_attribute((k.as_slice(), v.as_slice()));
                }
                writer.write_event(Event::Start(elem))?;
            }
            Event::Empty(e) => {
                let mut elem = e.into_owned();
                let mut attrs: Vec<(Vec<u8>, Vec<u8>)> = elem
                    .attributes()
                    .map(|a| {
                        let a = a?;
                        Ok((a.key.as_ref().to_vec(), a.value.into_owned()))
                    })
                    .collect::<crate::Result<_>>()?;
                attrs.sort_by(|a, b| a.0.cmp(&b.0));
                elem.clear_attributes();
                for (k, v) in attrs {
                    elem.push_attribute((k.as_slice(), v.as_slice()));
                }
                writer.write_event(Event::Empty(elem))?;
            }
            Event::End(e) => {
                writer.write_event(Event::End(e))?;
            }
            Event::Text(e) => {
                writer.write_event(Event::Text(e))?;
            }
            Event::CData(e) => {
                writer.write_event(Event::CData(e))?;
            }
            Event::Comment(_) => {}
            Event::Eof => break,
            evt => writer.write_event(evt)?,
        }
        buf.clear();
    }
    let vec = writer.into_inner().into_inner();
    Ok(String::from_utf8(vec).expect("canonicalized xml is valid utf-8"))
}

/// Sign the provided XML document using the given signer.
pub fn sign_document<S>(xml: &str, signer: S) -> crate::Result<String>
where
    S: Signer<Vec<u8>> + Digest,
{
    let canonical = canonicalize(xml)?;
    let digest = match signer.digest_method() {
        DigestMethod::Sha256 => Sha256::digest(canonical.as_bytes()).to_vec(),
        DigestMethod::Sha512 => Sha512::digest(canonical.as_bytes()).to_vec(),
    };
    let digest_b64 = BASE64.encode(&digest);

    let (sig_method, dig_method) = match signer.digest_method() {
        DigestMethod::Sha256 => (
            "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256",
            "http://www.w3.org/2001/04/xmlenc#sha256",
        ),
        DigestMethod::Sha512 => (
            "http://www.w3.org/2001/04/xmldsig-more#rsa-sha512",
            "http://www.w3.org/2001/04/xmlenc#sha512",
        ),
    };

    let signed_info = format!(
        "<SignedInfo>\
            <CanonicalizationMethod Algorithm=\"http://www.w3.org/2001/10/xml-exc-c14n#\"/>\
            <SignatureMethod Algorithm=\"{}\"/>\
            <Reference URI=\"\">\
                <Transforms>\
                    <Transform Algorithm=\"http://www.w3.org/2000/09/xmldsig#enveloped-signature\"/>\
                    <Transform Algorithm=\"http://www.w3.org/2001/10/xml-exc-c14n#\"/>\
                </Transforms>\
                <DigestMethod Algorithm=\"{}\"/>\
                <DigestValue>{}</DigestValue>\
            </Reference>\
        </SignedInfo>",
        sig_method, dig_method, digest_b64
    );
    let canonical_info = canonicalize(&signed_info)?;
    let signature = signer.sign(canonical_info.as_bytes());
    let signature_b64 = BASE64.encode(signature);

    let result = format!(
        "<Signature xmlns=\"http://www.w3.org/2000/09/xmldsig#\">{}<SignatureValue>{}</SignatureValue></Signature>",
        signed_info, signature_b64
    );

    Ok(result)
}
