//! 공통 타입: 오류 분류, 실행 기록, 자원 사용량.
//!
//! 이 모듈은 세 실행 경로(Symphonia / FFmpeg / Hybrid)가 같은 형식으로 결과를 남기게 한다.

use serde::{Deserialize, Serialize};
use std::fmt;

/// 요구사항 §4의 오류 분류 8종. 분류할 수 없으면 `UnknownFailure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrCat {
    UnsupportedCodec,
    UnsupportedContainer,
    /// 컨테이너(헤더·atom·EBML 등) 수준 구문 분석 실패
    ParseFailure,
    /// 비트스트림 수준 손상, 잘린 입력(예기치 않은 EOF), 한도 초과
    CorruptInput,
    IoError,
    OutOfMemory,
    /// 변환(리샘플·포맷·WAV 기록) 단계 실패
    ConversionFailure,
    UnknownFailure,
}

impl ErrCat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrCat::UnsupportedCodec => "UnsupportedCodec",
            ErrCat::UnsupportedContainer => "UnsupportedContainer",
            ErrCat::ParseFailure => "ParseFailure",
            ErrCat::CorruptInput => "CorruptInput",
            ErrCat::IoError => "IoError",
            ErrCat::OutOfMemory => "OutOfMemory",
            ErrCat::ConversionFailure => "ConversionFailure",
            ErrCat::UnknownFailure => "UnknownFailure",
        }
    }

    /// Hybrid가 FFmpeg fallback을 허용하는 범주.
    /// 파일 손상(CorruptInput), 메모리 부족(OutOfMemory), I/O 오류 등은 fallback하지 않는다.
    pub fn allows_fallback(&self) -> bool {
        matches!(self, ErrCat::UnsupportedCodec | ErrCat::UnsupportedContainer)
    }
}

#[derive(Debug, Clone)]
pub struct CodecError {
    pub cat: ErrCat,
    pub message: String,
}

impl CodecError {
    pub fn new(cat: ErrCat, message: impl Into<String>) -> Self {
        CodecError { cat, message: message.into() }
    }
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.cat.as_str(), self.message)
    }
}

impl std::error::Error for CodecError {}

impl From<std::io::Error> for CodecError {
    fn from(e: std::io::Error) -> Self {
        CodecError::new(ErrCat::IoError, e.to_string())
    }
}

pub type CResult<T> = Result<T, CodecError>;

/// 입력 파일의 오디오 트랙 1개. `ordinal`은 오디오 트랙만 센 0-기반 순번(사용자 선택용 식별자).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrackInfo {
    pub ordinal: usize,
    /// 컨테이너가 부여한 트랙 ID (Symphonia `Track.id` 또는 ffprobe `id`)
    pub track_id: Option<String>,
    /// ffprobe의 전체 스트림 인덱스 (FFmpeg 경로에서만)
    pub stream_index: Option<u64>,
    pub codec: String,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub language: Option<String>,
    pub is_default: Option<bool>,
    /// 컨테이너가 선언한 재생 프레임 수 (지연·패딩 제외, 있을 때만)
    pub declared_frames: Option<u64>,
    pub delay: Option<u32>,
    pub padding: Option<u32>,
}

/// 디코더 단계 보고
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecodeReport {
    pub decoder: String,
    pub tracks: Vec<TrackInfo>,
    /// 컨테이너에 있던 오디오가 아닌 트랙 수(영상 등). 디코드하지 않는다.
    pub non_audio_tracks: usize,
    pub selected_ordinal: Option<usize>,
    pub src_rate: u32,
    pub src_channels: u32,
    /// 디코더가 낸 원본 샘플레이트 기준 프레임 수
    pub src_frames: u64,
    pub packets_total: u64,
    pub packets_decoded: u64,
    pub packets_other_track: u64,
    pub decode_errors: u64,
    pub warnings: Vec<String>,
}

/// 공통 PCM 후처리 계층 보고
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineReport {
    pub in_frames: u64,
    pub expected_out_samples: u64,
    pub out_samples: u64,
    pub excess_removed: u64,
    pub clipped_samples: u64,
    pub nonfinite_samples: u64,
    pub downmix: String,
    pub resampler: String,
    pub clip_policy: String,
}

