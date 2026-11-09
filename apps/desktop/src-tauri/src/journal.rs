//! Mise de côté du journal d'un autre protocole (ADR 0005, ADR 0007) : au démarrage, un journal
//! dont le marqueur ne correspond pas au protocole courant est renommé, jamais supprimé.

use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Protocole du journal (ADR 0005). À changer quand le format des événements ou des règles change :
/// un journal d'un autre protocole ne se relit pas et est mis de côté au démarrage.
pub const PROTOCOL: &str = "adr-0007";
/// Marqueur, dans le dossier du nœud, du protocole du journal présent.
pub const PROTOCOL_FILE: &str = "protocol";
/// Dossiers du journal : mis de côté quand le protocole change. `node.seed` n'est jamais touché.
const JOURNAL_DIRS: [&str; 2] = ["ledger", "settlement"];

/// Met de côté le journal d'un autre protocole (`ledger.old-<heure>`), puis écrit le marqueur.
/// Sans effet quand le marqueur correspond déjà à `PROTOCOL`.
pub fn set_aside_stale_journal(data_dir: &Path) -> io::Result<()> {
    let marker = data_dir.join(PROTOCOL_FILE);
    if fs::read_to_string(&marker).is_ok_and(|found| found.trim() == PROTOCOL) {
        return Ok(());
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    for name in JOURNAL_DIRS {
        let from = data_dir.join(name);
        if from.exists() {
            fs::rename(&from, data_dir.join(format!("{name}.old-{stamp}")))?;
        }
    }
    fs::write(marker, PROTOCOL)
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "tests fail loudly on purpose")]
mod tests {
    use super::*;

    #[test]
    fn a_journal_of_another_protocol_is_set_aside_once_and_the_seed_is_kept() {
        let dir = tempfile::tempdir().expect("dir");
        fs::create_dir_all(dir.path().join("ledger")).expect("ledger");
        fs::write(dir.path().join("ledger").join("segment"), b"ancien").expect("segment");
        fs::write(dir.path().join("node.seed"), b"identite").expect("seed");

        set_aside_stale_journal(dir.path()).expect("set aside");
        assert!(!dir.path().join("ledger").exists());
        assert!(dir.path().join("node.seed").exists());
        assert_eq!(
            fs::read_to_string(dir.path().join(PROTOCOL_FILE)).expect("marker"),
            PROTOCOL
        );

        // Au démarrage suivant, le journal courant n'est plus touché.
        fs::create_dir_all(dir.path().join("ledger")).expect("ledger again");
        set_aside_stale_journal(dir.path()).expect("second start");
        assert!(dir.path().join("ledger").exists());
        let archives = fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("ledger.old-")
            })
            .count();
        assert_eq!(archives, 1);
    }
}
