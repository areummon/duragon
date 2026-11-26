//! Speech Dispatcher over SSIP: the system's voices, and speaking with one of them.
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct SdVoice {
    pub module: String,
    pub name: String,
    pub language: String,
}

/// The name people know an output module by.
pub fn engine_name(module: &str) -> &str {
    match module {
        "pico" => "Pico",
        "flite" => "Flite",
        "espeak-ng" => "eSpeak NG",
        "espeak-ng-mbrola" => "eSpeak NG MBROLA",
        "festival" => "Festival",
        other => other,
    }
}

/// SSIP's rate for a speed multiplier: 2× is +100, ½× is −100.
pub fn rate(speed: f64) -> i32 {
    (speed.max(0.01).log2() * 100.0).round().clamp(-100.0, 100.0) as i32
}

fn address() -> PathBuf {
    if let Some(path) = std::env::var("SPEECHD_ADDRESS").ok().and_then(|a| a.strip_prefix("unix_socket:").map(PathBuf::from)) {
        return path;
    }
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_default();
    runtime.join("speech-dispatcher/speechd.sock")
}

/// Connects to the running Speech Dispatcher, starting it once if it is not running.
pub fn connect() -> Result<Conn> {
    let path = address();
    match Conn::at(&path) {
        Ok(c) => Ok(c),
        Err(first) => {
            let spawned = Command::new("speech-dispatcher")
                .arg("--spawn")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            if spawned.is_err() {
                return Err(first);
            }
            Conn::at(&path)
        }
    }
}

/// A local socket takes a line at once, so a Speech Dispatcher that has stopped reading is stuck.
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

struct Reply {
    code: u16,
    lines: Vec<String>,
    last: String,
}

pub struct Conn {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl Conn {
    pub fn at(path: &std::path::Path) -> Result<Self> {
        let stream = UnixStream::connect(path).with_context(|| format!("connecting to Speech Dispatcher at {}", path.display()))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
        let writer = stream.try_clone()?;
        let mut c = Self { reader: BufReader::new(stream), writer };
        c.ok("SET SELF CLIENT_NAME user:duragon:main")?;
        Ok(c)
    }

    fn read_reply(&mut self) -> Result<Reply> {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            if self.reader.read_line(&mut line)? == 0 {
                bail!("Speech Dispatcher closed the connection");
            }
            let line = line.trim_end_matches(['\r', '\n']);
            let code = line.get(..3).and_then(|c| c.parse::<u16>().ok()).ok_or_else(|| anyhow!("unexpected reply: {line}"))?;
            let rest = line.get(4..).unwrap_or_default().to_string();
            if line.as_bytes().get(3) == Some(&b'-') {
                lines.push(rest);
            } else {
                return Ok(Reply { code, lines, last: rest });
            }
        }
    }

    fn send(&mut self, line: &str) -> Result<Reply> {
        write!(self.writer, "{line}\r\n")?;
        self.read_reply()
    }

    fn ok(&mut self, line: &str) -> Result<Reply> {
        let r = self.send(line)?;
        if !(200..300).contains(&r.code) {
            bail!("Speech Dispatcher refused {line}: {} {}", r.code, r.last);
        }
        Ok(r)
    }

    /// Every module's voices with no variant, modules in the order Speech Dispatcher lists them.
    pub fn voices(&mut self) -> Result<Vec<SdVoice>> {
        let modules = self.ok("LIST OUTPUT_MODULES")?.lines;
        let mut out = Vec::new();
        for module in modules.into_iter().filter(|m| m != "dummy") {
            self.ok(&format!("SET SELF OUTPUT_MODULE {module}"))?;
            for line in self.ok("LIST SYNTHESIS_VOICES")?.lines {
                let mut f = line.split('\t');
                if let (Some(name), Some(language), Some("none")) = (f.next(), f.next(), f.next()) {
                    out.push(SdVoice { module: module.clone(), name: name.into(), language: language.into() });
                }
            }
        }
        Ok(out)
    }

