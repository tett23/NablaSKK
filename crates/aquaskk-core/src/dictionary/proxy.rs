//! External skkserv dictionary (port of `SKKProxyDictionary`).

use super::{CompletionHelper, Dictionary};
use crate::candidate::CandidateSuite;
use crate::entry::Entry;
use crate::jconv;

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

/// A dictionary that forwards lookups to a running skkserv
/// (protocol is EUC-JP). Connection failures are silent, matching the
/// original: the dictionary just returns no candidates.
pub struct ProxyDictionary {
    address: String,
    session: RefCell<Option<BufReader<TcpStream>>>,
}

impl ProxyDictionary {
    /// `location` is "host:port" or "host" (port defaults to 1178).
    pub fn new(location: &str) -> Self {
        let address = if location.contains(':') {
            location.to_string()
        } else {
            format!("{location}:1178")
        };

        Self { address, session: RefCell::new(None) }
    }

    fn connect(&self) -> bool {
        let mut session = self.session.borrow_mut();

        if session.is_none() {
            match TcpStream::connect(&self.address) {
                Ok(stream) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    *session = Some(BufReader::new(stream));
                }
                Err(_) => return false,
            }
        }

        true
    }

    fn roundtrip(&self, request: &[u8]) -> Option<String> {
        if !self.connect() {
            return None;
        }

        let mut session = self.session.borrow_mut();
        let reader = session.as_mut()?;

        let io = (|| -> std::io::Result<String> {
            reader.get_mut().write_all(request)?;
            reader.get_mut().flush()?;

            let mut response = Vec::new();
            reader.read_until(b'\n', &mut response)?;
            Ok(jconv::utf8_from_eucj(&response))
        })();

        match io {
            Ok(response) => Some(response),
            Err(_) => {
                // Drop the broken connection; the next lookup reconnects
                *session = None;
                None
            }
        }
    }
}

impl Dictionary for ProxyDictionary {
    fn find(&self, entry: &Entry, result: &mut CandidateSuite) {
        let mut request = vec![b'1'];
        request.extend_from_slice(&jconv::eucj_from_utf8(entry.entry_string()));
        request.push(b' ');

        let Some(response) = self.roundtrip(&request) else { return };

        let response = response.trim_end();
        if response.len() < 2 || !response.starts_with('1') {
            return;
        }

        let suite = CandidateSuite::from_line(&response[1..]);
        result.add_suite(&suite);
    }

    fn complete(&self, helper: &mut CompletionHelper) {
        let mut request = vec![b'4'];
        request.extend_from_slice(&jconv::eucj_from_utf8(helper.entry()));
        request.push(b' ');

        let Some(response) = self.roundtrip(&request) else { return };

        let response = response.trim_end();
        if response.len() < 2 || !response.starts_with('1') {
            return;
        }

        for completion in response[1..].split('/').filter(|c| !c.is_empty()) {
            helper.add(completion);

            if !helper.can_continue() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpListener;

    /// Minimal in-process skkserv for the test.
    fn spawn_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            loop {
                let mut cmd = [0u8; 1];
                if stream.read_exact(&mut cmd).is_err() || cmd[0] == b'0' {
                    return;
                }

                let mut key = Vec::new();
                let mut byte = [0u8; 1];
                while stream.read_exact(&mut byte).is_ok() && byte[0] != b' ' {
                    key.push(byte[0]);
                }
                let key = jconv::utf8_from_eucj(&key);

                let reply = match (cmd[0], key.as_str()) {
                    (b'1', "かんじ") => "1/漢字/幹事/\n".to_string(),
                    (b'4', "かん") => "1/かんじ/かんとう/\n".to_string(),
                    _ => format!("4{key}\n"),
                };
                stream.write_all(&jconv::eucj_from_utf8(&reply)).unwrap();
            }
        });

        address
    }

    #[test]
    fn lookup_and_complete() {
        let address = spawn_server();
        let dictionary = ProxyDictionary::new(&address);

        let mut suite = CandidateSuite::new();
        dictionary.find(&Entry::from_entry("かんじ"), &mut suite);
        assert_eq!(suite.candidates().len(), 2);
        assert_eq!(suite.candidates()[0].word(), "漢字");

        let mut suite = CandidateSuite::new();
        dictionary.find(&Entry::from_entry("みつからない"), &mut suite);
        assert!(suite.is_empty());

        let mut helper = CompletionHelper::new("かん", 0, 0);
        dictionary.complete(&mut helper);
        assert_eq!(helper.into_result(), vec!["かんじ", "かんとう"]);
    }

    #[test]
    fn unreachable_server() {
        // A port from the reserved TEST-NET range that nothing listens on
        let dictionary = ProxyDictionary::new("127.0.0.1:1");

        let mut suite = CandidateSuite::new();
        dictionary.find(&Entry::from_entry("かんじ"), &mut suite);
        assert!(suite.is_empty());
    }
}
