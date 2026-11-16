# Windows x86_64 검증 체크리스트 — 상태: **부분 실행 (W1–W7 실행, W8·W9·W10 NOT_RUN)**

2026-11-16 에 실제 Windows 10 x86_64 한 대에서 호환성 세트(39개 픽스처)를 실행했다.
**성능 세트(1·10·61분), LGPL FFmpeg 번들 검증, 백신/SmartScreen 은 하지 않았다 (NOT_RUN).**
따라서 **U1 은 아직 완료로 선언하지 않는다.** 아래 PASS 는 "실행한 범위 안에서" 라는 뜻이다.

## 실행 환경 (`environment.json` 에서 옮김)

| 항목 | 값 |
|---|---|
| OS | Windows 10 Pro, build 19045 |
| CPU / RAM | Intel Core i5-7400 @ 3.00 GHz / 7.9 GB |
| Rust | rustc 1.100.0 (0ef1bc386 2026-11-09), MSVC 도구 사용 (VS Build Tools C++ 워크로드를 실행 중에 설치) |
| FFmpeg | Gyan.FFmpeg Essentials 9.0.1 (winget 설치). `-L` = **GNU GPL v3**, `--enable-gpl --enable-version3` |
| FFmpeg SHA-256 | ffmpeg.exe `72A489EC…E6AA3`, ffprobe.exe `19202B23…EC52F` (`ffmpeg_sha256.txt`) |

**이 FFmpeg 은 비교 기준용(a)이다. LGPL 번들 검증용(b)이 아니므로 W9 의 근거로 쓰지 않는다.**
`environment.json`·`ffmpeg_sha256.txt`·`path_test.jsonl` 에 들어 있던 Windows 사용자 이름은 `<user>` 로 지웠다 (원본 zip 은 랩 밖에만 있다).

## 확인 항목

| # | 항목 | 기준 | 결과 | 근거 |
|---|---|---|---|---|
| W1 | 릴리스 빌드 | `cargo build --release --locked` 성공, 경고 목록 기록 | **PASS** | 빌드 성공(약 1분 14초), 경고 출력 없음 (사용자 콘솔 출력 기준). 처음에는 `link.exe` 없음으로 실패 → VS Build Tools 설치 후 재실행. 이전에 한 번도 컴파일하지 못한 `cfg(windows)` 의 `resource_usage` 도 컴파일됨 |
| W2 | 픽스처 SHA-256 | `verify-fixtures` = 45 ok, 0 mismatched | **PASS** | 45 ok, 0 mismatched |
| W3 | 호환성 | macOS 와 같은 판정 (4모드 각 **42건 = 39 픽스처**), 다른 판정은 원인 분류 | **PASS** | (픽스처, 트랙, 모드) 로 짝지은 168행에서 판정·사용 디코더·fallback·오류 분류 차이 0건. 이 항목의 이전 문구 "각 45건"은 틀렸다: 45 는 `verify-fixtures` 대상이고 호환성 매트릭스는 39개 |
| W4 | 출력 규격 | 모든 출력이 WAV/16000/mono/S16LE, 헤더·데이터 크기 일치 | **PASS** | 위반 0건 (`audio_quality.csv`) |
| W5 | Windows 간 반복 | 같은 입력 3회 → 출력 SHA 동일 | **PASS** | 불안정 그룹 0건 |
| W6 | 크로스 플랫폼 샘플 일치 | 같은 모드의 macOS·Windows 출력 비교 | **PARTIAL** | 아래 설명 |
| W7 | 공백·한글 경로 | `path_test.jsonl` 3모드 성공 | **PASS (수동 실행)** | 아래 설명 |
| W8 | 성능 | 1·10·61분 ×3회. 플랫폼 내부 분석만 | **NOT_RUN** | `-SkipPerf` 로 건너뜀 |
| W9 | FFmpeg 번들 | LGPL 빌드의 크기, 의존 DLL 목록, `-L`, `-buildconf`, 외부 라이브러리 | **NOT_RUN** | 설치된 GPL 빌드만 있었음 |
| W10 | 바이러스 백신/SmartScreen | 번들 FFmpeg 실행 시 차단·지연 여부 | **NOT_RUN** | |

