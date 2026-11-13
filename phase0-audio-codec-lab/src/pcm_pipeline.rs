//! 공통 PCM 후처리 계층.
//!
//! Symphonia 경로와 FFmpeg 경로는 모두 디코더 출력(f32, 인터리브, 원본 샘플레이트·채널)을
//! 이 계층에 넣는다. 그래서 다운믹스·리샘플·포맷 변환·클리핑 처리가 두 경로에서 같다.
//!
//! 정책 (결과 보고서에도 그대로 기록한다):
//! - 다운믹스: 모든 채널의 단순 평균(가중치 1/N). 채널 레이아웃(LFE 등)은 구분하지 않는다.
//!   근거 없는 채널 가중치를 쓰지 않기 위한 기준선이며 권장안이 아니다.
//! - 리샘플: rubato 5.1.0 `Fft<f32>`(동기식 FFT), 입력 고정 1024 프레임, 기본 sub-chunk,
//!   BlackmanHarris2 창. 시작 지연은 제거한다. 출력 길이는 ceil(in_frames × 16000 / in_rate).
//! - 포맷 변환: x × 32768 을 반올림(0에서 먼 쪽)한 뒤 [-32768, 32767]로 포화한다.
//!   포화가 일어난 샘플 수를 클리핑으로 센다. 디더는 쓰지 않는다.
//! - 입력 샘플이 NaN/Inf이면 0으로 대체하고 개수를 센다.

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Indexing, Resampler};

use crate::metrics::{CResult, CodecError, ErrCat, PipelineReport};

pub const OUT_RATE: u32 = 16_000;
pub const DOWNMIX_POLICY: &str = "mean-of-all-channels (weights 1/N, no LFE/layout handling)";
pub const RESAMPLER_DESC: &str = "rubato 5.1.0 Fft<f32> FixedSync::Input chunk=1024 default-sub-chunks BlackmanHarris2; startup delay trimmed; out_len=ceil(in*16000/rate)";
pub const CLIP_POLICY: &str = "round-half-away(x*32768) then saturate to [-32768,32767]; saturation counted; no dither";

fn conv_err(msg: impl Into<String>) -> CodecError {
    CodecError::new(ErrCat::ConversionFailure, msg)
}

/// WAV(PCM, 16 kHz, mono, S16LE) 기록기. 길이는 마지막에 헤더에 채운다.
struct WavSink {
    w: BufWriter<File>,
    data_bytes: u64,
}

impl WavSink {
    fn create(path: &Path) -> std::io::Result<Self> {
        let mut w = BufWriter::new(File::create(path)?);
        w.write_all(&[0u8; 44])?;
        Ok(WavSink { w, data_bytes: 0 })
    }

    fn write_samples(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.w.write_all(bytes)?;
        self.data_bytes += bytes.len() as u64;
        Ok(())
    }

    fn finalize(mut self) -> CResult<()> {
        let data = self.data_bytes;
        if data + 36 > u32::MAX as u64 {
            return Err(conv_err("WAV 4 GiB limit exceeded"));
        }
        self.w.flush()?;
        let f = self.w.get_mut();
        f.seek(SeekFrom::Start(0))?;
        let mut h = Vec::with_capacity(44);
        h.extend_from_slice(b"RIFF");
        h.extend_from_slice(&((36 + data) as u32).to_le_bytes());
        h.extend_from_slice(b"WAVE");
        h.extend_from_slice(b"fmt ");
        h.extend_from_slice(&16u32.to_le_bytes());
        h.extend_from_slice(&1u16.to_le_bytes()); // PCM
        h.extend_from_slice(&1u16.to_le_bytes()); // mono
        h.extend_from_slice(&OUT_RATE.to_le_bytes());
        h.extend_from_slice(&(OUT_RATE * 2).to_le_bytes()); // byte rate
        h.extend_from_slice(&2u16.to_le_bytes()); // block align
        h.extend_from_slice(&16u16.to_le_bytes()); // bits
        h.extend_from_slice(b"data");
        h.extend_from_slice(&(data as u32).to_le_bytes());
        f.write_all(&h)?;
        f.flush()?;
        Ok(())
    }
}

