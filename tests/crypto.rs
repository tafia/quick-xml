use quick_xml::crypto::canonicalize;
use quick_xml::crypto::{sign_document, Digest, DigestMethod};
use quick_xml::Signer;
use rand_core::OsRng;
use rsa::pkcs1v15::SigningKey;
use rsa::RsaPrivateKey;
use sha2::{Sha256, Sha512};

struct Sha256Signer(SigningKey<Sha256>);

impl Digest for Sha256Signer {
    fn digest_method(&self) -> DigestMethod {
        DigestMethod::Sha256
    }
}

impl Signer<Vec<u8>> for Sha256Signer {
    fn try_sign(&self, msg: &[u8]) -> signature::Result<Vec<u8>> {
        Ok(self.0.sign(msg).as_ref().to_vec())
    }
}

struct Sha512Signer(SigningKey<Sha512>);

impl Digest for Sha512Signer {
    fn digest_method(&self) -> DigestMethod {
        DigestMethod::Sha512
    }
}

impl Signer<Vec<u8>> for Sha512Signer {
    fn try_sign(&self, msg: &[u8]) -> signature::Result<Vec<u8>> {
        Ok(self.0.sign(msg).as_ref().to_vec())
    }
}

#[test]
fn sign_sha256() {
    let mut rng = OsRng;
    let key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
    let signer = Sha256Signer(SigningKey::<Sha256>::new(key));
    let xml = "<test>hello</test>";
    let sig = sign_document(xml, signer).unwrap();
    assert!(sig.contains("SignatureValue"));
}

#[test]
fn sign_sha512() {
    let mut rng = OsRng;
    let key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
    let signer = Sha512Signer(SigningKey::<Sha512>::new(key));
    let xml = "<test>hello</test>";
    let sig = sign_document(xml, signer).unwrap();
    assert!(sig.contains("rsa-sha512"));
}

#[test]
fn canonicalization_sorts_attrs_and_removes_comments() {
    let xml = r#"<e b="2" a="1"><!--c--><child/></e>"#;
    let expected = r#"<e a="1" b="2"><child/></e>"#;
    let out = canonicalize(xml).unwrap();
    assert_eq!(out, expected);
}
