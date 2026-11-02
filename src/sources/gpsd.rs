//! Fahrzeugstandort vom gpsd des Routers: JSON-Protokoll über TCP (Port 2947).
//!
//! Die Verbindung bleibt offen. gpsd schließt das GPS-Gerät, sobald kein Client mehr lauscht; nach
//! jedem Neuaufbau käme erst wieder ein Kaltstart ohne Fix.

use std::net::SocketAddr;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use super::SourceError;
use crate::contract::Position;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Eine gpsd-Zeile hat wenige hundert Bytes; mehr ist kein gpsd.
const MAX_LINE: u64 = 64 * 1024;
const WATCH: &[u8] = b"?WATCH={\"enable\":true,\"json\":true}\n";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Report {
    Fix(Position),
    /// TPV ohne 2D/3D-Fix (kein Himmel, Kaltstart).
    NoFix,
}

#[derive(Deserialize)]
struct Tpv {
    class: String,
    #[serde(default)]
    mode: u8,
    lat: Option<f64>,
    lon: Option<f64>,
}

/// `None` für alles außer TPV (VERSION, DEVICES, WATCH, SKY …) und für Unlesbares.
pub fn parse(line: &str) -> Option<Report> {
    let tpv: Tpv = serde_json::from_str(line).ok()?;
    if tpv.class != "TPV" {
        return None;
    }
    Some(match (tpv.mode, tpv.lat, tpv.lon) {
        (2.., Some(lat), Some(lon))
            if lat.is_finite() && lon.is_finite() && lat.abs() <= 90.0 && lon.abs() <= 180.0 =>
        {
            Report::Fix(Position { lat, lon })
        }
        _ => Report::NoFix,
    })
}

pub struct Connection {
    reader: BufReader<TcpStream>,
    /// Mit Gerät meldet gpsd jede Sekunde; so lange Stille heißt: Verbindung tot.
    read_timeout: Duration,
}

impl Connection {
    pub async fn open(addr: SocketAddr, read_timeout: Duration) -> Result<Self, SourceError> {
        let mut stream = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(addr))
            .await
            .map_err(|_| SourceError::Http(format!("gpsd {addr}: keine Antwort")))?
            .map_err(|e| SourceError::Http(format!("gpsd {addr}: {e}")))?;
        stream
            .write_all(WATCH)
            .await
            .map_err(|e| SourceError::Http(format!("gpsd {addr}: {e}")))?;
        Ok(Self {
            reader: BufReader::new(stream),
            read_timeout,
        })
    }

    /// Liest bis zur nächsten TPV-Zeile.
    pub async fn next(&mut self) -> Result<Report, SourceError> {
        loop {
            let mut line = String::new();
            let mut limited = (&mut self.reader).take(MAX_LINE);
            let n = tokio::time::timeout(self.read_timeout, limited.read_line(&mut line))
                .await
                .map_err(|_| SourceError::Http("gpsd schweigt".into()))?
                .map_err(|e| SourceError::Http(format!("gpsd: {e}")))?;
            if n == 0 {
                return Err(SourceError::Http("gpsd hat die Verbindung beendet".into()));
            }
            if !line.ends_with('\n') {
                return Err(SourceError::Parse("gpsd: Zeile zu lang".into()));
            }
            if let Some(report) = parse(&line) {
                return Ok(report);
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use tokio::net::TcpListener;

    pub(crate) const VERSION: &str =
        r#"{"class":"VERSION","release":"3.25","rev":"3.25","proto_major":3,"proto_minor":15}"#;
    pub(crate) const NO_FIX: &str = r#"{"class":"TPV","device":"/dev/gps","mode":1}"#;
    pub(crate) const FIX: &str = r#"{"class":"TPV","device":"/dev/gps","mode":3,"time":"2026-11-02T14:50:00.000Z","lat":52.963709,"lon":10.577172,"altMSL":12.4}"#;

    #[test]
    fn parses_only_tpv() {
        assert_eq!(parse(VERSION), None);
        assert_eq!(parse("kein json"), None);
        assert_eq!(parse(NO_FIX), Some(Report::NoFix));
        assert_eq!(
            parse(FIX),
            Some(Report::Fix(Position {
                lat: 52.963709,
                lon: 10.577172
            }))
        );
        // 2D-Fix zählt, Fix ohne Koordinaten oder außerhalb der Erde nicht.
        assert!(matches!(
            parse(r#"{"class":"TPV","mode":2,"lat":1.0,"lon":2.0}"#),
            Some(Report::Fix(_))
        ));
        assert_eq!(
            parse(r#"{"class":"TPV","mode":3,"lat":1.0}"#),
            Some(Report::NoFix)
        );
        assert_eq!(
            parse(r#"{"class":"TPV","mode":1,"lat":1.0,"lon":2.0}"#),
            Some(Report::NoFix)
        );
        assert_eq!(
            parse(r#"{"class":"TPV","mode":3,"lat":91.0,"lon":2.0}"#),
            Some(Report::NoFix)
        );
    }

    /// gpsd zum Schein: erwartet `?WATCH`, schreibt `lines` und bleibt dann `hold` lang offen.
    pub(crate) async fn fake_gpsd(lines: &'static [&'static str], hold: Duration) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut watch = String::new();
            stream.read_line(&mut watch).await.unwrap();
            assert!(watch.starts_with("?WATCH="), "{watch}");
            for line in lines {
                stream
                    .get_mut()
                    .write_all(format!("{line}\n").as_bytes())
                    .await
                    .unwrap();
            }
            tokio::time::sleep(hold).await;
        });
        addr
    }

    #[tokio::test]
    async fn reads_reports_until_silence() {
        let addr = fake_gpsd(&[VERSION, NO_FIX, FIX], Duration::from_secs(5)).await;
        let mut conn = Connection::open(addr, Duration::from_millis(100))
            .await
            .unwrap();
        assert_eq!(conn.next().await.unwrap(), Report::NoFix);
        assert!(matches!(conn.next().await.unwrap(), Report::Fix(_)));
        let err = conn.next().await.unwrap_err();
        assert!(err.to_string().contains("schweigt"), "{err}");
    }

    #[tokio::test]
    async fn closed_connection_is_an_error() {
        let addr = fake_gpsd(&[VERSION], Duration::ZERO).await;
        let mut conn = Connection::open(addr, Duration::from_secs(2))
            .await
            .unwrap();
        let err = conn.next().await.unwrap_err();
        assert!(err.to_string().contains("beendet"), "{err}");
    }
}