### W6 — 부분

- 출력이 있는 (픽스처, 트랙, 모드) **151쌍 중 87쌍은 출력 SHA 가 다르다** (symphonia 18/34, ffmpeg 23/39, ffmpeg-native 23/39, hybrid 23/39). 87쌍 모두 **샘플 수가 같고**, 레벨(RMS·peak)·DC 차이는 부동소수점 정밀도 수준이다.
- **FFmpeg 를 쓰지 않는 symphonia 단독 경로에서도 18/34 가 다르다** → FFmpeg 버전 차이(macOS 9.0.2 / Windows 9.0.1)만으로는 설명되지 않는다.
- `decoder_compare.csv` 107행 중 값이 다른 곳은 RMS·상관계수·`rel_diff_rms_best` 의 1e-6 자리 차이뿐이다. 예외 하나: F14B 의 `symphonia~ffmpeg` 최적 lag 가 macOS −1960 / Windows −2016 으로 다르다.
  이 쌍은 두 디코더가 **서로 다른 트랙**을 내보내는 경우이고 `corr_best = 0.000000` 이라 lag 값 자체에 의미가 없다. 플랫폼 차이로 보지 않는다.
- 바이트가 다른 원인은 **추정**이다 (Rust 쪽 부동소수점/SIMD 경로 차이: 디코드 또는 rubato 리샘플). rustc 버전도 다르다(1.99.1 / 1.100.0). **[UNVERIFIED]** — 분리해서 확인하지 않았다.
- **하지 않은 것:** macOS·Windows 출력 WAV 를 샘플 단위로 직접 비교(상관계수·최대 샘플 차이)하지 않았다. Windows 출력 파일은 zip 에 넣지 않았다.
  그래서 PASS 가 아니라 PARTIAL 이다. 확인하려면 Windows 의 `results\windows\outputs\<픽스처>\<모드>_tdef.wav` 몇 개(예: F02, F07, F04)가 필요하다.

### W7 — 수동 실행

- 입력 `…\u1 경로 test\입력 파일 (1).m4a` (공백·한글·괄호), 출력 `출력 <모드>.wav` 로 **symphonia / ffmpeg / hybrid 3모드 모두 `success: true`.** 세 실행의 `input_sha256` 이 같고, symphonia 와 hybrid 의 출력 SHA 는 같다.
- `windows_run.ps1` 의 자동 경로 테스트는 **실패했다** (`Copy-Item : 경로에 잘못된 문자가 있습니다`). 원인은 스크립트가 BOM 없는 UTF-8 이라 Windows PowerShell 5.1 이 CP949 로 읽어 한글 리터럴이 깨진 것이다. 프로그램 결함이 아니다.
  사용자가 같은 블록을 콘솔에 직접 붙여 넣어 실행했고, 그 결과가 `path_test.jsonl` 이다.
- `path_test.jsonl` 안의 경로 문자열(`寃쎈줈`, `異쒕젰` 등)도 깨져 있다. 이것은 PowerShell 이 프로그램의 UTF-8 출력을 CP949 로 읽어 저장한 **캡처 문제**다. 증거는 경로 문자열이 아니라 `success` 와 SHA 일치다.
- 시험하지 않은 것: 260자 초과 경로, 입력 파일명에 한글만 있는 경우 외의 다른 유니코드(이모지·일본어 등).

## 스크립트 결함 (스크립트는 고쳤다 — Windows 에서 다시 실행하기 전까지 [UNVERIFIED])

`scripts/windows_run.ps1` 은 이번 실행에서 다음 문제를 보였다. 사용자 승인 뒤 고쳤지만 macOS 에는 PowerShell 이 없어 구문 검사도 못 했다. 첫 재실행이 실제 검증이다.

1. UTF-8 BOM 이 없어 한글 리터럴이 깨진다 → BOM 추가.
2. 프로그램 stdout 을 CP949 로 캡처한다 → `[Console]::OutputEncoding = [Text.Encoding]::UTF8`.
3. `cargo` 가 실패해도 스크립트가 멈추지 않는다 (네이티브 종료 코드를 검사하지 않음) → `$LASTEXITCODE` 검사.
4. 경로 테스트 도중 실패하면 임시 폴더가 남는다 (`try/finally` 없음). 이번에도 `%TEMP%` 에 깨진 이름의 `u1 …` 폴더가 남았고 사용자가 직접 지워야 했다 → `try/finally` 로 정리.

