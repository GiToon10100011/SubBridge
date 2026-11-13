# U1 — Audio Codec Technical Validation Lab

SubBridge 의 오디오 디코드·변환 후보 세 가지를 **같은 조건**에서 비교하는 독립 실험 프로젝트.

| 후보 | 구현 | 설명 |
|---|---|---|
| A. Symphonia 단독 | `src/symphonia_runner.rs` | 순수 Rust 디먹서/디코더 (0.6.1) |
| B. FFmpeg 단독 | `src/ffmpeg_runner.rs` | 외부 `ffmpeg` 프로세스 호출 |
| C. Hybrid | `src/hybrid_runner.rs` | Symphonia 먼저, **UnsupportedCodec/UnsupportedContainer 일 때만** FFmpeg fallback |

**어느 후보도 사전에 채택하지 않았다.** 이 프로젝트는 근거를 모으고 추천만 한다. 최종 채택과 SubBridge 본 문서 수정은 별도 승인 대상이다.

**상태: macOS(arm64) 검증 완료, Windows(x86_64) NOT_RUN → U1 전체 검증은 완료되지 않았다.**
결과 요약은 [`results/U1_TECHNICAL_VALIDATION.md`](results/U1_TECHNICAL_VALIDATION.md), Windows 절차는 [`results/windows/WINDOWS_CHECKLIST.md`](results/windows/WINDOWS_CHECKLIST.md).

## 디렉터리 구조

```
Cargo.toml / Cargo.lock        의존성 버전 고정 (직접 의존성은 `=` 고정)
src/
  main.rs                      CLI (run / probe / validate / compare / manifest / verify-fixtures / matrix)
  input_probe.rs               SHA-256, ffprobe JSON 조사
  symphonia_runner.rs          경로 A
  ffmpeg_runner.rs             경로 B ("common": FFmpeg는 디코드만 / "native": FFmpeg가 전부 처리하는 기준선)
  hybrid_runner.rs             경로 C
  pcm_pipeline.rs              공통 후처리: 다운믹스 → 리샘플 → S16LE → WAV 쓰기 (A, B, C 모두 같은 코드를 지난다)
  output_validator.rs          WAV 검증, 톤 분석, 두 WAV 비교
  metrics.rs                   오류 분류, 기록(JSON) 구조, 자원 측정
  matrix.rs                    ★ 요구 구조에 없던 파일 — 아래 "구조 변경" 참조
fixtures/
  catalog.csv                  ★ 픽스처 정의 (기대 결과, 공칭 길이, 기대 RMS/톤)
  generate.sh                  ★ 합성 픽스처 생성기
  manifest.csv                 45개 파일의 SHA-256·바이트·ffprobe 요약
  generated/                   실제 파일 (git 무시) + GENERATOR.txt
  README.md
results/
  macos/                       compatibility/audio_quality/performance/decoder_compare CSV, runs*.jsonl
  macos/lgpl_ffmpeg/           ★ 같은 매트릭스를 LGPL 최소 FFmpeg 로 다시 돌린 결과
  windows/                     WINDOWS_CHECKLIST.md (NOT_RUN)
  compatibility.csv  performance.csv  audio_quality.csv   macos + windows(NOT_RUN 표식) 합본
  decoder_compare.csv          ★ 디코더 간 비교
  U1_TECHNICAL_VALIDATION.md   최종 보고서
licenses/THIRD_PARTY_NOTICES.md
scripts/
  build_ffmpeg_lgpl.sh         ★ LGPL 최소 FFmpeg 소스 빌드 (번들 가능성 검증)
  windows_run.ps1              ★ Windows 재현 스크립트 (NOT_RUN)
  assemble_results.py          ★ 플랫폼별 CSV → results/ 합본
size-probe/                    ★ Symphonia 유무에 따른 바이너리 크기 차이 측정용 보조 크레이트
.gitignore                     ★ target/, .cargo-home/, .ffmpeg-lgpl/, fixtures/generated/, 출력 WAV
```

★ = 요구한 구조에 없던 항목. 추가한 이유:

