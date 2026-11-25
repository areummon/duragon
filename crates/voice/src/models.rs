//! Model assets: where they come from, where they live, and a resumable, atomic installer.
use anyhow::{anyhow, bail, Context, Result};
use futures_util::StreamExt;
use std::fs;
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Asset {
    pub url: &'static str,
    /// Approximate download size, used to weight overall progress.
    pub bytes: u64,
    /// Final path under the models dir (a file for a plain download, a directory for an archive).
    pub target: &'static str,
    /// Top-level directory inside the archive; `None` for a plain file.
    pub archive_top: Option<&'static str>,
    /// Files that must exist under `target` (relative; empty for a plain file).
    pub required: &'static [&'static str],
}

pub const VAD_FILE: &str = "silero_vad.onnx";
pub const STT_DIR: &str = "parakeet-tdt-0.6b-v2-int8";
pub const TTS_DIR: &str = "kokoro-int8-multi-lang-v1_0";

static VAD: Asset = Asset {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx",
    bytes: 650_000,
    target: VAD_FILE,
    archive_top: None,
    required: &[],
};

static STT: Asset = Asset {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2",
    bytes: 482_000_000,
    target: STT_DIR,
    archive_top: Some("sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8"),
    required: &["encoder.int8.onnx", "decoder.int8.onnx", "joiner.int8.onnx", "tokens.txt"],
};

static TTS: Asset = Asset {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/kokoro-int8-multi-lang-v1_0.tar.bz2",
    bytes: 132_000_000,
    target: TTS_DIR,
    archive_top: Some("kokoro-int8-multi-lang-v1_0"),
    required: &["model.int8.onnx", "voices.bin", "tokens.txt", "lexicon-us-en.txt", "lexicon-gb-en.txt", "espeak-ng-data"],
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Stt,
    Vad,
    Tts,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Stt, Kind::Vad, Kind::Tts];

    pub fn asset(self) -> &'static Asset {
        match self {
            Kind::Stt => &STT,
            Kind::Vad => &VAD,
            Kind::Tts => &TTS,
        }
    }
}

impl Asset {
    pub fn path(&self, models: &Path) -> PathBuf {
        models.join(self.target)
    }

    pub fn is_ready(&self, models: &Path) -> bool {
        let p = self.path(models);
        if self.archive_top.is_none() {
            return p.is_file();
        }
        self.required.iter().all(|f| p.join(f).exists())
    }
}

pub fn is_ready(models: &Path, kind: Kind) -> bool {
    kind.asset().is_ready(models)
}

pub fn stt_dir(models: &Path) -> PathBuf {
    models.join(STT_DIR)
}

pub fn tts_dir(models: &Path) -> PathBuf {
    models.join(TTS_DIR)
}

pub fn vad_path(models: &Path) -> PathBuf {
    models.join(VAD_FILE)
}

/// Progress callback: bytes downloaded so far for the asset being fetched.
pub type OnBytes<'a> = &'a mut (dyn FnMut(u64) + Send);

/// Downloads `url` into `part`, resuming from its current length when the server allows it.
pub async fn download(client: &reqwest::Client, url: &str, part: &Path, on_bytes: OnBytes<'_>) -> Result<()> {
    let have = fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    let mut req = client.get(url);
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let resp = req.send().await.with_context(|| format!("requesting {url}"))?;
    let status = resp.status();
    let mut written = match status.as_u16() {
        206 => have,
        200 => 0,
        416 => return Ok(()),
        _ => bail!("{url}: HTTP {status}"),
    };
    let mut file = fs::OpenOptions::new().create(true).write(true).append(written > 0).truncate(written == 0).open(part)?;
    on_bytes(written);
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("downloading {url}"))?;
        file.write_all(&chunk)?;
        written += chunk.len() as u64;
        on_bytes(written);
    }
    file.flush()?;
    Ok(())
}

/// Extracts a `.tar.bz2` into `staging`, then renames `staging/<top>` onto `dest` after verifying `required`.
pub fn extract_archive(archive: &Path, asset: &Asset, models: &Path) -> Result<()> {
    let top = asset.archive_top.ok_or_else(|| anyhow!("not an archive asset"))?;
    let staging = models.join(format!(".extract-{}", asset.target));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)?;
    let result = (|| -> Result<()> {
        let file = fs::File::open(archive)?;
        tar::Archive::new(bzip2::read::BzDecoder::new(std::io::BufReader::new(file)))
            .unpack(&staging)
            .context("extracting archive")?;
        let src = staging.join(top);
        for f in asset.required {
            if !src.join(f).exists() {
                bail!("archive is missing {f}");
            }
        }
        let dest = asset.path(models);
        let _ = fs::remove_dir_all(&dest);
        fs::rename(&src, &dest)?;
        Ok(())
    })();
    let _ = fs::remove_dir_all(&staging);
    result
}

/// Installs one asset: download to `<target>.part`, extract or rename atomically.
pub async fn install_asset(client: &reqwest::Client, asset: &'static Asset, models: &Path, on_bytes: OnBytes<'_>) -> Result<()> {
    if asset.is_ready(models) {
        return Ok(());
    }
    fs::create_dir_all(models)?;
    let part = models.join(format!("{}.part", asset.target));
    download(client, asset.url, &part, on_bytes).await?;
    if asset.archive_top.is_some() {
        let (part2, models2) = (part.clone(), models.to_path_buf());
        tokio::task::spawn_blocking(move || extract_archive(&part2, asset, &models2)).await??;
        let _ = fs::remove_file(&part);
    } else {
        fs::rename(&part, asset.path(models))?;
    }
    Ok(())
}
