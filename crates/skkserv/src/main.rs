//! skkserv: SKK dictionary server speaking the skkserv protocol
//! (port of AquaSKK's built-in skkserv, as a standalone binary).
//!
//! Protocol (requests and responses are EUC-JP):
//!   '0'          disconnect
//!   '1' key ' '  lookup     -> "1/cand1/cand2/\n" or "4key\n"
//!   '2'          version    -> "rust-skkserv/x.y.z "
//!   '3'          host info  -> "host:addr: "
//!   '4' key ' '  completion -> "1/comp1/comp2/\n" or "4key\n"

use aquaskk_core::backend::Backend;
use aquaskk_core::candidate::CandidateSuite;
use aquaskk_core::dictionary::{CommonDictionary, Encoding, LocalUserDictionary};
use aquaskk_core::entry::Entry;
use aquaskk_core::jconv;

use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

const VERSION: &str = concat!("rust-skkserv/", env!("CARGO_PKG_VERSION"), " ");

fn usage() -> ! {
    eprintln!(
        "usage: skkserv [options] DICTIONARY...

Serves SKK dictionaries over the skkserv protocol.

options:
  -p, --port PORT       port to listen on (default: 1178)
  -l, --listen ADDR     address to bind (default: 127.0.0.1)
  -u, --utf8            treat subsequent dictionaries as UTF-8
                        (default: EUC-JP)
  -h, --help            show this help

example:
  skkserv -p 1178 /usr/share/skk/SKK-JISYO.L"
    );
    std::process::exit(1);
}

fn main() {
    let mut port = 1178u16;
    let mut listen = "127.0.0.1".to_string();
    let mut encoding = Encoding::EucJp;
    let mut dictionaries: Vec<(String, Encoding)> = Vec::new();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-p" | "--port" => match args.next().and_then(|v| v.parse().ok()) {
                Some(value) => port = value,
                None => usage(),
            },
            "-l" | "--listen" => match args.next() {
                Some(value) => listen = value,
                None => usage(),
            },
            "-u" | "--utf8" => encoding = Encoding::Utf8,
            "-h" | "--help" => usage(),
            path if !path.starts_with('-') => dictionaries.push((path.to_string(), encoding)),
            _ => usage(),
        }
    }

    if dictionaries.is_empty() {
        usage();
    }

    // No user dictionary: the server is read-only. A throwaway path keeps
    // the backend constructible without touching the filesystem.
    let scratch = std::env::temp_dir().join(format!("skkserv-user-{}.jisyo", std::process::id()));
    let mut backend = Backend::new(LocalUserDictionary::open(&scratch, Encoding::Utf8));

    for (path, encoding) in &dictionaries {
        match CommonDictionary::open(path, *encoding) {
            Ok(dictionary) => {
                eprintln!("skkserv: loaded {path}");
                backend.add_dictionary(Box::new(dictionary));
            }
            Err(err) => {
                eprintln!("skkserv: can't load {path}: {err}");
                std::process::exit(1);
            }
        }
    }

    backend.enable_extended_completion(true);

    let backend = Arc::new(Mutex::new(backend));

    let address = format!("{listen}:{port}");
    let listener = match TcpListener::bind(&address) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("skkserv: can't listen on {address}: {err}");
            std::process::exit(1);
        }
    };

    eprintln!("skkserv: listening on {address}");

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let backend = Arc::clone(&backend);

        std::thread::spawn(move || {
            let peer = stream
                .peer_addr()
                .map(|a| a.to_string())
                .unwrap_or_else(|_| "?".to_string());
            eprintln!("skkserv: session start [{peer}]");

            if let Err(err) = serve(stream, &backend) {
                eprintln!("skkserv: session error [{peer}]: {err}");
            }

            eprintln!("skkserv: session finish [{peer}]");
        });
    }
}

/// Read one space-terminated EUC-JP token.
fn read_token(reader: &mut impl Read) -> std::io::Result<Vec<u8>> {
    let mut token = Vec::new();
    let mut byte = [0u8; 1];

    loop {
        if reader.read(&mut byte)? == 0 {
            break;
        }
        match byte[0] {
            b' ' | b'\n' | b'\r' => break,
            other => token.push(other),
        }
    }

    Ok(token)
}

fn serve(stream: TcpStream, backend: &Mutex<Backend>) -> std::io::Result<()> {
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);

    loop {
        let mut cmd = [0u8; 1];
        if reader.read(&mut cmd)? == 0 {
            return Ok(());
        }

        match cmd[0] {
            b'0' => return Ok(()),

            b'1' => {
                let word = read_token(&mut reader)?;
                let key = jconv::utf8_from_eucj(&word);

                // A trailing [a-z] on a non-ASCII key marks okuri-ari
                let entry = if key.chars().count() > 1
                    && !key.starts_with(|c: char| c.is_ascii())
                    && key.ends_with(|c: char| c.is_ascii_alphabetic())
                {
                    Entry::new(&key, "dummy")
                } else {
                    Entry::from_entry(&key)
                };

                let mut suite = CandidateSuite::new();
                backend.lock().unwrap().find(&entry, &mut suite);

                if suite.is_empty() {
                    writer.write_all(b"4")?;
                    writer.write_all(&word)?;
                    writer.write_all(b"\n")?;
                } else {
                    let candidates = jconv::eucj_from_utf8(&suite.to_line(true));
                    writer.write_all(b"1")?;
                    writer.write_all(&candidates)?;
                    writer.write_all(b"\n")?;
                }
                writer.flush()?;
            }

            b'2' => {
                writer.write_all(VERSION.as_bytes())?;
                writer.flush()?;
            }

            b'3' => {
                writer.write_all(b"127.0.0.1:0.0.0.0: ")?;
                writer.flush()?;
            }

            b'4' => {
                let word = read_token(&mut reader)?;
                let key = jconv::utf8_from_eucj(&word);

                let completions = backend.lock().unwrap().complete(&key, 0);

                if completions.is_empty() {
                    writer.write_all(b"4")?;
                    writer.write_all(&word)?;
                    writer.write_all(b"\n")?;
                } else {
                    let mut line = String::from("/");
                    for completion in &completions {
                        line += completion;
                        line.push('/');
                    }
                    writer.write_all(b"1")?;
                    writer.write_all(&jconv::eucj_from_utf8(&line))?;
                    writer.write_all(b"\n")?;
                }
                writer.flush()?;
            }

            other => {
                eprintln!("skkserv: unknown command [{:#04x}]", other);
                writer.write_all(b"0")?;
                writer.flush()?;
            }
        }
    }
}