/// 스트리밍 리샘플러. 입력 샘플레이트가 16 kHz이면 통과시킨다.
struct StreamResampler {
    rs: Option<Fft<f32>>,
    rate_in: u32,
    in_buf: Vec<f32>,
    in_tmp: Vec<f32>,
    out_tmp: Vec<f32>,
    to_trim: usize,
    total_in: u64,
    emitted: u64,
}

impl StreamResampler {
    fn new(rate_in: u32) -> CResult<Self> {
        let rs = if rate_in == OUT_RATE {
            None
        } else {
            let r = Fft::<f32>::new(rate_in as usize, OUT_RATE as usize, 1024, 1, FixedSync::Input)
                .map_err(|e| conv_err(format!("resampler construction failed: {e:?}")))?;
            Some(r)
        };
        let to_trim = rs.as_ref().map(|r| r.output_delay()).unwrap_or(0);
        Ok(StreamResampler {
            rs,
            rate_in,
            in_buf: Vec::new(),
            in_tmp: Vec::new(),
            out_tmp: Vec::new(),
            to_trim,
            total_in: 0,
            emitted: 0,
        })
    }

    fn expected_len(&self) -> u64 {
        let n = self.total_in as u128 * OUT_RATE as u128;
        let d = self.rate_in as u128;
        n.div_ceil(d) as u64
    }

    /// 입력 1 chunk(또는 마지막 부분 chunk / 무음 chunk)를 처리한다.
    fn process_one(&mut self, partial: Option<usize>, out: &mut Vec<f32>) -> CResult<()> {
        let Self { rs, in_buf, in_tmp, out_tmp, to_trim, emitted, .. } = self;
        let rs = rs.as_mut().expect("resampler present");
        let need = rs.input_frames_next();
        let out_max = rs.output_frames_max();
        if in_tmp.len() < need {
            in_tmp.resize(need, 0.0);
        }
        if out_tmp.len() < out_max {
            out_tmp.resize(out_max, 0.0);
        }
        let avail = match partial {
            None => need,
            Some(n) => n.min(need),
        };
        let have = avail.min(in_buf.len());
        in_tmp[..have].copy_from_slice(&in_buf[..have]);
        for v in in_tmp[have..need].iter_mut() {
            *v = 0.0;
        }
        let indexing = partial.map(|n| Indexing {
            input_offset: 0,
            output_offset: 0,
            partial_len: Some(n),
            active_channels_mask: None,
        });
        let (nin, nout) = {
            let inp = InterleavedSlice::new(&in_tmp[..need], 1, need)
                .map_err(|e| conv_err(format!("input adapter: {e:?}")))?;
            let mut outp = InterleavedSlice::new_mut(&mut out_tmp[..out_max], 1, out_max)
                .map_err(|e| conv_err(format!("output adapter: {e:?}")))?;
            rs.process_into_buffer(&inp, &mut outp, indexing.as_ref())
                .map_err(|e| conv_err(format!("resample failed: {e:?}")))?
        };
        match partial {
            None => {
                in_buf.drain(..nin.min(in_buf.len()));
            }
            Some(_) => in_buf.clear(),
        }
        let skip = (*to_trim).min(nout);
        *to_trim -= skip;
        out.extend_from_slice(&out_tmp[skip..nout]);
        *emitted += (nout - skip) as u64;
        Ok(())
    }

    fn push(&mut self, mono: &[f32], out: &mut Vec<f32>) -> CResult<()> {
        self.total_in += mono.len() as u64;
        if self.rs.is_none() {
            out.extend_from_slice(mono);
            self.emitted += mono.len() as u64;
            return Ok(());
        }
        self.in_buf.extend_from_slice(mono);
        loop {
            let need = self.rs.as_ref().unwrap().input_frames_next();
            if self.in_buf.len() < need {
                break;
            }
            self.process_one(None, out)?;
        }
        Ok(())
    }

