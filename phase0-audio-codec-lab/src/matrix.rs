//! 매트릭스 실행기: 픽스처 × 모드 × 반복 실행, 결과 CSV 작성, manifest 생성/검증.
//!
//! 각 실행은 **별도 프로세스**(`u1lab run`)로 돌린다. 그래야 실행마다 최대 메모리(ru_maxrss)와
//! CPU 시간이 독립적으로 측정된다. 실행 순서는 (반복) → (모드) 순으로 돌려
//! 시스템 부하 변동이 한 모드에 몰리지 않게 한다.
//!
//! 판정 기준은 모두 이 실험이 정한 가정이며 표준이 아니다 (README 참조):
//! - 시간축: 무손실은 ±2 샘플(16 kHz 기준), 손실 압축은 ±ceil(3 × 코덱 프레임 크기 × 16000 / 원본 레이트) 샘플
//! - 기대 RMS/톤 진폭: 무손실 상대 오차 2%, 손실 압축 10% (절대 오차 하한 0.0005 / 0.002)

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use clap::Args;

use crate::input_probe::{ffprobe, sha256_file};
use crate::metrics::{platform_name, RunRecord};
use crate::output_validator;

#[derive(Args, Clone)]
pub struct ManifestArgs {
    #[arg(long, default_value = "fixtures")]
    pub dir: PathBuf,
    #[arg(long, default_value = "ffprobe")]
    pub ffprobe: String,
}

#[derive(Args, Clone)]
pub struct VerifyArgs {
    #[arg(long, default_value = "fixtures")]
    pub dir: PathBuf,
}

#[derive(Args, Clone)]
pub struct MatrixArgs {
    #[arg(long, default_value = "fixtures")]
    pub dir: PathBuf,
    /// compat | perf | all  (perf = group 'perf')
    #[arg(long, default_value = "compat")]
    pub set: String,
    #[arg(long, default_value = "symphonia,ffmpeg,hybrid,ffmpeg-native")]
    pub modes: String,
    #[arg(long, default_value_t = 3)]
    pub repeats: usize,
    /// 쉼표로 구분한 픽스처 ID만 실행
    #[arg(long)]
    pub only: Option<String>,
    #[arg(long, default_value = "results/macos")]
    pub results: PathBuf,
    #[arg(long, default_value = "ffmpeg")]
    pub ffmpeg: String,
    #[arg(long, default_value = "ffprobe")]
    pub ffprobe: String,
    /// 성능 세트에서도 출력 WAV를 지우지 않는다
    #[arg(long)]
    pub keep_outputs: bool,
    /// CSV 파일 이름에 붙일 접미사 (예: _perf)
    #[arg(long, default_value = "")]
    pub suffix: String,
}

#[derive(Debug, Clone)]
struct Tone {
    track: Option<usize>,
    freq: f64,
    amp: f64,
}

#[derive(Debug, Clone)]
struct CatRow {
    id: String,
    file: String,
    group: String,
    description: String,
    nominal: Option<f64>,
    expected: String,
    lossless: bool,
    frame_size: u32,
    expected_rms: Option<f64>,
    tones: Vec<Tone>,
    tracks: Vec<Option<usize>>,
}

fn load_catalog(dir: &Path) -> Vec<CatRow> {
    let text = fs::read_to_string(dir.join("catalog.csv")).expect("cannot read catalog.csv");
    let mut rows = vec![];
    for (i, line) in text.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        assert!(f.len() == 11, "catalog.csv line {} has {} fields (expected 11)", i + 1, f.len());
        let tones = f[9]
            .split(';')
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                let (track, rest) = match s.split_once('@') {
                    Some((t, r)) => (t.trim_start_matches('t').parse::<usize>().ok(), r),
                    None => (None, s),
                };
                let (fr, am) = rest.split_once(':').expect("tone spec freq:amp");
                Tone { track, freq: fr.parse().unwrap(), amp: am.parse().unwrap() }
            })
            .collect();
        let tracks = f[10]
            .split('|')
            .map(|t| if t == "default" { None } else { Some(t.parse::<usize>().unwrap()) })
            .collect();
        rows.push(CatRow {
            id: f[0].into(),
            file: f[1].into(),
            group: f[2].into(),
            description: f[3].into(),
            nominal: f[4].parse().ok(),
            expected: f[5].into(),
            lossless: f[6] == "1",
            frame_size: f[7].parse().unwrap_or(0),
            expected_rms: f[8].parse().ok(),
            tones,
            tracks,
        });
    }
    rows
}

