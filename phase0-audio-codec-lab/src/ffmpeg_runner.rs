//! 경로 B: FFmpeg 단독 (외부 프로세스).
//!
//! `common` 방식(기본): FFmpeg는 **디코딩만** 한다. 원본 샘플레이트·채널 그대로의 f32le PCM을 받아
//! Symphonia 경로와 같은 공통 PCM 후처리 계층(다운믹스·리샘플·s16 변환)에 넣는다.
//! `native` 방식(참고용): FFmpeg가 다운믹스·리샘플·변환까지 직접 한다. 후처리 구현 차이를 보기 위한 기준이다.
//!
//! 트랙 선택은 `-map 0:a:<ordinal>` 이라 선택하지 않은 영상·오디오 스트림은 디코드되지 않는다.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::input_probe::ffprobe;
use crate::metrics::{CResult, CodecError, DecodeReport, ErrCat, PipelineReport, TrackInfo};
use crate::pcm_pipeline::PcmPipeline;

/// FFmpeg/ffprobe stderr를 분류한다. 판정 불가면 UnknownFailure.
pub fn classify_ffmpeg_stderr(stderr: &str, code: Option<i32>) -> CodecError {
    let s = stderr.to_lowercase();
    let first = stderr.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
    let msg = format!("exit={:?}: {}", code, first);
    let has = |p: &[&str]| p.iter().any(|x| s.contains(x));
    let cat = if has(&["cannot allocate memory", "out of memory", "enomem"]) {
        ErrCat::OutOfMemory
    } else if has(&["no such file", "permission denied", "is a directory", "input/output error"]) {
        ErrCat::IoError
    } else if has(&["decoder (codec", "unknown decoder", "decoder not found", "unsupported codec", "codec not currently supported", "no decoder"]) {
        ErrCat::UnsupportedCodec
    } else if has(&["moov atom not found", "could not find codec parameters", "stream map", "matches no streams", "does not contain any stream", "no audio"]) {
        ErrCat::ParseFailure
    } else if has(&["invalid data found when processing input", "error while decoding", "corrupt", "truncated", "invalid argument"]) {
        ErrCat::CorruptInput
    } else if has(&["error initializing", "error reinitializing filters", "conversion failed", "error while opening encoder"]) {
        ErrCat::ConversionFailure
    } else {
        ErrCat::UnknownFailure
    };
    CodecError::new(cat, msg)
}

fn tracks_from_probe(p: &crate::input_probe::FfProbe) -> (Vec<TrackInfo>, usize) {
    let mut out = vec![];
    let mut non_audio = 0;
    for s in &p.streams {
        if s.codec_type == "audio" {
            out.push(TrackInfo {
                ordinal: out.len(),
                track_id: s.id.clone(),
                stream_index: Some(s.index),
                codec: s.codec_name.clone(),
                sample_rate: s.sample_rate,
                channels: s.channels,
                language: s.language.clone(),
                is_default: Some(s.is_default),
                declared_frames: None,
                delay: None,
                padding: None,
            });
        } else if s.codec_type == "video" || s.codec_type == "subtitle" || s.codec_type == "data" {
            non_audio += 1;
        }
    }
    (out, non_audio)
}

pub fn list_tracks(ffprobe_bin: &str, path: &Path) -> CResult<(Vec<TrackInfo>, usize)> {
    Ok(tracks_from_probe(&ffprobe(ffprobe_bin, path)?))
}

fn pick_ordinal(tracks: &[TrackInfo], requested: Option<usize>) -> CResult<usize> {
    if tracks.is_empty() {
        return Err(CodecError::new(ErrCat::ParseFailure, "no audio stream in container"));
    }
    match requested {
        Some(o) if o < tracks.len() => Ok(o),
        Some(o) => Err(CodecError::new(ErrCat::ParseFailure, format!("audio track ordinal {o} does not exist ({} audio tracks)", tracks.len()))),
        None => Ok(tracks.iter().position(|t| t.is_default == Some(true)).unwrap_or(0)),
    }
}