/// 한 번의 `run` 실행 결과(JSON 한 줄로 출력)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunRecord {
    pub fixture_id: Option<String>,
    pub input: String,
    pub input_sha256: String,
    pub mode: String,
    pub decoder_used: String,
    pub fallback: bool,
    pub fallback_reason: Option<String>,
    pub symphonia_error_category: Option<String>,
    pub symphonia_error_message: Option<String>,
    pub output: Option<String>,
    pub output_sha256: Option<String>,
    pub success: bool,
    pub error_category: Option<String>,
    pub error_message: Option<String>,
    pub decode: Option<DecodeReport>,
    pub pipeline: Option<PipelineReport>,
    pub out_sample_rate: Option<u32>,
    pub out_channels: Option<u32>,
    pub out_format: Option<String>,
    pub out_samples: Option<u64>,
    pub out_duration_s: Option<f64>,
    pub out_bytes: Option<u64>,
    pub peak: Option<f64>,
    pub rms: Option<f64>,
    pub dc_offset: Option<f64>,
    pub full_scale_samples: Option<u64>,
    pub elapsed_s: f64,
    pub peak_rss_self_kb: Option<u64>,
    pub peak_rss_children_kb: Option<u64>,
    pub cpu_user_s: Option<f64>,
    pub cpu_sys_s: Option<f64>,
    pub rtf: Option<f64>,
    pub platform: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceUsage {
    pub peak_rss_self_kb: Option<u64>,
    pub peak_rss_children_kb: Option<u64>,
    pub user_s: Option<f64>,
    pub sys_s: Option<f64>,
}

/// 프로세스 자원 사용량.
/// - unix: `getrusage`. macOS의 `ru_maxrss`는 **바이트**, Linux는 KB이므로 KB로 환산한다.
/// - windows: 최대 working set만 조회한다(자식 프로세스·CPU 시간은 미측정).
///   Windows 경로는 이 저장소에서 컴파일하지 못했다(NOT_RUN).
#[cfg(unix)]
pub fn resource_usage() -> ResourceUsage {
    fn tv(t: libc::timeval) -> f64 {
        t.tv_sec as f64 + t.tv_usec as f64 / 1e6
    }
    fn rss_kb(v: i64) -> u64 {
        #[cfg(target_os = "macos")]
        {
            (v as u64) / 1024
        }
        #[cfg(not(target_os = "macos"))]
        {
            v as u64
        }
    }
    unsafe {
        let mut me: libc::rusage = std::mem::zeroed();
        let mut ch: libc::rusage = std::mem::zeroed();
        let ok_me = libc::getrusage(libc::RUSAGE_SELF, &mut me) == 0;
        let ok_ch = libc::getrusage(libc::RUSAGE_CHILDREN, &mut ch) == 0;
        ResourceUsage {
            peak_rss_self_kb: ok_me.then(|| rss_kb(me.ru_maxrss as i64)),
            peak_rss_children_kb: ok_ch.then(|| rss_kb(ch.ru_maxrss as i64)),
            user_s: ok_me.then(|| tv(me.ru_utime) + if ok_ch { tv(ch.ru_utime) } else { 0.0 }),
            sys_s: ok_me.then(|| tv(me.ru_stime) + if ok_ch { tv(ch.ru_stime) } else { 0.0 }),
        }
    }
}

#[cfg(windows)]
pub fn resource_usage() -> ResourceUsage {
    #[repr(C)]
    #[allow(non_snake_case)]
    struct ProcessMemoryCounters {
        cb: u32,
        PageFaultCount: u32,
        PeakWorkingSetSize: usize,
        WorkingSetSize: usize,
        QuotaPeakPagedPoolUsage: usize,
        QuotaPagedPoolUsage: usize,
        QuotaPeakNonPagedPoolUsage: usize,
        QuotaNonPagedPoolUsage: usize,
        PagefileUsage: usize,
        PeakPagefileUsage: usize,
    }
    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(h: isize, p: *mut ProcessMemoryCounters, cb: u32) -> i32;
    }
    unsafe {
        let mut c: ProcessMemoryCounters = std::mem::zeroed();
        c.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
        let ok = K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) != 0;
        ResourceUsage {
            peak_rss_self_kb: ok.then(|| (c.PeakWorkingSetSize / 1024) as u64),
            peak_rss_children_kb: None,
            user_s: None,
            sys_s: None,
        }
    }
}

#[cfg(not(any(unix, windows)))]
pub fn resource_usage() -> ResourceUsage {
    ResourceUsage::default()
}

pub fn platform_name() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}