    /// Queues `text` and returns at once. `on_playing` hears `true` when it begins and `false` when it ends.
    pub fn speak(
        mut self,
        voice: &SdVoice,
        rate: i32,
        volume: Option<i32>,
        text: &str,
        on_playing: impl Fn(bool) + Send + 'static,
    ) -> Result<Speech> {
        self.ok(&format!("SET SELF OUTPUT_MODULE {}", voice.module))?;
        self.ok(&format!("SET SELF SYNTHESIS_VOICE {}", voice.name))?;
        self.ok(&format!("SET SELF RATE {rate}"))?;
        if let Some(v) = volume {
            self.ok(&format!("SET SELF VOLUME {v}"))?;
        }
        self.ok("SET SELF NOTIFICATION all on")?;
        let r = self.send("SPEAK")?;
        if r.code != 230 {
            bail!("Speech Dispatcher refused to speak: {} {}", r.code, r.last);
        }
        self.writer.write_all(escape(text).as_bytes())?;
        self.reader.get_ref().set_read_timeout(None)?;
        let writer = self.writer.try_clone()?;
        let (done, ended) = mpsc::channel();
        let join = thread::Builder::new().name("duragon-speechd".into()).spawn(move || {
            let mut began = false;
            while let Ok(r) = self.read_reply() {
                match r.code {
                    701 => {
                        began = true;
                        on_playing(true);
                    }
                    702 | 703 => break,
                    300..=699 => {
                        eprintln!("duragon: Speech Dispatcher: {} {}", r.code, r.last);
                        break;
                    }
                    _ => {}
                }
            }
            if began {
                on_playing(false);
            }
            let _ = self.writer.shutdown(std::net::Shutdown::Both);
            let _ = done.send(());
        })?;
        Ok(Speech { writer, join, ended })
    }
}

/// SSIP's message body: CRLF lines, a leading dot doubled, ended by a line holding one dot.
fn escape(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if line.starts_with('.') {
            out.push('.');
        }
        out.push_str(line);
        out.push_str("\r\n");
    }
    out.push_str(".\r\n");
    out
}

pub struct Speech {
    writer: UnixStream,
    join: JoinHandle<()>,
    ended: mpsc::Receiver<()>,
}

