//! 출력 WAV 검증기: 헤더 파싱, 통계(Peak/RMS/클리핑/DC), 두 WAV 비교, 톤 진폭.
//!
//! WAV 청크를 순회하며 알 수 없는 청크(LIST 등)는 건너뛴다. 그래서 FFmpeg가 쓴 WAV도 읽는다.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WavHeader {
    pub format_tag: u16,
    pub channels: u16,
    pub sample_rate: u32,
    pub bits: u16,
    pub block_align: u16,
    pub byte_rate: u32,
    pub riff_size: u32,
    pub data_size: u32,
    pub data_offset: u64,
    pub file_len: u64,
    pub chunk_ids: Vec<String>,
}

fn rd_u16(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}
fn rd_u32(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

pub fn read_wav_header(path: &Path) -> Result<WavHeader, String> {
    let mut f = File::open(path).map_err(|e| format!("open: {e}"))?;
    let file_len = f.metadata().map_err(|e| e.to_string())?.len();
    let mut head = [0u8; 12];
    f.read_exact(&mut head).map_err(|e| format!("RIFF header: {e}"))?;
    if &head[0..4] != b"RIFF" || &head[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let mut h = WavHeader { riff_size: rd_u32(&head[4..8]), file_len, ..Default::default() };
    let mut have_fmt = false;
    let mut pos: u64 = 12;
    loop {
        if pos + 8 > file_len {
            return Err("data chunk not found".into());
        }
        f.seek(SeekFrom::Start(pos)).map_err(|e| e.to_string())?;
        let mut ch = [0u8; 8];
        f.read_exact(&mut ch).map_err(|e| format!("chunk header: {e}"))?;
        let id = String::from_utf8_lossy(&ch[0..4]).to_string();
        let size = rd_u32(&ch[4..8]);
        h.chunk_ids.push(id.clone());
        if id == "fmt " {
            if size < 16 {
                return Err("fmt chunk too small".into());
            }
            let mut fmt = [0u8; 16];
            f.read_exact(&mut fmt).map_err(|e| format!("fmt chunk: {e}"))?;
            h.format_tag = rd_u16(&fmt[0..2]);
            h.channels = rd_u16(&fmt[2..4]);
            h.sample_rate = rd_u32(&fmt[4..8]);
            h.byte_rate = rd_u32(&fmt[8..12]);
            h.block_align = rd_u16(&fmt[12..14]);
            h.bits = rd_u16(&fmt[14..16]);
            have_fmt = true;
        } else if id == "data" {
            if !have_fmt {
                return Err("data chunk before fmt chunk".into());
            }
            h.data_size = size;
            h.data_offset = pos + 8;
            return Ok(h);
        }
        pos += 8 + size as u64 + (size as u64 & 1);
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WavStats {
    pub samples: u64,
    pub peak_abs: i32,
    pub peak: f64,
    pub rms: f64,
    pub dc_offset: f64,
    /// 값이 -32768 또는 32767인 샘플 수 (출력 쪽에서 본 full-scale 도달)
    pub full_scale: u64,
}

pub fn wav_stats(path: &Path, h: &WavHeader) -> Result<WavStats, String> {
    let mut f = BufReader::with_capacity(1 << 20, File::open(path).map_err(|e| e.to_string())?);
    f.seek(SeekFrom::Start(h.data_offset)).map_err(|e| e.to_string())?;
    let avail = h.file_len.saturating_sub(h.data_offset);
    let total_bytes = (h.data_size as u64).min(avail);
    let mut left = total_bytes;
    let mut buf = vec![0u8; 1 << 16];
    let (mut n, mut sum, mut sum_sq, mut peak, mut fs) = (0u64, 0f64, 0f64, 0i32, 0u64);
    let mut carry: Option<u8> = None;
    while left > 0 {
        let want = (buf.len() as u64).min(left) as usize;
        f.read_exact(&mut buf[..want]).map_err(|e| e.to_string())?;
        left -= want as u64;
        let mut chunk: &[u8] = &buf[..want];
        let mut tmp;
        if let Some(c) = carry.take() {
            tmp = vec![c];
            tmp.extend_from_slice(chunk);
            chunk = &tmp[..];
        }
        let even = chunk.len() & !1;
        for p in chunk[..even].chunks_exact(2) {
            let s = i16::from_le_bytes([p[0], p[1]]) as i32;
            n += 1;
            sum += s as f64;
            sum_sq += (s as f64) * (s as f64);
            peak = peak.max(s.abs());
            if s == 32767 || s == -32768 {
                fs += 1;
            }
        }
        if chunk.len() & 1 == 1 {
            carry = Some(chunk[chunk.len() - 1]);
        }
    }
    let nf = n.max(1) as f64;
    Ok(WavStats {
        samples: n,
        peak_abs: peak,
        peak: peak as f64 / 32768.0,
        rms: (sum_sq / nf).sqrt() / 32768.0,
        dc_offset: sum / nf / 32768.0,
        full_scale: fs,
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Validation {
    pub ok: bool,
    pub parse_ok: bool,
    pub rate_ok: bool,
    pub channels_ok: bool,
    pub format_ok: bool,
    pub sizes_consistent: bool,
    pub header: Option<WavHeader>,
    pub stats: Option<WavStats>,
    pub duration_s: f64,
    pub problems: Vec<String>,
}

/// 요구사항 §8의 항목 1~5, 6, 7, 9~12를 검증한다 (8번 NaN/Inf는 후처리 계층이 센다).
pub fn validate(path: &Path) -> Validation {
    let mut v = Validation::default();
    let h = match read_wav_header(path) {
        Ok(h) => h,
        Err(e) => {
            v.problems.push(format!("WAV parse failed: {e}"));
            return v;
        }
    };
    v.parse_ok = true;
    v.rate_ok = h.sample_rate == 16_000;
    v.channels_ok = h.channels == 1;
    v.format_ok = h.format_tag == 1 && h.bits == 16 && h.block_align == 2 && h.byte_rate == 32_000;
    let avail = h.file_len.saturating_sub(h.data_offset);
    let riff_ok = h.riff_size as u64 + 8 == h.file_len;
    let data_ok = h.data_size as u64 == avail;
    v.sizes_consistent = riff_ok && data_ok;
    if !v.rate_ok {
        v.problems.push(format!("sample rate {} != 16000", h.sample_rate));
    }
    if !v.channels_ok {
        v.problems.push(format!("channels {} != 1", h.channels));
    }
    if !v.format_ok {
        v.problems.push(format!("format tag={} bits={} not PCM S16LE", h.format_tag, h.bits));
    }
    if !riff_ok {
        v.problems.push(format!("RIFF size {} != file_len-8 {}", h.riff_size, h.file_len.saturating_sub(8)));
    }
    if !data_ok {
        v.problems.push(format!("data size {} != actual {}", h.data_size, avail));
    }
    match wav_stats(path, &h) {
        Ok(s) => {
            v.duration_s = s.samples as f64 / h.sample_rate.max(1) as f64;
            v.stats = Some(s);
        }
        Err(e) => v.problems.push(format!("stats failed: {e}")),
    }
    v.header = Some(h);
    v.ok = v.parse_ok && v.rate_ok && v.channels_ok && v.format_ok && v.sizes_consistent && v.stats.is_some();
    v
}

pub fn read_samples(path: &Path, h: &WavHeader, start: u64, count: usize) -> Result<Vec<i16>, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    f.seek(SeekFrom::Start(h.data_offset + start * 2)).map_err(|e| e.to_string())?;
    let mut raw = vec![0u8; count * 2];
    f.read_exact(&mut raw).map_err(|e| e.to_string())?;
    Ok(raw.chunks_exact(2).map(|p| i16::from_le_bytes([p[0], p[1]])).collect())
}

/// Hann 창을 적용한 Goertzel로 사인파 **최대 진폭**(풀스케일 1.0 기준)을 추정한다.
pub fn tone_amplitude(samples: &[i16], freq: f64, rate: f64) -> f64 {
    let n = samples.len();
    if n < 8 {
        return 0.0;
    }
    let w = 2.0 * std::f64::consts::PI * freq / rate;
    let coeff = 2.0 * w.cos();
    let (mut s1, mut s2, mut wsum) = (0.0f64, 0.0f64, 0.0f64);
    for (i, &x) in samples.iter().enumerate() {
        let win = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (n as f64 - 1.0)).cos();
        wsum += win;
        let s0 = (x as f64 / 32768.0) * win + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let power = s1 * s1 + s2 * s2 - coeff * s1 * s2;
    2.0 * power.max(0.0).sqrt() / wsum
}

/// "440:0.25,1000:0.25" 형식의 기대 톤(주파수:최대 진폭)을 측정값과 함께 돌려준다.
pub fn check_tones(path: &Path, spec: &str) -> Result<Vec<(f64, f64, f64)>, String> {
    let h = read_wav_header(path)?;
    let total = (h.data_size as u64 / 2).min(h.file_len.saturating_sub(h.data_offset) / 2);
    let len = total.min(160_000) as usize;
    let start = (total as usize - len) / 2;
    let s = read_samples(path, &h, start as u64, len)?;
    let mut out = vec![];
    for part in spec.split(',').filter(|p| !p.trim().is_empty()) {
        let mut it = part.split(':');
        let f: f64 = it.next().and_then(|x| x.trim().parse().ok()).ok_or("bad tone spec")?;
        let a: f64 = it.next().and_then(|x| x.trim().parse().ok()).ok_or("bad tone spec")?;
        out.push((f, a, tone_amplitude(&s, f, h.sample_rate as f64)));
    }
    Ok(out)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompareResult {
    pub samples_a: u64,
    pub samples_b: u64,
    pub diff_samples: i64,
    pub rms_a: f64,
    pub rms_b: f64,
    pub peak_a: f64,
    pub peak_b: f64,
    pub window_samples: usize,
    pub corr_lag0: f64,
    pub best_lag: i32,
    pub corr_best: f64,
    /// 최적 지연에서 (a-b)의 RMS를 a의 RMS로 나눈 값
    pub rel_diff_rms_best: f64,
}

/// 두 WAV의 중간 구간(최대 80,000 샘플)에서 ±`max_lag` 샘플 범위의 정규화 상호상관을 구한다.
pub fn compare(a: &Path, b: &Path, max_lag: i32) -> Result<CompareResult, String> {
    let ha = read_wav_header(a)?;
    let hb = read_wav_header(b)?;
    let sa = wav_stats(a, &ha)?;
    let sb = wav_stats(b, &hb)?;
    let mut r = CompareResult {
        samples_a: sa.samples,
        samples_b: sb.samples,
        diff_samples: sa.samples as i64 - sb.samples as i64,
        rms_a: sa.rms,
        rms_b: sb.rms,
        peak_a: sa.peak,
        peak_b: sb.peak,
        ..Default::default()
    };
    let min_len = sa.samples.min(sb.samples) as i64;
    let lag = max_lag as i64;
    let mut len = (min_len - 2 * lag - 2).min(80_000);
    if len < 1000 {
        return Ok(r);
    }
    len = len.max(1000);
    let start = ((min_len - len) / 2).max(lag);
    let xa: Vec<f32> = read_samples(a, &ha, start as u64, len as usize)?.iter().map(|&v| v as f32).collect();
    let rb = read_samples(b, &hb, (start - lag) as u64, (len + 2 * lag) as usize)?;
    let xb: Vec<f32> = rb.iter().map(|&v| v as f32).collect();
    let ea: f64 = xa.iter().map(|&v| (v as f64) * (v as f64)).sum();
    r.window_samples = len as usize;
    let mut best = (-2.0f64, 0i32, 0.0f64);
    for l in -lag..=lag {
        let off = (l + lag) as usize;
        let seg = &xb[off..off + len as usize];
        let mut dot = 0.0f64;
        let mut eb = 0.0f64;
        for (p, q) in xa.iter().zip(seg.iter()) {
            dot += (*p as f64) * (*q as f64);
            eb += (*q as f64) * (*q as f64);
        }
        let c = if ea > 0.0 && eb > 0.0 { dot / (ea * eb).sqrt() } else { 0.0 };
        if l == 0 {
            r.corr_lag0 = c;
        }
        if c > best.0 {
            best = (c, l as i32, eb);
        }
    }
    r.best_lag = best.1;
    r.corr_best = best.0;
    let off = (best.1 as i64 + lag) as usize;
    let seg = &xb[off..off + len as usize];
    let d: f64 = xa.iter().zip(seg.iter()).map(|(p, q)| ((*p - *q) as f64).powi(2)).sum();
    r.rel_diff_rms_best = if ea > 0.0 { (d / ea).sqrt() } else { 0.0 };
    Ok(r)
}
