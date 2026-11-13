//! 입력 조사: SHA-256, ffprobe 기반 스트림 목록.

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::metrics::{CResult, CodecError, ErrCat};

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    let d = h.finalize();
    Ok(d.iter().map(|b| format!("{b:02x}")).collect())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FfStream {
    pub index: u64,
    pub id: Option<String>,
    pub codec_type: String,
    pub codec_name: String,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub channel_layout: Option<String>,
    pub language: Option<String>,
    pub is_default: bool,
    pub duration_s: Option<f64>,
    pub bit_rate: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FfProbe {
    pub format_name: String,
    pub duration_s: Option<f64>,
    pub streams: Vec<FfStream>,
}

impl FfProbe {
    pub fn audio_streams(&self) -> Vec<&FfStream> {
        self.streams.iter().filter(|s| s.codec_type == "audio").collect()
    }
}

fn s_to<T: std::str::FromStr>(v: &serde_json::Value) -> Option<T> {
    match v {
        serde_json::Value::String(s) => s.parse().ok(),
        serde_json::Value::Number(n) => n.to_string().parse().ok(),
        _ => None,
    }
}

pub fn ffprobe(bin: &str, path: &Path) -> CResult<FfProbe> {
    let out = Command::new(bin)
        .args(["-v", "error", "-show_streams", "-show_format", "-of", "json"])
        .arg(path)
        .output()
        .map_err(|e| CodecError::new(ErrCat::IoError, format!("cannot run {bin}: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(crate::ffmpeg_runner::classify_ffmpeg_stderr(&stderr, out.status.code()));
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| CodecError::new(ErrCat::ParseFailure, format!("ffprobe json: {e}")))?;
    let mut p = FfProbe {
        format_name: v["format"]["format_name"].as_str().unwrap_or("").to_string(),
        duration_s: s_to(&v["format"]["duration"]),
        streams: vec![],
    };
    if let Some(arr) = v["streams"].as_array() {
        for s in arr {
            p.streams.push(FfStream {
                index: s["index"].as_u64().unwrap_or(0),
                id: s["id"].as_str().map(|x| x.to_string()),
                codec_type: s["codec_type"].as_str().unwrap_or("").to_string(),
                codec_name: s["codec_name"].as_str().unwrap_or("unknown").to_string(),
                sample_rate: s_to(&s["sample_rate"]),
                channels: s["channels"].as_u64().map(|x| x as u32),
                channel_layout: s["channel_layout"].as_str().map(|x| x.to_string()),
                language: s["tags"]["language"].as_str().map(|x| x.to_string()),
                is_default: s["disposition"]["default"].as_i64().unwrap_or(0) == 1,
                duration_s: s_to(&s["duration"]),
                bit_rate: s_to(&s["bit_rate"]),
            });
        }
    }
    Ok(p)
}
