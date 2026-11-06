use super::support::*;
use crate::*;

#[test]
fn token_files_are_validated() {
    assert!(TokenSet::parse("read:short").is_err(), "secret trop court");
    assert!(
        TokenSet::parse("admin:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").is_err(),
        "scope inconnu"
    );
    assert!(TokenSet::parse("pas de deux points").is_err());
    assert!(TokenSet::parse("# rien\n\n").unwrap().is_empty());
    let set = TokenSet::parse(&format!("{READER}\n{SUBMITTER}")).unwrap();
    let who = set
        .authenticate(Some(&format!("Bearer {}", secret(SUBMITTER))))
        .unwrap();
    assert!(who.allows(Scope::Submit) && !who.allows(Scope::Read));
    assert!(
        set.authenticate(Some(secret(READER))).is_none(),
        "préfixe Bearer obligatoire"
    );
}

#[test]
fn a_token_file_is_created_once_with_a_strong_secret() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("control.token");
    let first = TokenSet::load_or_create(&path).unwrap();
    assert!(!first.is_empty());
    let text = std::fs::read_to_string(&path).unwrap();
    let secret = text
        .lines()
        .find(|l| l.starts_with("read,submit:"))
        .unwrap();
    let secret = secret.split_once(':').unwrap().1.to_string();
    assert_eq!(secret.len(), 64);
    let again = TokenSet::load_or_create(&path).unwrap();
    assert!(
        again
            .authenticate(Some(&format!("Bearer {secret}")))
            .is_some()
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        text,
        "jamais réécrit"
    );
}
