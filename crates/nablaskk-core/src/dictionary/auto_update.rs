// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/dictionary/SKKAutoUpdateDictionary.h
//   src/engine/dictionary/SKKHttpDictionaryLoader.{h,cpp}
// Copyright (C) 2010 Tomotaka SUWA <tomotaka.suwa@gmail.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! Auto-downloading dictionary
//! (port of `SKKAutoUpdateDictionary` / `SKKHttpDictionaryLoader`).
//!
//! Fetches a SKK-JISYO over plain HTTP with `If-Modified-Since`, then
//! behaves like a `CommonDictionary` on the local copy. The original
//! polled from a timer thread; here [`AutoUpdateDictionary::update`]
//! checks on demand.

use super::common::CommonDictionary;
use super::file::Encoding;
use super::{CompletionHelper, Dictionary};
use crate::candidate::CandidateSuite;
use crate::entry::Entry;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub struct AutoUpdateDictionary {
    host: String,
    url: String,
    path: PathBuf,
    dictionary: CommonDictionary,
}

impl AutoUpdateDictionary {
    /// `location` format matches the original: "host[:port] url local-path"
    /// e.g. "openlab.jp /skk/dict/SKK-JISYO.L /path/to/SKK-JISYO.L".
    pub fn open(location: &str) -> std::io::Result<Self> {
        let mut fields = location.splitn(3, ' ');
        let (host, url, path) = match (fields.next(), fields.next(), fields.next()) {
            (Some(host), Some(url), Some(path)) if !path.is_empty() => (host, url, path),
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "expected \"host url path\"",
                ))
            }
        };

        let host = if host.contains(':') { host.to_string() } else { format!("{host}:80") };

        let mut result = Self {
            host,
            url: url.to_string(),
            path: PathBuf::from(path),
            dictionary: CommonDictionary::default(),
        };

        result.update();

        // A failed download is fine as long as a previous copy exists
        result.dictionary = CommonDictionary::open(&result.path, Encoding::Auto)?;

        Ok(result)
    }

    /// Download if the server copy is newer, then reload.
    /// Returns true when the dictionary changed.
    pub fn update(&mut self) -> bool {
        match self.download() {
            Ok(true) => {
                if let Ok(dictionary) = CommonDictionary::open(&self.path, Encoding::Auto) {
                    self.dictionary = dictionary;
                    return true;
                }
                false
            }
            Ok(false) => false,
            Err(err) => {
                eprintln!("AutoUpdateDictionary: download failed: {err}");
                false
            }
        }
    }

    fn download(&self) -> std::io::Result<bool> {
        let stream = TcpStream::connect(&self.host)?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;

        let mut reader = BufReader::new(stream);

        let mtime = std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let host_only = self.host.split(':').next().unwrap_or(&self.host);
        let request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nIf-Modified-Since: {}\r\nConnection: close\r\n\r\n",
            self.url,
            host_only,
            http_date(mtime)
        );
        reader.get_mut().write_all(request.as_bytes())?;

        // Response headers
        let mut status_line = String::new();
        reader.read_line(&mut status_line)?;

        if status_line.split_whitespace().nth(1) != Some("200") {
            return Ok(false); // 304 Not Modified, or an error status
        }

        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(value) = line
                .split_once(':')
                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map(|(_, v)| v.trim())
            {
                content_length = value.parse().unwrap_or(0);
            }
        }

        if content_length == 0 {
            return Ok(false);
        }

        // Body
        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body)?;

        let tmp_path = self.path.with_extension("download");
        std::fs::write(&tmp_path, &body)?;
        std::fs::rename(&tmp_path, &self.path)?;

        Ok(true)
    }
}

impl Dictionary for AutoUpdateDictionary {
    fn find(&self, entry: &Entry, result: &mut CandidateSuite) {
        self.dictionary.find(entry, result);
    }

    fn reverse_lookup(&self, candidate: &str) -> Option<String> {
        self.dictionary.reverse_lookup(candidate)
    }

    fn complete(&self, helper: &mut CompletionHelper) {
        self.dictionary.complete(helper);
    }
}

/// Format a SystemTime as an RFC 1123 HTTP date
/// ("Sun, 06 Nov 1994 08:49:37 GMT").
fn http_date(time: SystemTime) -> String {
    let secs = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let days = secs.div_euclid(86_400);
    let time_of_day = secs.rem_euclid(86_400);
    let (hour, minute, second) =
        (time_of_day / 3600, (time_of_day % 3600) / 60, time_of_day % 60);

    // Howard Hinnant's civil_from_days
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };

    let weekday = (days + 4).rem_euclid(7) as usize; // 1970-01-01 was Thursday

    static WEEKDAYS: &[&str] = &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    static MONTHS: &[&str] =
        &["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        WEEKDAYS[weekday],
        day,
        MONTHS[(month - 1) as usize],
        year,
        hour,
        minute,
        second
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_date_format() {
        // 784111777 = Sun, 06 Nov 1994 08:49:37 GMT (the RFC example)
        let time = SystemTime::UNIX_EPOCH + Duration::from_secs(784_111_777);
        assert_eq!(http_date(time), "Sun, 06 Nov 1994 08:49:37 GMT");

        assert_eq!(http_date(SystemTime::UNIX_EPOCH), "Thu, 01 Jan 1970 00:00:00 GMT");
    }

    #[test]
    fn download_and_lookup() {
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();

        let body = crate::jconv::eucj_from_utf8(
            ";; okuri-ari entries.\n;; okuri-nasi entries.\nかんじ /漢字/\n",
        );

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);

            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
        });

        let dir = std::env::temp_dir().join("nablaskk-core-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("auto-update.jisyo");
        std::fs::remove_file(&path).ok();

        let location = format!("{address} /SKK-JISYO.S {}", path.display());
        let dictionary = AutoUpdateDictionary::open(&location).unwrap();

        let mut suite = CandidateSuite::new();
        dictionary.find(&Entry::from_entry("かんじ"), &mut suite);
        assert_eq!(suite.candidates()[0].word(), "漢字");

        std::fs::remove_file(&path).ok();
    }
}
