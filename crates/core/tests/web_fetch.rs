mod common;
use common::StubServer;
use duragon_core::ai::web_fetch::{fetch_web_page, is_public_ip};
use std::net::IpAddr;

#[test]
fn is_public_ip_rejects_private_reserved_and_allows_public() {
    let blocked = [
        "0.0.0.0",
        "0.1.2.3",
        "10.0.0.1",
        "10.255.255.254",
        "100.64.0.1",
        "100.127.255.254",
        "127.0.0.1",
        "127.255.255.254",
        "169.254.169.254",
        "169.254.0.1",
        "172.16.0.1",
        "172.31.255.254",
        "192.0.0.1",
        "192.0.2.1",
        "192.88.99.1",
        "192.168.1.1",
        "198.18.0.1",
        "198.19.255.254",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "240.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "fc00::1",
        "fd12:3456::1",
        "fe80::1",
        "ff02::1",
        "64:ff9b::1",
        "64:ff9b:1::1",
        "100::1",
        "2001:db8::1",
        "2002::1",
        "::ffff:127.0.0.1",
        "::ffff:169.254.169.254",
        "::ffff:10.0.0.1",
    ];
    for raw in blocked {
        let ip: IpAddr = raw.parse().unwrap();
        assert!(!is_public_ip(ip), "expected {raw} to be blocked");
    }

    let allowed = [
        "1.1.1.1",
        "8.8.8.8",
        "100.63.255.255",
        "100.128.0.1",
        "172.15.255.255",
        "172.32.0.1",
        "2606:4700:4700::1111",
        "2001:4860:4860::8888",
        "::ffff:1.1.1.1",
    ];
    for raw in allowed {
        let ip: IpAddr = raw.parse().unwrap();
        assert!(is_public_ip(ip), "expected {raw} to be public");
    }
}

#[tokio::test]
async fn fetch_web_page_ssrf_html_credentials_and_binary_checks() {
    let server = StubServer::start_raw(|req, _| match req.path.as_str() {
        "/page" => (
            200,
            "text/html; charset=utf-8",
            b"<html><head><title>Rust &amp; Friends</title><style>body{color:red}</style><script>alert(1)</script></head><body><noscript>nojs</noscript><svg><path d=\"M0 0\"/></svg><h1>Hello &lt;World&gt;</h1><p>It&#39;s&nbsp;&quot;clean&quot;.</p></body></html>".to_vec(),
        ),
        "/bin" => (200, "image/png", vec![0x89, 0x50, 0x4E, 0x47]),
        "/pdf" => (200, "application/pdf", b"%PDF-1.4".to_vec()),
        _ => (404, "text/plain", b"missing".to_vec()),
    });
    let base = &server.url;

    // 1. Blocked when allow_private = false
    let err = fetch_web_page(&format!("{base}/page"), false).await.unwrap_err();
    assert!(err.contains("non-public"), "expected SSRF block, got: {err}");

    // 2. Clean HTML extraction when allow_private = true
    let out = fetch_web_page(&format!("{base}/page"), true).await.unwrap();
    assert!(out.starts_with(&format!("Source: {base}/page\n\n")), "{out}");
    assert!(out.contains("Title: Rust & Friends"), "{out}");
    assert!(out.contains("Hello <World>"), "{out}");
    assert!(out.contains("It's \"clean\"."), "{out}");
    assert!(!out.contains("alert(1)"), "{out}");
    assert!(!out.contains("color:red"), "{out}");
    assert!(!out.contains("nojs"), "{out}");

    // 3. Embedded credentials rejected
    let with_creds = base.replacen("http://", "http://user:pass@", 1);
    let err = fetch_web_page(&format!("{with_creds}/page"), true)
        .await
        .unwrap_err();
    assert!(err.contains("credentials"), "got: {err}");

    // 4. Binary content types rejected
    let err = fetch_web_page(&format!("{base}/bin"), true).await.unwrap_err();
    assert!(err.contains("unsupported content type"), "got: {err}");
    let err = fetch_web_page(&format!("{base}/pdf"), true).await.unwrap_err();
    assert!(err.contains("unsupported content type"), "got: {err}");
}
