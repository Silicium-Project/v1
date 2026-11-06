//! Lecture plafonnée d'un flux de processus, dans un thread.

use std::{
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
};

/// Lit `source` jusqu'à `cap` octets. Au-delà : lève `overflow` et s'arrête (l'appelant tue
/// le processus). Le résultat arrive sur le canal retourné.
pub(crate) fn spawn<R: Read + Send + 'static>(
    mut source: R,
    cap: usize,
    overflow: Arc<AtomicBool>,
) -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut data = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match source.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if data.len() + n > cap {
                        overflow.store(true, Ordering::SeqCst);
                        break;
                    }
                    data.extend_from_slice(&buf[..n]);
                }
            }
        }
        let _ = sender.send(data);
    });
    receiver
}