fn q(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn opt<T: ToString>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

fn f6(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.6}")).unwrap_or_default()
}

// ---------------------------------------------------------------- manifest / verify

pub fn manifest(a: &ManifestArgs) {
    let rows = load_catalog(&a.dir);
    let gen_line = fs::read_to_string(a.dir.join("generated/GENERATOR.txt")).unwrap_or_default().lines().next().unwrap_or("").to_string();
    let mut out = String::from("id,file,group,bytes,sha256,container,audio_tracks,first_audio_codec,sample_rate,channels,format_duration_s,nominal_duration_s,description,generator\n");
    let mut missing = 0;
    for r in &rows {
        let p = a.dir.join("generated").join(&r.file);
        if !p.exists() {
            missing += 1;
            eprintln!("MISSING: {}", p.display());
            out.push_str(&format!("{},{},{},,,,,,,,,{},{},{}\n", r.id, r.file, r.group, opt(r.nominal), q(&r.description), q(&gen_line)));
            continue;
        }
        let bytes = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        let sha = sha256_file(&p).unwrap_or_default();
        let (cont, ntr, codec, sr, ch, dur) = match ffprobe(&a.ffprobe, &p) {
            Ok(fp) => {
                let au = fp.audio_streams();
                let first = au.first();
                (
                    fp.format_name.replace(',', "+"),
                    au.len().to_string(),
                    first.map(|s| s.codec_name.clone()).unwrap_or_default(),
                    opt(first.and_then(|s| s.sample_rate)),
                    opt(first.and_then(|s| s.channels)),
                    f6(fp.duration_s),
                )
            }
            Err(e) => (format!("ffprobe-error:{}", e.cat.as_str()), String::new(), String::new(), String::new(), String::new(), String::new()),
        };
        out.push_str(&format!("{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n", r.id, r.file, r.group, bytes, sha, cont, ntr, codec, sr, ch, dur, opt(r.nominal), q(&r.description), q(&gen_line)));
    }
    fs::write(a.dir.join("manifest.csv"), out).expect("write manifest.csv");
    println!("manifest.csv written ({} fixtures, {} missing)", rows.len(), missing);
}

pub fn verify_fixtures(a: &VerifyArgs) {
    let text = fs::read_to_string(a.dir.join("manifest.csv")).expect("cannot read manifest.csv");
    let (mut ok, mut bad) = (0, 0);
    for (i, line) in text.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        let (id, file, bytes, sha) = (f[0], f[1], f[3], f[4]);
        let p = a.dir.join("generated").join(file);
        let got_bytes = fs::metadata(&p).map(|m| m.len().to_string()).unwrap_or_else(|_| "MISSING".into());
        let got_sha = sha256_file(&p).unwrap_or_else(|_| "MISSING".into());
        if got_bytes == bytes && got_sha == sha {
            ok += 1;
        } else {
            bad += 1;
            println!("MISMATCH {id} {file}: bytes {got_bytes} vs {bytes}; sha {} vs {}", &got_sha[..got_sha.len().min(12)], &sha[..sha.len().min(12)]);
        }
    }
    println!("verify-fixtures on {}: {} ok, {} mismatched", platform_name(), ok, bad);
    std::process::exit(if bad == 0 { 0 } else { 1 });
}

// ---------------------------------------------------------------- matrix

struct Run {
    track: Option<usize>,
    mode: String,
    idx: usize,
    rec: RunRecord,
}

fn crash_record(id: &str, mode: &str, why: String) -> RunRecord {
    RunRecord {
        fixture_id: Some(id.into()),
        mode: mode.into(),
        success: false,
        error_category: Some("UnknownFailure".into()),
        error_message: Some(why),
        platform: platform_name(),
        notes: vec!["RUNNER_CRASH_OR_TIMEOUT".into()],
        ..Default::default()
    }
}

