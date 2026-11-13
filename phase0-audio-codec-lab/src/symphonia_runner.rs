//! 경로 A: Symphonia 0.6.1 단독.
//!
//! 컨테이너 파싱 → 오디오 트랙 선택 → 디코딩 → 공통 PCM 후처리 계층.
//! 선택하지 않은 트랙(영상, 다른 오디오 트랙)의 패킷은 읽기만 하고 디코드하지 않는다.
//!
//! 오류 분류 (단계별):
//! - probe 단계 `Unsupported` → UnsupportedContainer. 주의: Symphonia는 "알 수 없는 컨테이너"와
//!   "손상되어 인식되지 않는 컨테이너"를 구분하지 못한다(둘 다 같은 오류). 이 한계는 보고서에 기록한다.
//! - 디코더 생성 단계 `Unsupported` → UnsupportedCodec
//! - probe/demux/디코더 생성 단계 `DecodeError` → ParseFailure
//! - 디코드 단계 `DecodeError` → 패킷을 건너뛰고 센다. 하나도 못 디코드하면 CorruptInput
//! - 예기치 않은 EOF → 이미 디코드한 프레임이 있으면 경고만 남기고 성공(부분 출력), 없으면 CorruptInput
//! - `LimitError` → CorruptInput (악의적·손상 스트림 방어 한도)

use std::fs::File;
use std::path::Path;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, Track, TrackFlags, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::metrics::{CResult, CodecError, DecodeReport, ErrCat, PipelineReport, TrackInfo};
use crate::pcm_pipeline::PcmPipeline;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Probe,
    Demux,
    DecoderInit,
    Decode,
}

fn classify(e: &Error, stage: Stage) -> CodecError {
    match e {
        Error::IoError(io) => {
            if io.kind() == std::io::ErrorKind::UnexpectedEof {
                CodecError::new(ErrCat::CorruptInput, format!("unexpected EOF (truncated input?): {io}"))
            } else {
                CodecError::new(ErrCat::IoError, io.to_string())
            }
        }
        Error::DecodeError(m) => match stage {
            Stage::Decode => CodecError::new(ErrCat::CorruptInput, format!("malformed stream: {m}")),
            _ => CodecError::new(ErrCat::ParseFailure, format!("malformed stream: {m}")),
        },
        Error::Unsupported(m) => match stage {
            Stage::Probe | Stage::Demux => CodecError::new(ErrCat::UnsupportedContainer, format!("unsupported: {m}")),
            Stage::DecoderInit | Stage::Decode => CodecError::new(ErrCat::UnsupportedCodec, format!("unsupported: {m}")),
        },
        Error::LimitError(m) => CodecError::new(ErrCat::CorruptInput, format!("limit reached: {m}")),
        other => CodecError::new(ErrCat::UnknownFailure, format!("{other}")),
    }
}

fn open_format(path: &Path) -> CResult<Box<dyn FormatReader>> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .map_err(|e| classify(&e, Stage::Probe))
}

fn codec_name(params: &symphonia::core::codecs::audio::AudioCodecParameters) -> String {
    symphonia::default::get_codecs()
        .get_audio_decoder(params.codec)
        .map(|r| r.codec.info.short_name.to_string())
        .unwrap_or_else(|| format!("unknown({:?})", params.codec))
}

fn track_info(ordinal: usize, t: &Track) -> Option<TrackInfo> {
    let params = match t.codec_params.as_ref() {
        Some(CodecParameters::Audio(p)) => p,
        _ => return None,
    };
    Some(TrackInfo {
        ordinal,
        track_id: Some(t.id.to_string()),
        stream_index: None,
        codec: codec_name(params),
        sample_rate: params.sample_rate,
        channels: params.channels.as_ref().map(|c| c.count() as u32),
        language: t.language.clone(),
        is_default: Some(t.flags.contains(TrackFlags::DEFAULT)),
        declared_frames: t.num_frames,
        delay: t.delay,
        padding: t.padding,
    })
}

/// (오디오 트랙 정보, 오디오가 아닌 트랙 수, 코덱 정보를 알 수 없는 트랙 수)
fn enumerate(format: &dyn FormatReader) -> (Vec<(TrackInfo, u32)>, usize, usize) {
    let mut audio = vec![];
    let (mut non_audio, mut unknown) = (0, 0);
    for t in format.tracks() {
        match t.codec_params.as_ref() {
            Some(CodecParameters::Audio(_)) => {
                let ord = audio.len();
                if let Some(ti) = track_info(ord, t) {
                    audio.push((ti, t.id));
                }
            }
            Some(_) => non_audio += 1,
            None => unknown += 1,
        }
    }
    (audio, non_audio, unknown)
}

/// 디코드 없이 트랙 목록만 조사한다.
pub fn list_tracks(path: &Path) -> CResult<(Vec<TrackInfo>, usize, usize)> {
    let format = open_format(path)?;
    let (audio, non_audio, unknown) = enumerate(format.as_ref());
    Ok((audio.into_iter().map(|(t, _)| t).collect(), non_audio, unknown))
}