- **`src/matrix.rs`**: 매 실행을 **별도 프로세스**(`u1lab run`)로 돌려야 실행마다 최대 메모리·CPU 가 독립적으로 측정된다. 이 조율 코드를 디코더 코드와 분리했다.
- **`catalog.csv` / `generate.sh`**: 픽스처를 손으로 만들지 않고 정의→생성→SHA 기록의 순서를 코드로 남기기 위해. 손상 파일도 정상 파일에서 파생한다.
- **`decoder_compare.csv`**: 요구된 "디코더 간 비교"(샘플 수, RMS, Peak, 상관계수)의 결과 파일.
- **`macos/lgpl_ffmpeg/`, `build_ffmpeg_lgpl.sh`**: 설치된 FFmpeg 가 GPL 빌드라 번들링 검증으로 인정할 수 없어서 LGPL 전용 최소 빌드를 직접 만들어 같은 테스트를 반복했다.
- **`size-probe/`**: 실험 바이너리에는 CLI/JSON 코드가 섞여 있어 Symphonia 만의 크기를 알 수 없다. 같은 릴리스 프로파일에서 Symphonia 유무만 다른 두 바이너리를 비교한다.
- **`.cargo-home/`**: `CARGO_HOME` 을 랩 안으로 돌려 의존성 소스가 `~/.cargo` 에 쌓이지 않게 했다 (기존 환경 변경 금지).

## 빌드와 실행

```sh
export CARGO_HOME="$PWD/.cargo-home"        # 의존성을 랩 폴더 안에 둔다
cargo build --release --locked
B=./target/release/u1_audio_codec_lab

$B verify-fixtures                          # 45 ok, 0 mismatched
$B run --mode hybrid --input fixtures/generated/F09_mp4_h264_aac.mp4 --output /tmp/out.wav --json
$B probe --input fixtures/generated/F14_mkv_multitrack_aac_flac_ac3.mkv
$B matrix --set compat --results results/macos           # 4모드 x 3회
$B matrix --set perf --suffix _perf --results results/macos
```

### CLI (세 방식 모두 같은 인터페이스)

`run --mode symphonia|ffmpeg|hybrid|ffmpeg-native --input <파일> --output <wav> [--track <오디오 트랙 순번>] [--ffmpeg <경로>] [--ffprobe <경로>] [--fixture-id <id>] [--json]`

- `ffmpeg-native` 는 후보가 아니라 **참고 기준선**이다 (FFmpeg 가 다운믹스·리샘플·S16 변환까지 직접 한다).
- `--track` 은 **오디오 트랙만 센 0-기반 순번**이다. 기록에는 컨테이너 트랙 ID(`track_id`)와 ffprobe 스트림 인덱스도 함께 나온다 (UI 연결용 식별자).
- JSON 한 줄에 `input`, `input_sha256`, `mode`, `decoder_used`, `fallback`, `fallback_reason`, `output`, `output_sha256`, `out_sample_rate`,
  `out_channels`, `out_format`, `out_duration_s`, `out_samples`, `elapsed_s`, `peak_rss_self_kb`, `peak_rss_children_kb`, `cpu_user_s`, `cpu_sys_s`,
  `rtf`, `success`, `error_category`, `error_message`, 그리고 디코드/후처리 세부(`decode`, `pipeline`)가 들어 있다. 종료 코드는 성공 0, 실패 1.

## 공통 출력 규격과 후처리 정책

출력: **WAV(RIFF, 44바이트 PCM 헤더), 16000 Hz, mono, signed 16-bit little-endian.** 샘플 값이 디코더/리샘플러마다 달라질 수 있으므로
"완전 동일"을 가정하지 않고 검증기로 판정한다.

| 항목 | 정책 (모두 이 실험이 정한 **기준선**이며 SubBridge 에 대한 권고가 아니다) |
|---|---|
| 다운믹스 | 채널 **동일 가중 평균**(합/채널 수). 5.1 의 LFE·채널 배치는 고려하지 않는다. 근거 없는 가중치를 쓰지 않기 위한 선택이며, 위상 반전 스테레오는 0 으로 상쇄된다(S05) |
| 리샘플링 | rubato 5.1.0 `Fft<f32>` 동기식 FFT 리샘플러 (청크 1024, 입력 고정, BlackmanHarris2 창). 원본이 16 kHz 이면 건너뛴다. 스트리밍 처리, 시작 지연 제거, 마지막 부분 청크는 `partial_len` + 0 채움 flush |
| 샘플 수 | 출력 샘플 수 = ceil(입력 프레임 × 16000 / 입력 레이트). 초과분은 잘라 낸다 |
| 포맷 변환 | f32 → i16: `round(x × 32768)` 반올림(0에서 먼 쪽) 후 포화. 디더 없음 |
| 클리핑 | 포화된 샘플 수를 센다(`clipped_samples`). 이번 매트릭스 전체에서 0 이었다 |
| NaN/Inf | 0 으로 바꾸고 개수를 센다(`nonfinite_samples`). 전체 0 이었다 |
| FFmpeg 경로 | `ffmpeg` 는 **디코드만** 한다: `-map 0:a:N -vn -sn -dn -f f32le`(원본 레이트·채널 유지) → 위와 같은 후처리 계층. 디코더 차이와 다운믹스/리샘플 차이가 섞이지 않게 하기 위함 |

## 오류 분류 (8종)

