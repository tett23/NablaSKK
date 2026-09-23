// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/dictionary/SKKProxyDictionary.{h,cpp}
// Copyright (C) 2006-2010 Tomotaka SUWA <tomotaka.suwa@gmail.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! External skkserv dictionary (port of `SKKProxyDictionary`).

use super::file::Encoding;
use super::{CompletionHelper, Dictionary};
use crate::candidate::CandidateSuite;
use crate::entry::Entry;
use crate::jconv;

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

/// A dictionary that forwards lookups to a running skkserv. The protocol
/// is EUC-JP in the original; NablaSKK also speaks to UTF-8 servers
/// (e.g. yaskkserv2 with `--utf8`). Connection failures are silent,
/// matching the original: the dictionary just returns no candidates.
pub struct ProxyDictionary {
    address: String,
    encoding: Encoding,
    session: RefCell<Option<BufReader<TcpStream>>>,
}

impl ProxyDictionary {
    /// `location` is "host:port" or "host" (port defaults to 1178).
    /// The server is assumed to speak EUC-JP.
    pub fn new(location: &str) -> Self {
        Self::with_encoding(location, Encoding::EucJp)
    }

    /// Like `new`, with the encoding the server speaks. `Encoding::Auto`
    /// is treated as EUC-JP (the protocol default).
    pub fn with_encoding(location: &str, encoding: Encoding) -> Self {
        let address = if location.contains(':') {
            location.to_string()
        } else {
            format!("{location}:1178")
        };
        let encoding = match encoding {
            Encoding::Utf8 => Encoding::Utf8,
            Encoding::EucJp | Encoding::Auto => Encoding::EucJp,
        };

        Self { address, encoding, session: RefCell::new(None) }
    }

    fn encode(&self, text: &str) -> Vec<u8> {
        match self.encoding {
            Encoding::Utf8 => text.as_bytes().to_vec(),
            _ => jconv::eucj_from_utf8(text),
        }
    }

    fn decode(&self, bytes: &[u8]) -> String {
        match self.encoding {
            Encoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            _ => jconv::utf8_from_eucj(bytes),
        }
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
            Ok(self.decode(&response))
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
        request.extend_from_slice(&self.encode(entry.entry_string()));
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
        request.extend_from_slice(&self.encode(helper.entry()));
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
        spawn_server_with(Encoding::EucJp)
    }

    fn spawn_server_with(encoding: Encoding) -> String {
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
                let key = match encoding {
                    Encoding::Utf8 => String::from_utf8(key).unwrap(),
                    _ => jconv::utf8_from_eucj(&key),
                };

                let reply = match (cmd[0], key.as_str()) {
                    (b'1', "かんじ") => "1/漢字/幹事/\n".to_string(),
                    (b'4', "かん") => "1/かんじ/かんとう/\n".to_string(),
                    _ => format!("4{key}\n"),
                };
                let bytes = match encoding {
                    Encoding::Utf8 => reply.into_bytes(),
                    _ => jconv::eucj_from_utf8(&reply),
                };
                stream.write_all(&bytes).unwrap();
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
    fn utf8_server() {
        let address = spawn_server_with(Encoding::Utf8);
        let dictionary = ProxyDictionary::with_encoding(&address, Encoding::Utf8);

        let mut suite = CandidateSuite::new();
        dictionary.find(&Entry::from_entry("かんじ"), &mut suite);
        assert_eq!(suite.candidates().len(), 2);
        assert_eq!(suite.candidates()[0].word(), "漢字");

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
