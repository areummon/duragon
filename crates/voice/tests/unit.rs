use duragon_voice::audio::{level, Resampler};
use duragon_voice::models::{self, Asset, OnBytes};
use duragon_voice::tts::{speaker_id, split_sentences, voices};
use duragon_voice::{Emit, ModelState, Voice};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

#[test]
fn sentences_split_at_terminators_but_not_inside_numbers() {
    assert_eq!(
        split_sentences("Hello there. It costs 3.5 dollars! Really? yes"),
        vec!["Hello there.", "It costs 3.5 dollars!", "Really?", "yes"]
    );
    assert!(split_sentences("  ...  ").is_empty());
    let long = format!("{}, {}.", "word ".repeat(50), "tail ".repeat(5));
    assert!(split_sentences(&long).len() <= 2);
}

#[test]
fn resampler_keeps_duration_and_averages() {
    let mut out = Vec::new();
    let mut r = Resampler::new(48_000, 16_000);
    for _ in 0..10 {
        r.push(&vec![0.5; 4_800], &mut out);
    }
    assert!((out.len() as i64 - 16_000).abs() <= 1, "{}", out.len());
    assert!(out.iter().all(|v| (v - 0.5).abs() < 1e-4));
    let mut up = Vec::new();
    Resampler::new(24_000, 48_000).push(&[0.0; 2400], &mut up);
    assert_eq!(up.len(), 4800);
}

#[test]
fn level_is_clamped() {
    assert_eq!(level(&[]), 0.0);
    assert_eq!(level(&[1.0; 100]), 1.0);
    assert!(level(&[0.01; 100]) < 0.1);
}

#[test]
fn voices_map_to_distinct_kokoro_speakers() {
    let v = voices();
    assert_eq!(v.len(), 4);
    assert_eq!(v[0].id, "bm_george");
    assert_eq!(speaker_id("bf_emma"), 21);
    assert_eq!(speaker_id("Aria"), speaker_id("bm_george"));
}

#[test]
fn status_reports_missing_without_models() {
    let dir = tempfile::tempdir().unwrap();
    let emit: Emit = Arc::new(|_, _| {});
    let s = Voice::new(dir.path().join("models"), emit).status();
    assert_eq!((s.stt, s.tts, s.listening), (ModelState::Missing, ModelState::Missing, false));
    let json = serde_json::to_value(&s).unwrap();
    assert_eq!(json["stt"], "missing");
}

/// Serves `body`, honouring `Range: bytes=N-`, then closes. Returns the base URL.
fn serve(body: Vec<u8>, connections: usize) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    std::thread::spawn(move || {
        for _ in 0..connections {
            let (mut s, _) = l.accept().unwrap();
            let mut buf = [0u8; 2048];
            let n = s.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_lowercase();
            let from: usize = req
                .lines()
                .find_map(|l| l.strip_prefix("range: bytes="))
                .and_then(|r| r.trim_end_matches('-').parse().ok())
                .unwrap_or(0);
            let (status, slice) = if from > 0 { ("206 Partial Content", &body[from..]) } else { ("200 OK", &body[..]) };
            let _ = write!(s, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", slice.len());
            let _ = s.write_all(slice);
        }
    });
    format!("http://{addr}/file")
}

#[tokio::test]
async fn download_resumes_from_a_part_file() {
    let body: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
    let url = serve(body.clone(), 1);
    let dir = tempfile::tempdir().unwrap();
    let part = dir.path().join("f.part");
    std::fs::write(&part, &body[..30_000]).unwrap();
    let mut seen = Vec::new();
    let mut cb = |b: u64| seen.push(b);
    models::download(&reqwest::Client::new(), &url, &part, &mut cb as OnBytes).await.unwrap();
    assert_eq!(std::fs::read(&part).unwrap(), body);
    assert_eq!(seen.first(), Some(&30_000), "resumed, not restarted");
    assert_eq!(seen.last(), Some(&100_000));
}

#[tokio::test]
async fn failed_download_is_an_error_and_leaves_no_final_file() {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    std::thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut b = [0u8; 1024];
        let _ = s.read(&mut b);
        let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
    });
    static A: Asset = Asset { url: "", bytes: 1, target: "x.bin", archive_top: None, required: &[] };
    let dir = tempfile::tempdir().unwrap();
    let part = dir.path().join("x.bin.part");
    let mut cb = |_: u64| {};
    let err = models::download(&reqwest::Client::new(), &format!("http://{addr}/x"), &part, &mut cb as OnBytes)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("404"), "{err}");
    assert!(!A.path(dir.path()).exists());
}