fn spawn_run(exe: &Path, args: &[String], id: &str, mode: &str, timeout: Duration) -> RunRecord {
    let mut child = match Command::new(exe).args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => return crash_record(id, mode, format!("spawn failed: {e}")),
    };
    let mut so = child.stdout.take().unwrap();
    let mut se = child.stderr.take().unwrap();
    let t_out = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = so.read_to_string(&mut s);
        s
    });
    let t_err = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = se.read_to_string(&mut s);
        s
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Some(st),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => break None,
        }
    };
    let out = t_out.join().unwrap_or_default();
    let err = t_err.join().unwrap_or_default();
    let last = out.lines().rev().find(|l| l.trim_start().starts_with('{'));
    match (last, status) {
        (Some(l), _) => serde_json::from_str::<RunRecord>(l).unwrap_or_else(|e| crash_record(id, mode, format!("bad JSON from runner: {e}"))),
        (None, Some(st)) => crash_record(id, mode, format!("runner exited with {:?} and no JSON; stderr: {}", st.code(), err.lines().next().unwrap_or(""))),
        (None, None) => crash_record(id, mode, format!("timeout after {timeout:?}")),
    }
}

fn compat_verdict(expected: &str, runs: &[&Run]) -> (&'static str, String) {
    let n = runs.len();
    let ok = runs.iter().filter(|r| r.rec.success).count();
    let crashed = runs.iter().any(|r| r.rec.notes.iter().any(|x| x == "RUNNER_CRASH_OR_TIMEOUT"));
    if crashed {
        return ("FAIL", "runner crashed or timed out".into());
    }
    match expected {
        "ok" => {
            if ok == n {
                ("PASS", String::new())
            } else if ok == 0 {
                ("FAIL", "all runs failed".into())
            } else {
                ("FAIL", "inconsistent across repeats".into())
            }
        }
        "error" => {
            if ok == 0 {
                ("PASS", "failed cleanly as expected".into())
            } else {
                ("FAIL", "expected failure but produced output".into())
            }
        }
        _ => {
            if ok == n {
                ("PASS", "recovered and produced valid output".into())
            } else if ok == 0 {
                ("PASS", "failed cleanly".into())
            } else {
                ("FAIL", "inconsistent across repeats".into())
            }
        }
    }
}