    fn finish(&mut self, out: &mut Vec<f32>) -> CResult<()> {
        if self.rs.is_none() {
            return Ok(());
        }
        let expected = self.expected_len();
        if !self.in_buf.is_empty() {
            let n = self.in_buf.len();
            self.process_one(Some(n), out)?;
        }
        let mut guard = 0u32;
        while self.emitted < expected {
            self.process_one(Some(0), out)?;
            guard += 1;
            if guard > 100_000 {
                return Err(conv_err("resampler flush did not converge"));
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct StatAcc {
    n: u64,
    sum: f64,
    sum_sq: f64,
    peak: i32,
    clipped: u64,
}

pub struct PcmPipeline {
    channels: usize,
    rs: StreamResampler,
    sink: WavSink,
    path: PathBuf,
    mono: Vec<f32>,
    f_out: Vec<f32>,
    bytes: Vec<u8>,
    acc: StatAcc,
    in_frames: u64,
    nonfinite: u64,
    written: u64,
}

impl PcmPipeline {
    pub fn create(path: &Path, rate: u32, channels: usize) -> CResult<Self> {
        if rate == 0 || channels == 0 {
            return Err(conv_err(format!("invalid source format: rate={rate} channels={channels}")));
        }
        let sink = WavSink::create(path)?;
        Ok(PcmPipeline {
            channels,
            rs: StreamResampler::new(rate)?,
            sink,
            path: path.to_path_buf(),
            mono: Vec::new(),
            f_out: Vec::new(),
            bytes: Vec::new(),
            acc: StatAcc::default(),
            in_frames: 0,
            nonfinite: 0,
            written: 0,
        })
    }

    /// 인터리브 f32 샘플(원본 샘플레이트, 원본 채널 수)을 넣는다.
    pub fn push_interleaved(&mut self, samples: &[f32]) -> CResult<()> {
        let ch = self.channels;
        let frames = samples.len() / ch;
        self.in_frames += frames as u64;
        self.mono.clear();
        for f in samples.chunks_exact(ch) {
            let mut s = 0.0f32;
            for &x in f {
                if x.is_finite() {
                    s += x;
                } else {
                    self.nonfinite += 1;
                }
            }
            self.mono.push(s / ch as f32);
        }
        let mono = std::mem::take(&mut self.mono);
        let r = self.rs.push(&mono, &mut self.f_out);
        self.mono = mono;
        r?;
        self.write_out(None)
    }

    fn write_out(&mut self, limit: Option<u64>) -> CResult<()> {
        if let Some(max_total) = limit {
            let allowed = max_total.saturating_sub(self.written) as usize;
            if self.f_out.len() > allowed {
                self.f_out.truncate(allowed);
            }
        }
        self.bytes.clear();
        for &x in &self.f_out {
            let v = (x * 32768.0).round();
            let (s, clipped) = if v > 32767.0 {
                (32767i32, true)
            } else if v < -32768.0 {
                (-32768i32, true)
            } else {
                (v as i32, false)
            };
            self.bytes.extend_from_slice(&(s as i16).to_le_bytes());
            self.acc.n += 1;
            self.acc.sum += s as f64;
            self.acc.sum_sq += (s as f64) * (s as f64);
            self.acc.peak = self.acc.peak.max(s.abs());
            if clipped {
                self.acc.clipped += 1;
            }
        }
        self.written += self.f_out.len() as u64;
        self.f_out.clear();
        let bytes = std::mem::take(&mut self.bytes);
        let r = self.sink.write_samples(&bytes);
        self.bytes = bytes;
        r?;
        Ok(())
    }

    pub fn finish(mut self) -> CResult<PipelineReport> {
        let expected = self.rs.expected_len();
        let before = self.written;
        self.rs.finish(&mut self.f_out)?;
        // 리샘플러가 기대 길이보다 더 낸 만큼은 잘라낸다(시작 지연을 제거한 뒤 남는 꼬리).
        let pending = self.f_out.len() as u64;
        let excess = (before + pending).saturating_sub(expected);
        self.write_out(Some(expected))?;
        let report = PipelineReport {
            in_frames: self.in_frames,
            expected_out_samples: expected,
            out_samples: self.written,
            excess_removed: excess,
            clipped_samples: self.acc.clipped,
            nonfinite_samples: self.nonfinite,
            downmix: DOWNMIX_POLICY.to_string(),
            resampler: if self.rs.rs.is_some() { RESAMPLER_DESC.to_string() } else { "passthrough (source already 16 kHz)".to_string() },
            clip_policy: CLIP_POLICY.to_string(),
        };
        self.sink.finalize()?;
        Ok(report)
    }

    /// 실패했을 때 부분 출력 파일을 지운다 (이 프로그램이 만든 출력 파일만).
    pub fn abort(self) {
        let p = self.path.clone();
        drop(self);
        let _ = std::fs::remove_file(p);
    }
}