/// 디코딩만 FFmpeg, 후처리는 공통 계층.
pub fn run_common(ffmpeg: &str, ffprobe_bin: &str, input: &Path, output: &Path, track: Option<usize>) -> CResult<(DecodeReport, PipelineReport)> {
    let probe = ffprobe(ffprobe_bin, input)?;
    let (tracks, non_audio) = tracks_from_probe(&probe);
    let ord = pick_ordinal(&tracks, track)?;
    let sel = tracks[ord].clone();
    let (rate, ch) = match (sel.sample_rate, sel.channels) {
        (Some(r), Some(c)) if r > 0 && c > 0 => (r, c),
        _ => return Err(CodecError::new(ErrCat::ConversionFailure, "ffprobe reported no sample rate/channels for the selected track")),
    };
    let mut rep = DecodeReport {
        decoder: format!("ffmpeg ({})", probe.format_name),
        tracks,
        non_audio_tracks: non_audio,
        selected_ordinal: Some(ord),
        src_rate: rate,
        src_channels: ch,
        ..Default::default()
    };
    let mut child = Command::new(ffmpeg)
        .args(["-nostdin", "-hide_banner", "-v", "error", "-i"])
        .arg(input)
        .args(["-map", &format!("0:a:{ord}"), "-vn", "-sn", "-dn", "-ar", &rate.to_string(), "-ac", &ch.to_string(), "-f", "f32le", "-acodec", "pcm_f32le", "pipe:1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CodecError::new(ErrCat::IoError, format!("cannot run {ffmpeg}: {e}")))?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let mut pipe = PcmPipeline::create(output, rate, ch as usize)?;
    let frame_bytes = 4 * ch as usize;
    let mut buf = vec![0u8; 256 * 1024];
    let mut carry: Vec<u8> = Vec::new();
    let mut floats: Vec<f32> = Vec::new();
    let mut push_err: Option<CodecError> = None;
    loop {
        let n = match stdout.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                push_err = Some(CodecError::from(e));
                break;
            }
        };
        carry.extend_from_slice(&buf[..n]);
        let usable = carry.len() / frame_bytes * frame_bytes;
        floats.clear();
        floats.extend(carry[..usable].chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])));
        carry.drain(..usable);
        rep.src_frames += (usable / frame_bytes) as u64;
        if let Err(e) = pipe.push_interleaved(&floats) {
            push_err = Some(e);
            break;
        }
    }
    drop(stdout);
    let status = child.wait();
    let stderr_text = err_thread.join().unwrap_or_default();
    if let Some(e) = push_err {
        let _ = child.kill();
        pipe.abort();
        return Err(e);
    }
    let status = match status {
        Ok(s) => s,
        Err(e) => {
            pipe.abort();
            return Err(CodecError::from(e));
        }
    };
    if !status.success() {
        pipe.abort();
        return Err(classify_ffmpeg_stderr(&stderr_text, status.code()));
    }
    if rep.src_frames == 0 {
        pipe.abort();
        return Err(CodecError::new(ErrCat::ParseFailure, format!("ffmpeg produced no audio. {}", stderr_text.lines().next().unwrap_or(""))));
    }
    for l in stderr_text.lines().filter(|l| !l.trim().is_empty()).take(5) {
        rep.warnings.push(format!("ffmpeg stderr: {}", l.trim()));
    }
    let pr = pipe.finish()?;
    Ok((rep, pr))
}

/// 참고용: FFmpeg가 다운믹스·리샘플·s16 변환·WAV 기록까지 직접 한다.
pub fn run_native(ffmpeg: &str, ffprobe_bin: &str, input: &Path, output: &Path, track: Option<usize>) -> CResult<DecodeReport> {
    let probe = ffprobe(ffprobe_bin, input)?;
    let (tracks, non_audio) = tracks_from_probe(&probe);
    let ord = pick_ordinal(&tracks, track)?;
    let rep = DecodeReport {
        decoder: format!("ffmpeg-native ({})", probe.format_name),
        src_rate: tracks[ord].sample_rate.unwrap_or(0),
        src_channels: tracks[ord].channels.unwrap_or(0),
        tracks,
        non_audio_tracks: non_audio,
        selected_ordinal: Some(ord),
        ..Default::default()
    };
    let out = Command::new(ffmpeg)
        .args(["-nostdin", "-hide_banner", "-v", "error", "-y", "-i"])
        .arg(input)
        .args(["-map", &format!("0:a:{ord}"), "-vn", "-sn", "-dn", "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", "-fflags", "+bitexact", "-f", "wav"])
        .arg(output)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| CodecError::new(ErrCat::IoError, format!("cannot run {ffmpeg}: {e}")))?;
    if !out.status.success() {
        let _ = std::fs::remove_file(output);
        return Err(classify_ffmpeg_stderr(&String::from_utf8_lossy(&out.stderr), out.status.code()));
    }
    Ok(rep)
}