pub fn run_matrix(a: &MatrixArgs) {
    let rows = load_catalog(&a.dir);
    let exe = std::env::current_exe().expect("current_exe");
    let modes: Vec<String> = a.modes.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let only: Option<Vec<String>> = a.only.as_ref().map(|s| s.split(',').map(|x| x.trim().to_string()).collect());
    let outputs = a.results.join("outputs");
    fs::create_dir_all(&outputs).expect("create outputs dir");
    let sel: Vec<usize> = (0..rows.len())
        .filter(|&i| match a.set.as_str() {
            "perf" => rows[i].group == "perf",
            "all" => true,
            _ => rows[i].group != "perf",
        })
        .filter(|&i| only.as_ref().map(|o| o.contains(&rows[i].id)).unwrap_or(true))
        .collect();

    let mut compat = String::from("platform,fixture_id,group,track,mode,expected_outcome,runs_ok,runs,decoder_used,fallback,fallback_reason,symphonia_error_category,error_category,error_message,verdict,note\n");
    let mut quality = String::from("platform,fixture_id,track,mode,src_rate,src_channels,src_frames,out_samples,out_duration_s,nominal_duration_s,time_axis_diff_samples,time_axis_diff_ms,tolerance_samples,time_axis_verdict,peak,rms,dc_offset,full_scale_samples,clipped_samples,nonfinite_samples,expected_rms,rms_check,tone_check,repeat_sha256,verdict\n");
    let mut compare = String::from("platform,fixture_id,track,pair,samples_a,samples_b,diff_samples,rms_a,rms_b,peak_a,peak_b,corr_lag0,best_lag_samples,corr_best,rel_diff_rms_best,window_samples\n");
    let mut perf = String::from("platform,fixture_id,track,mode,run,run_kind,input_bytes,audio_duration_s,elapsed_s,rtf,peak_rss_self_kb,peak_rss_children_kb,cpu_user_s,cpu_sys_s,cpu_pct,out_bytes,success,decoder_used,fallback\n");
    let mut raw = String::new();
    let plat = platform_name();

    for &ri in &sel {
        let row = &rows[ri];
        let input = a.dir.join("generated").join(&row.file);
        if !input.exists() {
            eprintln!("skip {} (missing file)", row.id);
            continue;
        }
        let in_bytes = fs::metadata(&input).map(|m| m.len()).unwrap_or(0);
        // 모든 모드가 같은 조건(페이지 캐시에 올라간 입력)에서 시작하도록 한 번 읽어 둔다.
        let _ = fs::read(&input);
        let timeout = Duration::from_secs(if row.group == "perf" { 3600 } else { 300 });
        let mut runs: Vec<Run> = vec![];
        for &track in &row.tracks {
            for rep in 1..=a.repeats {
                for mode in &modes {
                    let tag = track.map(|t| format!("t{t}")).unwrap_or_else(|| "tdef".into());
                    let dir = outputs.join(&row.id);
                    fs::create_dir_all(&dir).expect("create fixture output dir");
                    let out = dir.join(format!("{mode}_{tag}.wav"));
                    let mut args = vec![
                        "run".to_string(), "--mode".into(), mode.clone(), "--input".into(), input.display().to_string(),
                        "--output".into(), out.display().to_string(), "--fixture-id".into(), row.id.clone(),
                        "--ffmpeg".into(), a.ffmpeg.clone(), "--ffprobe".into(), a.ffprobe.clone(), "--json".into(),
                    ];
                    if let Some(t) = track {
                        args.push("--track".into());
                        args.push(t.to_string());
                    }
                    let rec = spawn_run(&exe, &args, &row.id, mode, timeout);
                    raw.push_str(&serde_json::to_string(&serde_json::json!({"track": track, "run": rep, "record": rec})).unwrap());
                    raw.push('\n');
                    runs.push(Run { track, mode: mode.clone(), idx: rep, rec });
                }
            }
        }
        // ---- 분석
        for &track in &row.tracks {
            let tlabel = opt(track);
            let mut outs: Vec<(String, PathBuf)> = vec![];
            for mode in &modes {
                let rs: Vec<&Run> = runs.iter().filter(|r| r.track == track && &r.mode == mode).collect();
                if rs.is_empty() {
                    continue;
                }
                let r1 = &rs[0].rec;
                let (verdict, note) = compat_verdict(&row.expected, &rs);
                compat.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                    plat, row.id, row.group, tlabel, mode, row.expected, rs.iter().filter(|r| r.rec.success).count(), rs.len(),
                    r1.decoder_used, r1.fallback, q(&opt(r1.fallback_reason.clone())), opt(r1.symphonia_error_category.clone()),
                    opt(r1.error_category.clone()), q(&r1.error_message.clone().unwrap_or_default().chars().take(160).collect::<String>()), verdict, q(&note)
                ));
                for r in &rs {
                    let rec = &r.rec;
                    let dur = rec.out_duration_s;
                    let cpu = match (rec.cpu_user_s, rec.cpu_sys_s) {
                        (Some(u), Some(s)) if rec.elapsed_s > 0.0 => Some((u + s) / rec.elapsed_s * 100.0),
                        _ => None,
                    };
                    perf.push_str(&format!(
                        "{},{},{},{},{},{},{},{},{:.4},{},{},{},{},{},{},{},{},{},{}\n",
                        plat, row.id, tlabel, mode, r.idx, if r.idx == 1 { "first" } else { "repeat" }, in_bytes, f6(dur.or(row.nominal)),
                        rec.elapsed_s, f6(rec.rtf), opt(rec.peak_rss_self_kb), opt(rec.peak_rss_children_kb), f6(rec.cpu_user_s), f6(rec.cpu_sys_s),
                        f6(cpu), opt(rec.out_bytes), rec.success, rec.decoder_used, rec.fallback
                    ));
                }
                if !r1.success {
                    continue;
                }
                let dec = r1.decode.clone().unwrap_or_default();
                let out_samples = r1.out_samples.unwrap_or(0) as i64;
                let (diff, diff_ms, tol, tv) = match row.nominal {
                    Some(nom) => {
                        let exp = (nom * 16000.0).round() as i64;
                        let d = out_samples - exp;
                        let tol = if row.lossless || row.frame_size == 0 || dec.src_rate == 0 {
                            2
                        } else {
                            (3.0 * row.frame_size as f64 * 16000.0 / dec.src_rate as f64).ceil() as i64
                        };
                        (Some(d), Some(d as f64 / 16.0), Some(tol), if d.abs() <= tol { "PASS" } else { "FAIL" })
                    }
                    None => (None, None, None, "NOT_APPLICABLE"),
                };
                let rel = if row.lossless { 0.02 } else { 0.10 };
                let rms_check = match (row.expected_rms, r1.rms) {
                    (Some(e), Some(m)) => {
                        let err = (m - e).abs();
                        if err <= (rel * e).max(0.0005) { "PASS" } else { "FAIL" }
                    }
                    _ => "NOT_APPLICABLE",
                };
                let tone_spec: Vec<String> = row
                    .tones
                    .iter()
                    .filter(|t| t.track.is_none() || t.track == track || (t.track == Some(0) && track.is_none()))
                    .map(|t| format!("{}:{}", t.freq, t.amp))
                    .collect();
                let out_path = r1.output.clone().map(PathBuf::from);
                let tone_check = if tone_spec.is_empty() {
                    "NOT_APPLICABLE".to_string()
                } else if let Some(p) = &out_path {
                    match output_validator::check_tones(p, &tone_spec.join(",")) {
                        Ok(list) => {
                            let all = list.iter().all(|(_, e, m)| (m - e).abs() <= (rel * e).max(0.002));
                            let detail: Vec<String> = list.iter().map(|(f, e, m)| format!("{f:.0}Hz exp{e:.3}/got{m:.3}")).collect();
                            format!("{} [{}]", if all { "PASS" } else { "FAIL" }, detail.join(" "))
                        }
                        Err(e) => format!("ERROR {e}"),
                    }
                } else {
                    "NOT_APPLICABLE".into()
                };
                let shas: Vec<String> = rs.iter().filter_map(|r| r.rec.output_sha256.clone()).collect();
                let distinct = { let mut s = shas.clone(); s.sort(); s.dedup(); s.len() };
                let repeat = if shas.len() == rs.len() && distinct == 1 { format!("identical({})", &shas[0][..12]) } else { format!("DIFFERENT({} distinct of {})", distinct, shas.len()) };
                let p = r1.pipeline.clone().unwrap_or_default();
                let all_pass = tv != "FAIL" && rms_check != "FAIL" && !tone_check.starts_with("FAIL") && !tone_check.starts_with("ERROR") && distinct == 1;
                quality.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                    plat, row.id, tlabel, mode, dec.src_rate, dec.src_channels, dec.src_frames, out_samples, f6(r1.out_duration_s), opt(row.nominal),
                    opt(diff), f6(diff_ms), opt(tol), tv, f6(r1.peak), f6(r1.rms), f6(r1.dc_offset), opt(r1.full_scale_samples),
                    p.clipped_samples, p.nonfinite_samples, f6(row.expected_rms), rms_check, q(&tone_check), repeat, if all_pass { "PASS" } else { "FAIL" }
                ));
                if let Some(pth) = out_path {
                    outs.push((mode.clone(), pth));
                }
            }
            // 디코더 간 비교
            let get = |m: &str| outs.iter().find(|(k, _)| k == m).map(|(_, p)| p.clone());
            let mut pairs: Vec<(&str, &str)> = vec![("symphonia", "ffmpeg"), ("ffmpeg", "ffmpeg-native"), ("symphonia", "ffmpeg-native")];
            pairs.retain(|(x, y)| get(x).is_some() && get(y).is_some());
            for (x, y) in pairs {
                if let Ok(c) = output_validator::compare(&get(x).unwrap(), &get(y).unwrap(), 2048) {
                    compare.push_str(&format!(
                        "{},{},{},{}~{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{},{:.6},{:.6},{}\n",
                        plat, row.id, tlabel, x, y, c.samples_a, c.samples_b, c.diff_samples, c.rms_a, c.rms_b, c.peak_a, c.peak_b,
                        c.corr_lag0, c.best_lag, c.corr_best, c.rel_diff_rms_best, c.window_samples
                    ));
                }
            }
        }
        if row.group == "perf" && !a.keep_outputs {
            let _ = fs::remove_dir_all(outputs.join(&row.id));
        }
        eprintln!("done {} ({} runs)", row.id, runs.len());
    }
    let sfx = &a.suffix;
    fs::write(a.results.join(format!("compatibility{sfx}.csv")), compat).expect("write compatibility csv");
    fs::write(a.results.join(format!("audio_quality{sfx}.csv")), quality).expect("write audio_quality csv");
    fs::write(a.results.join(format!("decoder_compare{sfx}.csv")), compare).expect("write decoder_compare csv");
    fs::write(a.results.join(format!("performance{sfx}.csv")), perf).expect("write performance csv");
    fs::write(a.results.join(format!("runs{sfx}.jsonl")), raw).expect("write runs jsonl");
    println!("matrix finished on {} for set '{}': {} fixtures", plat, a.set, sel.len());
}