pub fn run(input: &Path, output: &Path, track_ordinal: Option<usize>) -> CResult<(DecodeReport, PipelineReport)> {
    let mut format = open_format(input)?;
    let (audio, non_audio, unknown) = enumerate(format.as_ref());
    let mut rep = DecodeReport {
        decoder: "symphonia-0.6.1".into(),
        tracks: audio.iter().map(|(t, _)| t.clone()).collect(),
        non_audio_tracks: non_audio,
        ..Default::default()
    };
    if audio.is_empty() {
        return Err(if unknown > 0 {
            CodecError::new(ErrCat::UnsupportedCodec, format!("{unknown} track(s) have no usable codec parameters"))
        } else {
            CodecError::new(ErrCat::ParseFailure, "no audio track in container")
        });
    }
    // 기본 선택: DEFAULT 플래그가 있는 오디오 트랙, 없으면 첫 오디오 트랙
    let ordinal = match track_ordinal {
        Some(o) => o,
        None => format
            .default_track(TrackType::Audio)
            .and_then(|dt| audio.iter().position(|(_, id)| *id == dt.id))
            .unwrap_or(0),
    };
    let (sel, track_id) = match audio.get(ordinal) {
        Some((t, id)) => (t.clone(), *id),
        None => {
            return Err(if unknown > 0 {
                CodecError::new(ErrCat::UnsupportedCodec, format!("audio track ordinal {ordinal} not identifiable ({unknown} track(s) without codec parameters)"))
            } else {
                CodecError::new(ErrCat::ParseFailure, format!("audio track ordinal {ordinal} does not exist ({} audio tracks)", audio.len()))
            });
        }
    };
    rep.selected_ordinal = Some(ordinal);
    let params = format
        .tracks()
        .iter()
        .find(|t| t.id == track_id)
        .and_then(|t| t.codec_params.as_ref())
        .and_then(|p| p.audio())
        .cloned()
        .ok_or_else(|| CodecError::new(ErrCat::ParseFailure, "selected track lost its audio parameters"))?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|e| classify(&e, Stage::DecoderInit))?;

    let mut pipe: Option<PcmPipeline> = None;
    let mut samples: Vec<f32> = Vec::new();
    let mut eof_error: Option<String> = None;
    // 진단 전용(기본 꺼짐, 어느 후보의 동작도 아님): 환경변수 U1_DIAG_TRIM_START_FRAMES=N 이면
    // 디코드된 앞쪽 N 프레임(원본 샘플레이트 기준)을 버린다. MP4/AAC 의 priming delay(1024)를 직접 빼 보면
    // Symphonia 와 FFmpeg 출력의 차이가 "트리밍 누락"만으로 설명되는지 확인할 수 있다.
    let mut diag_skip: u64 = std::env::var("U1_DIAG_TRIM_START_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    if diag_skip > 0 {
        rep.warnings.push(format!("DIAGNOSTIC: dropping first {diag_skip} decoded frames (U1_DIAG_TRIM_START_FRAMES)"));
    }

    let result: CResult<()> = (|| {
        loop {
            let packet = match format.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => break,
                Err(Error::IoError(io)) if io.kind() == std::io::ErrorKind::UnexpectedEof => {
                    eof_error = Some(io.to_string());
                    break;
                }
                Err(Error::ResetRequired) => {
                    rep.warnings.push("ResetRequired from demuxer; stopped".into());
                    break;
                }
                Err(e) => return Err(classify(&e, Stage::Demux)),
            };
            rep.packets_total += 1;
            if packet.track_id != track_id {
                rep.packets_other_track += 1;
                continue;
            }
            match decoder.decode(&packet) {
                Ok(buf) => {
                    rep.packets_decoded += 1;
                    let spec = buf.spec();
                    let (rate, ch) = (spec.rate(), spec.channels().count() as u32);
                    if pipe.is_none() {
                        rep.src_rate = rate;
                        rep.src_channels = ch;
                        pipe = Some(PcmPipeline::create(output, rate, ch as usize)?);
                    } else if rate != rep.src_rate || ch != rep.src_channels {
                        return Err(CodecError::new(
                            ErrCat::ConversionFailure,
                            format!("stream format changed mid-stream: {}Hz/{}ch -> {rate}Hz/{ch}ch", rep.src_rate, rep.src_channels),
                        ));
                    }
                    samples.resize(buf.samples_interleaved(), 0.0f32);
                    buf.copy_to_slice_interleaved(&mut samples);
                    rep.src_frames += buf.frames() as u64;
                    let mut chunk: &[f32] = &samples;
                    if diag_skip > 0 {
                        let n = diag_skip.min(buf.frames() as u64);
                        chunk = &samples[n as usize * ch as usize..];
                        diag_skip -= n;
                    }
                    pipe.as_mut().unwrap().push_interleaved(chunk)?;
                }
                Err(Error::DecodeError(_)) => rep.decode_errors += 1,
                Err(Error::IoError(io)) if io.kind() == std::io::ErrorKind::UnexpectedEof => {
                    eof_error = Some(io.to_string());
                    break;
                }
                Err(e) => return Err(classify(&e, Stage::Decode)),
            }
        }
        Ok(())
    })();

    if let Err(e) = result {
        if let Some(p) = pipe.take() {
            p.abort();
        }
        return Err(e);
    }
    if let Some(m) = eof_error {
        if rep.src_frames == 0 {
            if let Some(p) = pipe.take() {
                p.abort();
            }
            return Err(CodecError::new(ErrCat::CorruptInput, format!("unexpected EOF before any audio: {m}")));
        }
        rep.warnings.push(format!("unexpected EOF (truncated input): {m}"));
    }
    if rep.decode_errors > 0 {
        rep.warnings.push(format!("{} packet(s) failed to decode and were skipped", rep.decode_errors));
    }
    let pipe = match pipe {
        Some(p) => p,
        None => {
            let _ = sel;
            return Err(CodecError::new(
                if rep.decode_errors > 0 { ErrCat::CorruptInput } else { ErrCat::ParseFailure },
                if rep.decode_errors > 0 { "no packet could be decoded".to_string() } else { "no audio packets found".to_string() },
            ));
        }
    };
    let pr = pipe.finish()?;
    Ok((rep, pr))
}
