//! U1 오디오 디코더·변환 기술검증 실험 (SubBridge phase0-audio-codec-lab).
//!
//! 세 실행 경로를 같은 CLI·같은 출력 규격으로 비교한다. 어느 경로도 사전에 채택하지 않았다.
//!   --mode symphonia | ffmpeg | hybrid | ffmpeg-native(참고용 기준)

mod ffmpeg_runner;
mod hybrid_runner;
mod input_probe;
mod matrix;
mod metrics;
mod output_validator;
mod pcm_pipeline;
mod symphonia_runner;

use std::path::PathBuf;
use std::time::Instant;

use clap::{Args, Parser, Subcommand};

use metrics::{platform_name, resource_usage, CodecError, DecodeReport, ErrCat, PipelineReport, RunRecord};

#[derive(Parser)]
#[command(name = "u1lab", about = "SubBridge U1: Symphonia / FFmpeg / Hybrid audio decode+convert validation")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 한 파일을 16kHz mono s16le WAV로 변환하고 JSON 기록 한 줄을 출력한다
    Run(RunArgs),
    /// 입력의 오디오 트랙 목록을 Symphonia와 ffprobe 양쪽으로 조사한다
    Probe {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value = "ffprobe")]
        ffprobe: String,
    },
    /// 출력 WAV를 검증한다
    Validate {
        #[arg(long)]
        wav: PathBuf,
        /// 기대 톤 "440:0.25,1000:0.25" (주파수:최대 진폭)
        #[arg(long)]
        tones: Option<String>,
    },
    /// 두 WAV를 비교한다 (샘플 수, RMS, 상관계수)
    Compare {
        #[arg(long)]
        a: PathBuf,
        #[arg(long)]
        b: PathBuf,
        #[arg(long, default_value_t = 2048)]
        max_lag: i32,
    },
    /// fixtures/catalog.csv 로부터 manifest.csv 를 만든다 (SHA-256, ffprobe 요약)
    Manifest(matrix::ManifestArgs),
    /// manifest.csv 의 SHA-256 과 크기를 현재 fixtures/generated 와 대조한다
    VerifyFixtures(matrix::VerifyArgs),
    /// 픽스처 × 모드 × 반복 실행 매트릭스를 돌리고 결과 CSV를 쓴다
    Matrix(matrix::MatrixArgs),
}

#[derive(Args, Clone)]
pub struct RunArgs {
    #[arg(long)]
    pub mode: String,
    #[arg(long)]
    pub input: PathBuf,
    #[arg(long)]
    pub output: PathBuf,
    /// 오디오 트랙 순번 (0-기반, 오디오 트랙만 셈). 생략하면 기본 트랙
    #[arg(long)]
    pub track: Option<usize>,
    #[arg(long, default_value = "ffmpeg")]
    pub ffmpeg: String,
    #[arg(long, default_value = "ffprobe")]
    pub ffprobe: String,
    #[arg(long)]
    pub fixture_id: Option<String>,
    #[arg(long)]
    pub json: bool,
}

struct Outcome {
    decode: Option<DecodeReport>,
    pipeline: Option<PipelineReport>,
    err: Option<CodecError>,
    decoder_used: String,
    fallback: bool,
    fallback_reason: Option<String>,
    sym_err: Option<CodecError>,
}

impl Outcome {
    fn from_result(r: Result<(DecodeReport, PipelineReport), CodecError>, used: &str) -> Self {
        match r {
            Ok((d, p)) => Outcome { decode: Some(d), pipeline: Some(p), err: None, decoder_used: used.into(), fallback: false, fallback_reason: None, sym_err: None },
            Err(e) => Outcome { decode: None, pipeline: None, err: Some(e), decoder_used: used.into(), fallback: false, fallback_reason: None, sym_err: None },
        }
    }
}

