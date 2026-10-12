// skkserv integration tests (ADR 0008): run the built server on a free
// port and talk the skkserv protocol to it, with EUC-JP and UTF-8
// dictionaries, and connect the engine's ProxyDictionary to it.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

use nablaskk_core::candidate::CandidateSuite;
use nablaskk_core::dictionary::{CompletionHelper, Dictionary, ProxyDictionary};
use nablaskk_core::entry::Entry;
use nablaskk_core::jconv;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const DICTIONARY: &str = ";; okuri-ari entries.
かえs /返/帰/
;; okuri-nasi entries.
かんじ /漢字/幹事;かんじ（名）/
かんとう /関東/
";

/// A running skkserv, killed when dropped.
struct Server {
    child: Child,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("skkserv-test-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write the test dictionary in EUC-JP or UTF-8.
fn dictionary(name: &str, euc: bool) -> PathBuf {
    let path = scratch(name).join("SKK-JISYO.test");
    let bytes = if euc { jconv::eucj_from_utf8(DICTIONARY) } else { DICTIONARY.as_bytes().to_vec() };
    std::fs::write(&path, bytes).unwrap();
    path
}

/// Start skkserv with `args` on a port the OS picked, and wait until it accepts.
fn start(args: &[&str]) -> Server {
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let child = Command::new(env!("CARGO_BIN_EXE_skkserv"))
        .args(["-p", &port.to_string(), "-l", "127.0.0.1"])
        .args(args)
        .stderr(Stdio::null())
        .spawn()
        .expect("skkserv starts");
    let server = Server { child, port };
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return server;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("skkserv did not start listening on {port}");
}

/// One protocol round trip. `request` is UTF-8; the wire is EUC-JP.
fn ask(server: &Server, request: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    stream.write_all(&jconv::eucj_from_utf8(request)).unwrap();
    let mut reader = BufReader::new(stream);
    let mut reply = Vec::new();
    if request.starts_with('2') || request.starts_with('3') {
        // Version and host replies end with a space, not a newline
        let mut byte = [0u8; 1];
        while reader.read(&mut byte).unwrap() == 1 {
            reply.push(byte[0]);
            if byte[0] == b' ' {
                break;
            }
        }
    } else {
        reader.read_until(b'\n', &mut reply).unwrap();
    }
    jconv::utf8_from_eucj(&reply)
}

fn check_lookups(server: &Server) {
    assert_eq!(ask(server, "1かんじ "), "1/漢字/幹事;かんじ（名）/\n", "found, annotation kept");
    assert_eq!(ask(server, "1みつからない "), "4みつからない\n", "not found");
    assert_eq!(ask(server, "1かえs "), "1/返/帰/\n", "okuri-ari");
    assert_eq!(ask(server, "4かん "), "1/かんじ/かんとう/\n", "completion");
    assert_eq!(ask(server, "4ぬ "), "4ぬ\n", "no completion");
}

#[test]
fn euc_jp_dictionary_auto_detected() {
    let server = start(&[dictionary("euc", true).to_str().unwrap()]);
    check_lookups(&server);
}

#[test]
fn utf8_dictionary_auto_detected() {
    let server = start(&[dictionary("utf8", false).to_str().unwrap()]);
    check_lookups(&server);
}

#[test]
fn forced_encodings() {
    // -e reads the EUC-JP file correctly; -u on the same file finds nothing
    let path = dictionary("forced", true);
    let euc = start(&["-e", path.to_str().unwrap()]);
    assert_eq!(ask(&euc, "1かんじ "), "1/漢字/幹事;かんじ（名）/\n");
    let wrong = start(&["-u", path.to_str().unwrap()]);
    assert!(ask(&wrong, "1かんじ ").starts_with('4'), "UTF-8 forced on an EUC-JP file finds nothing");
}

#[test]
fn version_host_and_close() {
    let server = start(&[dictionary("misc", false).to_str().unwrap()]);
    assert!(ask(&server, "2").starts_with("rust-skkserv/"), "version");
    assert_eq!(ask(&server, "3"), "127.0.0.1:0.0.0.0: ", "host");

    // "0" closes the connection
    let mut stream = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    stream.write_all(b"0").unwrap();
    let mut rest = Vec::new();
    assert_eq!(stream.read_to_end(&mut rest).unwrap(), 0, "closed without a reply");
}

#[test]
fn several_requests_on_one_connection() {
    let server = start(&[dictionary("session", false).to_str().unwrap()]);
    let mut stream = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    for (request, expected) in [("1かんじ ", "1/漢字/幹事;かんじ（名）/\n"), ("1かんとう ", "1/関東/\n")] {
        stream.write_all(&jconv::eucj_from_utf8(request)).unwrap();
        let mut reply = Vec::new();
        reader.read_until(b'\n', &mut reply).unwrap();
        assert_eq!(jconv::utf8_from_eucj(&reply), expected);
    }
}

#[test]
fn engine_proxy_dictionary_against_the_real_server() {
    let server = start(&[dictionary("proxy", true).to_str().unwrap()]);
    let proxy = ProxyDictionary::new(&format!("127.0.0.1:{}", server.port));

    let mut suite = CandidateSuite::new();
    proxy.find(&Entry::from_entry("かんじ"), &mut suite);
    let words: Vec<&str> = suite.candidates().iter().map(|c| c.word()).collect();
    assert_eq!(words, vec!["漢字", "幹事"]);
    assert_eq!(suite.candidates()[1].annotation(), "かんじ（名）");

    let mut helper = CompletionHelper::new("かん", 0, 0);
    proxy.complete(&mut helper);
    assert_eq!(helper.into_result(), vec!["かんじ", "かんとう"]);
}
