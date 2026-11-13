//! 경로 C: Symphonia 우선 + FFmpeg fallback.
//!
//! fallback 조건: Symphonia 오류가 `UnsupportedCodec` 또는 `UnsupportedContainer`일 때만.
//! 파일 손상(CorruptInput), 구문 분석 실패(ParseFailure), I/O 오류, 메모리 부족, 변환 실패,
//! 알 수 없는 오류는 FFmpeg로 재시도하지 않고 그대로 실패로 보고한다.
//!
//! 알려진 한계: Symphonia는 "알 수 없는 컨테이너"와 "손상되어 인식되지 않는 컨테이너"를
//! 구분하지 못해 후자도 UnsupportedContainer로 분류된다 → 이 경우 FFmpeg로 한 번 시도한다.
//! fallback이 일어난 이유와 Symphonia의 원래 오류를 모두 기록한다.

use std::path::Path;

use crate::ffmpeg_runner;
use crate::metrics::{CResult, CodecError, DecodeReport, PipelineReport};
use crate::symphonia_runner;

pub struct HybridOutcome {
    pub result: CResult<(DecodeReport, PipelineReport)>,
    pub decoder_used: String,
    pub fallback: bool,
    pub fallback_reason: Option<String>,
    pub symphonia_error: Option<CodecError>,
}

pub fn run(ffmpeg: &str, ffprobe_bin: &str, input: &Path, output: &Path, track: Option<usize>) -> HybridOutcome {
    match symphonia_runner::run(input, output, track) {
        Ok(r) => HybridOutcome { result: Ok(r), decoder_used: "symphonia".into(), fallback: false, fallback_reason: None, symphonia_error: None },
        Err(e) if e.cat.allows_fallback() => {
            let reason = format!("{} -> fallback to ffmpeg", e);
            let r = ffmpeg_runner::run_common(ffmpeg, ffprobe_bin, input, output, track);
            HybridOutcome { result: r, decoder_used: "ffmpeg".into(), fallback: true, fallback_reason: Some(reason), symphonia_error: Some(e) }
        }
        Err(e) => HybridOutcome {
            result: Err(e.clone()),
            decoder_used: "symphonia".into(),
            fallback: false,
            fallback_reason: Some(format!("no fallback: {} is not an allowed fallback category", e.cat.as_str())),
            symphonia_error: Some(e),
        },
    }
}