`UnsupportedCodec`, `UnsupportedContainer`, `ParseFailure`, `CorruptInput`, `IoError`, `OutOfMemory`, `ConversionFailure`, `UnknownFailure`

| Symphonia 단계/오류 | 분류 |
|---|---|
| probe 단계 `Unsupported` | UnsupportedContainer (**알 수 없는 컨테이너와 손상된 컨테이너를 구분하지 못한다**) |
| 디코더 생성/디코드 `Unsupported` | UnsupportedCodec |
| probe/디먹스/디코더 생성 `DecodeError` | ParseFailure |
| 디코드 단계 `DecodeError` | 패킷을 건너뛰고 센다. 하나도 못 디코드하면 CorruptInput |
| 예기치 않은 EOF | 이미 디코드한 프레임이 있으면 경고와 함께 부분 성공, 없으면 CorruptInput |
| `LimitError` | CorruptInput |
| 파일 I/O 오류 | IoError |
| 출력 검증 실패 | ConversionFailure |

FFmpeg 는 stderr 문구 패턴으로 분류한다 (`moov atom not found` → ParseFailure, `Invalid data` → CorruptInput, `Decoder ... not found` → UnsupportedCodec 등).
**OutOfMemory 는 시험하지 못했다**: Rust 의 할당 실패는 오류가 아니라 abort 이고, 스트리밍 설계라 메모리가 길이와 무관하게 평탄했다.
Hybrid 는 UnsupportedCodec/UnsupportedContainer 에서만 fallback 하며, 이유와 Symphonia 의 원래 오류를 둘 다 기록한다.

## 판정 기준과 허용 오차 (이 실험이 정한 가정이며 표준이 아니다)

- **호환성**: 기대 `ok` → 3회 모두 성공+출력 검증 통과일 때 PASS. `error` → 깨끗한 실패여야 PASS. `ok_or_error`(손상 파일) → 3회 모두 성공(복구)이거나 3회 모두 깨끗한 실패여야 PASS, 섞이면 FAIL
- **시간축**: 공칭 길이(합성 파일이라 정확히 안다)와 출력 길이의 차이. 무손실 ±2 샘플(16 kHz), 손실 압축 ±ceil(3 × 코덱 프레임 크기 × 16000 / 원본 레이트) 샘플.
  **이 허용 오차는 넓다**(AAC 1115 샘플 = 70 ms). 허용 오차 안이어도 디코더 간 차이는 별도로 보고서에 적는다
- **내용**: 기대 RMS 상대 오차 2%(무손실)/10%(손실), 톤 진폭은 Hann 창 Goertzel 로 측정, 절대 오차 하한 0.0005/0.002
- **반복성**: 같은 입력·설정 3회의 출력 SHA-256 이 같아야 PASS
- **디코더 간 비교**: 정규화 상호상관(±2048 샘플 lag, 가운데 ≤80,000 샘플 창). 순음 한 개짜리 파일은 주기 때문에 lag 값이 의미 없고, 스윕 파일(F16, P*)에서만 정렬을 해석한다

## 재현

- macOS: 위 "빌드와 실행". 픽스처가 없으면 `fixtures/generate.sh` (FFmpeg 필요). 생성 결과는 같은 머신에서 3회 연속 바이트 동일했다.
- Windows: `results/windows/WINDOWS_CHECKLIST.md` — **픽스처는 macOS 에서 복사하고 `verify-fixtures` 로 SHA-256 을 확인**한다 (재생성 금지).

## 의존성

Rust 1.99.1 / edition 2024. 직접 의존성: symphonia 0.6.1(`all`), rubato 5.1.0, clap 4.6.7, serde 1.0.229, serde_json 1.0.154, sha2 0.11.1, libc 0.2.191(unix).
`Cargo.lock` 79 패키지. 라이선스와 FFmpeg 빌드 정보는 `licenses/THIRD_PARTY_NOTICES.md`.

## 알려진 한계

- Windows 미검증. Windows 전용 코드(`metrics.rs` 의 `cfg(windows)`)는 컴파일해 보지 못했다.
- 픽스처가 전부 **합성 신호이며 FFmpeg 인코더로 만든 것**이다. 다른 인코더, HE-AAC, 복잡한 편집 목록, 실제 영상 파일은 포함되지 않았다.
- 디스크 캐시: 입력을 한 번 읽어 둔 **warm cache** 상태에서만 측정했다. 콜드 캐시는 `purge`(관리자 권한)가 필요해 NOT_RUN.
- 음질(지각)이나 ASR 정확도는 평가하지 않았다. 이 실험은 디코드·변환의 정합성만 본다.
- DRM 보호 파일은 생성하지도 우회하지도 않았다. 별도 정책 확인 항목이다.