## 측정 한계

- Windows 의 `peak_rss_children_kb`, `cpu_user_s`, `cpu_sys_s` 는 **측정하지 않았다 (null)**. FFmpeg 자식 프로세스의 메모리·CPU 는 이 데이터로 알 수 없다.
- `peak_rss_self_kb` 는 두 플랫폼에서 측정 방식이 달라(Windows = 최대 working set, macOS = 최대 RSS) 절대값을 비교하지 않는다 (Windows 중앙값 9.8 MB, macOS 3.9 MB).
- 짧은 파일(호환성 세트 전체, 모드별 중앙값): Windows 는 symphonia 17.2 ms / ffmpeg 186.6 ms / hybrid 18.1 ms / ffmpeg-native 181.4 ms, macOS 는 6.5 / 44.0 / 7.6 / 41.9 ms. 디코더 사이 순서는 같다.
  Windows 의 FFmpeg 프로세스 기동이 눈에 띄게 크다(F15 0.5초 파일 185 ms, F03 237 ms; macOS 40·77 ms). 원인은 **확인하지 않았다** (큰 GPL 정적 exe, 백신 실시간 검사, 프로세스 생성 비용, i5-7400 의 성능 중 무엇인지 분리하지 못함).
  **다른 하드웨어이므로 플랫폼 간 절대 비교는 하지 않는다.**

## 남은 일 (W8–W10)

사용자가 W8–W10 과 W6 샘플 비교를 진행하기로 승인했다. 도구는 준비했고 **아직 Windows 에서 실행하지 않았다.**

- **W8:** `scripts\windows_run.ps1 -Ffmpeg … -Ffprobe … -PerfOnly` (비교 기준용 Gyan GPL FFmpeg 그대로). 결과는 `*_perf` 접미사로 쓰여 앞선 실행을 덮어쓰지 않는다. 1·10·61분 ×3회.
- **W9:** LGPL 전용 FFmpeg 를 구한다. (a) 다운로드 후보: BtbN `ffmpeg-n9.0-latest-win64-lgpl-9.0.zip` — 출처 URL·SHA-256·`-L` 을 기록한다. 이 빌드는 정적 빌드라 **우리가 쓸 최소 빌드의 크기를 대표하지 않는다.** (b) 크기 근거가 필요하면 `scripts\build_ffmpeg_lgpl.sh` 를 MSYS2 MinGW64 에서 빌드한다 (시험하지 않았음).
  `scripts\windows_bundle_check.ps1` 이 크기·SHA-256·`-L`·`-buildconf`·외부 라이브러리·의존 DLL 을 기록하고, `windows_run.ps1 … -SkipPerf -SkipPathTest -ResultsDir results\windows_lgpl` 로 호환성 세트를 LGPL 빌드로 다시 돌린다.
- **W10:** 같은 `windows_bundle_check.ps1` 이 서명 상태·Zone.Identifier·첫 실행/이후 실행 시간·Defender 상태와 탐지 기록을 남긴다. SmartScreen 팝업은 GUI 에서만 뜨므로 사람이 탐색기 더블클릭 결과를 따로 적는다.
- **W6:** Windows 의 `results\windows\outputs\<픽스처>` 일부를 보내 주면 `python3 -I scripts/compare_wavs.py results/macos/outputs <압축 푼 폴더>` 로 샘플 단위 비교한다.

## Windows 에서 특히 볼 점 (macOS 결과에서 나온 위험) — 확인 결과

- Symphonia 의 MP4/AAC priming delay 미트리밍(+497 샘플@16 kHz): Windows 에서도 **같은 값**이 나왔다 (F13B `symphonia~ffmpeg` `diff_samples = 497`). OS 와 무관한 현상이다.
- 출력 파일 삭제: 성능 세트를 실행하지 않아 **확인하지 못했다.**
- 줄바꿈(CRLF)과 긴 경로: **확인하지 못했다.**