pub fn execute_run(a: &RunArgs) -> RunRecord {
    let mut rec = RunRecord {
        fixture_id: a.fixture_id.clone(),
        input: a.input.display().to_string(),
        mode: a.mode.clone(),
        platform: platform_name(),
        ..Default::default()
    };
    rec.input_sha256 = input_probe::sha256_file(&a.input).unwrap_or_else(|e| format!("error: {e}"));
    let u0 = resource_usage();
    let t0 = Instant::now();
    let out = match a.mode.as_str() {
        "symphonia" => Outcome::from_result(symphonia_runner::run(&a.input, &a.output, a.track), "symphonia"),
        "ffmpeg" => Outcome::from_result(ffmpeg_runner::run_common(&a.ffmpeg, &a.ffprobe, &a.input, &a.output, a.track), "ffmpeg"),
        "ffmpeg-native" => match ffmpeg_runner::run_native(&a.ffmpeg, &a.ffprobe, &a.input, &a.output, a.track) {
            Ok(d) => Outcome { decode: Some(d), pipeline: None, err: None, decoder_used: "ffmpeg-native".into(), fallback: false, fallback_reason: None, sym_err: None },
            Err(e) => Outcome { decode: None, pipeline: None, err: Some(e), decoder_used: "ffmpeg-native".into(), fallback: false, fallback_reason: None, sym_err: None },
        },
        "hybrid" => {
            let h = hybrid_runner::run(&a.ffmpeg, &a.ffprobe, &a.input, &a.output, a.track);
            let (decode, pipeline, err) = match h.result {
                Ok((d, p)) => (Some(d), Some(p), None),
                Err(e) => (None, None, Some(e)),
            };
            Outcome { decode, pipeline, err, decoder_used: h.decoder_used, fallback: h.fallback, fallback_reason: h.fallback_reason, sym_err: h.symphonia_error }
        }
        other => Outcome {
            decode: None,
            pipeline: None,
            err: Some(CodecError::new(ErrCat::UnknownFailure, format!("unknown mode '{other}'"))),
            decoder_used: "none".into(),
            fallback: false,
            fallback_reason: None,
            sym_err: None,
        },
    };
    rec.elapsed_s = t0.elapsed().as_secs_f64();
    let u1 = resource_usage();
    rec.peak_rss_self_kb = u1.peak_rss_self_kb;
    rec.peak_rss_children_kb = u1.peak_rss_children_kb;
    rec.cpu_user_s = u1.user_s.zip(u0.user_s).map(|(b, a)| b - a);
    rec.cpu_sys_s = u1.sys_s.zip(u0.sys_s).map(|(b, a)| b - a);
    rec.decoder_used = out.decoder_used;
    rec.fallback = out.fallback;
    rec.fallback_reason = out.fallback_reason;
    rec.symphonia_error_category = out.sym_err.as_ref().map(|e| e.cat.as_str().to_string());
    rec.symphonia_error_message = out.sym_err.as_ref().map(|e| e.message.clone());
    rec.decode = out.decode;
    rec.pipeline = out.pipeline;

    if let Some(e) = out.err {
        rec.success = false;
        rec.error_category = Some(e.cat.as_str().to_string());
        rec.error_message = Some(e.message);
        return rec;
    }
    // 출력 규격 검증 (모든 모드에 같은 검증기를 쓴다)
    let v = output_validator::validate(&a.output);
    rec.output = Some(a.output.display().to_string());
    rec.output_sha256 = input_probe::sha256_file(&a.output).ok();
    if let (Some(h), Some(s)) = (&v.header, &v.stats) {
        rec.out_sample_rate = Some(h.sample_rate);
        rec.out_channels = Some(h.channels as u32);
        rec.out_format = Some(format!("tag{}-{}bit", h.format_tag, h.bits));
        rec.out_samples = Some(s.samples);
        rec.out_duration_s = Some(v.duration_s);
        rec.out_bytes = Some(h.file_len);
        rec.peak = Some(s.peak);
        rec.rms = Some(s.rms);
        rec.dc_offset = Some(s.dc_offset);
        rec.full_scale_samples = Some(s.full_scale);
        if v.duration_s > 0.0 {
            rec.rtf = Some(rec.elapsed_s / v.duration_s);
        }
    }
    if v.ok {
        rec.success = true;
    } else {
        rec.success = false;
        rec.error_category = Some(ErrCat::ConversionFailure.as_str().to_string());
        rec.error_message = Some(format!("output validation failed: {}", v.problems.join("; ")));
    }
    if let Some(p) = &rec.pipeline {
        if p.nonfinite_samples > 0 {
            rec.notes.push(format!("{} non-finite input sample(s) replaced by 0", p.nonfinite_samples));
        }
    }
    rec
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Run(a) => {
            let rec = execute_run(&a);
            if a.json {
                println!("{}", serde_json::to_string(&rec).unwrap());
            } else {
                println!("{}", serde_json::to_string_pretty(&rec).unwrap());
            }
            std::process::exit(if rec.success { 0 } else { 1 });
        }
        Cmd::Probe { input, ffprobe } => {
            let sym = symphonia_runner::list_tracks(&input);
            let ff = ffmpeg_runner::list_tracks(&ffprobe, &input);
            let v = serde_json::json!({
                "symphonia": match &sym { Ok((t, n, u)) => serde_json::json!({"audio_tracks": t, "non_audio_tracks": n, "tracks_without_codec_params": u}), Err(e) => serde_json::json!({"error": e.to_string()}) },
                "ffprobe": match &ff { Ok((t, n)) => serde_json::json!({"audio_tracks": t, "non_audio_streams": n}), Err(e) => serde_json::json!({"error": e.to_string()}) },
            });
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        }
        Cmd::Validate { wav, tones } => {
            let v = output_validator::validate(&wav);
            let t = tones.as_deref().map(|s| output_validator::check_tones(&wav, s));
            let tv = match t {
                Some(Ok(list)) => serde_json::json!(list.iter().map(|(f, e, m)| serde_json::json!({"freq": f, "expected_amp": e, "measured_amp": m})).collect::<Vec<_>>()),
                Some(Err(e)) => serde_json::json!({"error": e}),
                None => serde_json::Value::Null,
            };
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({"validation": v, "tones": tv})).unwrap());
            std::process::exit(if v.ok { 0 } else { 1 });
        }
        Cmd::Compare { a, b, max_lag } => match output_validator::compare(&a, &b, max_lag) {
            Ok(r) => println!("{}", serde_json::to_string_pretty(&r).unwrap()),
            Err(e) => {
                eprintln!("compare failed: {e}");
                std::process::exit(1);
            }
        },
        Cmd::Manifest(a) => matrix::manifest(&a),
        Cmd::VerifyFixtures(a) => matrix::verify_fixtures(&a),
        Cmd::Matrix(a) => matrix::run_matrix(&a),
    }
}