impl Speech {
    /// Stops only Duragon's own speech: a screen reader may be speaking too.
    pub fn stop(mut self) {
        let sent = self.writer.write_all(b"STOP SELF\r\n").is_ok();
        if !sent || self.ended.recv_timeout(Duration::from_secs(1)).is_err() {
            let _ = self.writer.shutdown(std::net::Shutdown::Both);
        }
        let _ = self.join.join();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::sync::{Arc, Mutex};

    /// A Speech Dispatcher that answers like the real one and records what it was sent. A line of the text
    /// steers it: `END NOW` ends the speech at once, `SLOW` answers it late, `DEAF` stops reading.
    pub(crate) fn fake(dir: &std::path::Path) -> (PathBuf, Arc<Mutex<Vec<String>>>) {
        let path = dir.join("speechd.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = seen.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let log = log.clone();
                thread::spawn(move || {
                    let mut w = stream.try_clone().unwrap();
                    let mut module = String::new();
                    let mut body = false;
                    let (mut ends, mut slow) = (false, false);
                    for line in BufReader::new(stream).lines() {
                        let Ok(line) = line else { break };
                        let line = line.trim_end_matches('\r').to_string();
                        log.lock().unwrap().push(line.clone());
                        let reply = if body {
                            if line == "." {
                                body = false;
                                if slow {
                                    thread::sleep(Duration::from_millis(150));
                                }
                                let end = if ends { "702-1\r\n702-7\r\n702 END\r\n" } else { "" };
                                format!("225-1\r\n225 OK MESSAGE QUEUED\r\n701-1\r\n701-7\r\n701 BEGIN\r\n{end}")
                            } else {
                                ends |= line == "END NOW";
                                slow |= line == "SLOW";
                                if line == "DEAF" {
                                    thread::sleep(Duration::from_secs(30));
                                }
                                continue;
                            }
                        } else if line == "LIST OUTPUT_MODULES" {
                            "250-pico\r\n250-dummy\r\n250-espeak-ng\r\n250 OK MODULE LIST SENT\r\n".into()
                        } else if let Some(m) = line.strip_prefix("SET SELF OUTPUT_MODULE ") {
                            module = m.into();
                            "216 OK OUTPUT MODULE SET\r\n".into()
                        } else if line == "LIST SYNTHESIS_VOICES" {
                            match module.as_str() {
                                "pico" => "249-samantha\ten-US\tnone\r\n249-serena\ten-GB\tnone\r\n249 OK VOICE LIST SENT\r\n".into(),
                                "espeak-ng" => "249-English (Great Britain)\ten-gb\tnone\r\n249-English (Great Britain)\ten-gb\tklatt\r\n249 OK VOICE LIST SENT\r\n".into(),
                                _ => "249-Nobody\txx\tnone\r\n249 OK VOICE LIST SENT\r\n".into(),
                            }
                        } else if line == "SPEAK" {
                            body = true;
                            "230 OK RECEIVING DATA\r\n".into()
                        } else if line == "STOP SELF" {
                            "210 OK STOPPED\r\n703-1\r\n703-7\r\n703 CANCELED\r\n".into()
                        } else {
                            "200 OK\r\n".into()
                        };
                        if w.write_all(reply.as_bytes()).is_err() {
                            break;
                        }
                    }
                });
            }
        });
        (path, seen)
    }

    fn events() -> (Arc<Mutex<Vec<bool>>>, impl Fn(bool) + Send + 'static) {
        let got: Arc<Mutex<Vec<bool>>> = Arc::default();
        let sink = got.clone();
        (got, move |p| sink.lock().unwrap().push(p))
    }

    fn wait_for(got: &Mutex<Vec<bool>>, n: usize) {
        for _ in 0..200 {
            if got.lock().unwrap().len() >= n {
                return;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn samantha() -> SdVoice {
        SdVoice { module: "pico".into(), name: "samantha".into(), language: "en-US".into() }
    }

    #[test]
    fn lists_every_module_but_dummy_and_only_plain_voices() {
        let dir = tempfile_dir();
        let (path, _) = fake(&dir);
        let voices = Conn::at(&path).unwrap().voices().unwrap();
        let names: Vec<_> = voices.iter().map(|v| format!("{}/{}/{}", v.module, v.name, v.language)).collect();
        assert_eq!(names, ["pico/samantha/en-US", "pico/serena/en-GB", "espeak-ng/English (Great Britain)/en-gb"]);
    }

    #[test]
    fn speaks_until_the_end_with_leading_dots_doubled() {
        let dir = tempfile_dir();
        let (path, seen) = fake(&dir);
        let (got, on) = events();
        let speech = Conn::at(&path).unwrap().speak(&samantha(), -100, Some(-100), "Hello.\n.dot first\nEND NOW", on).unwrap();
        wait_for(&got, 2);
        assert_eq!(*got.lock().unwrap(), [true, false], "began, then ended");
        let log = seen.lock().unwrap().clone();
        let at = log.iter().position(|l| l == "SPEAK").unwrap();
        assert_eq!(&log[at + 1..at + 5], ["Hello.", "..dot first", "END NOW", "."]);
        assert!(log.contains(&"SET SELF SYNTHESIS_VOICE samantha".to_string()), "{log:?}");
        assert!(log.contains(&"SET SELF RATE -100".to_string()) && log.contains(&"SET SELF VOLUME -100".to_string()));
        speech.stop();
        assert_eq!(*got.lock().unwrap(), [true, false], "a speech that ended stops quietly");
    }

    #[test]
    fn stopping_cancels_only_its_own_speech() {
        let dir = tempfile_dir();
        let (path, seen) = fake(&dir);
        let (got, on) = events();
        let speech = Conn::at(&path).unwrap().speak(&samantha(), 0, None, "A long thought.", on).unwrap();
        wait_for(&got, 1);
        speech.stop();
        assert_eq!(*got.lock().unwrap(), [true, false], "began, then canceled");
        let log = seen.lock().unwrap().clone();
        assert!(log.contains(&"STOP SELF".to_string()), "{log:?}");
        assert!(!log.iter().any(|l| l.contains("STOP ALL")));
    }

    #[test]
    fn a_speech_dispatcher_that_stops_reading_fails_the_speech_instead_of_hanging() {
        let dir = tempfile_dir();
        let (path, _) = fake(&dir);
        let text = format!("DEAF\n{}", "words ".repeat(1 << 20));
        let (done, finished) = mpsc::channel();
        thread::spawn(move || {
            let _ = done.send(Conn::at(&path).unwrap().speak(&samantha(), 0, None, &text, |_| {}).is_err());
        });
        // A write that sent part of it waits once more before it fails.
        assert_eq!(finished.recv_timeout(WRITE_TIMEOUT * 3), Ok(true));
    }

    #[test]
    fn rate_follows_speed_on_a_log_scale() {
        assert_eq!([rate(1.0), rate(2.0), rate(0.5), rate(1.5), rate(4.0)], [0, 100, -100, 58, 100]);
    }

    fn tempfile_dir() -> PathBuf {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("duragon-speechd-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `cargo test -p duragon speechd -- --ignored --nocapture`: this machine's voices, and one silent line.
    #[test]
    #[ignore]
    fn real_speech_dispatcher() {
        let voices = connect().unwrap().voices().unwrap();
        let mut modules: Vec<(&str, usize)> = Vec::new();
        for v in &voices {
            match modules.iter_mut().find(|m| m.0 == v.module) {
                Some(m) => m.1 += 1,
                None => modules.push((&v.module, 1)),
            }
        }
        println!("{} voices: {modules:?}", voices.len());
        let voice = voices.first().expect("a voice");
        println!("speaking with {voice:?}");
        let (got, on) = events();
        let t = std::time::Instant::now();
        let _speech = connect().unwrap().speak(voice, 0, Some(-100), "Duragon is speaking.", on).unwrap();
        for _ in 0..200 {
            if got.lock().unwrap().len() >= 2 {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        println!("events {:?} after {:.1}s", got.lock().unwrap(), t.elapsed().as_secs_f32());
        assert_eq!(*got.lock().unwrap(), [true, false]);
    }
}
