# U1 기술 검증 보고서 — Symphonia / FFmpeg / Hybrid

> **U1 전체 검증은 완료되지 않았다.** macOS(arm64)는 검증했고 **Windows(x86_64)는 NOT_RUN** 이다.
> 어느 후보도 채택하지 않았다. 아래 추천은 macOS 증거에 근거한 **잠정** 의견이며 최종 선택은 사용자 결정이다.
> 라이선스는 사실만 기록했고 법률 결론은 내리지 않았다.

표기: 각 섹션의 **상태**는 `PASS / FAIL / NOT_RUN / NOT_APPLICABLE` 중 하나로, 플랫폼별로 나눠 적었다. 실제로 확인하지 않은 것은 PASS 로 적지 않았다.
원자료: `results/compatibility.csv`, `audio_quality.csv`, `performance.csv`, `decoder_compare.csv` (macOS 행 + Windows `NOT_RUN` 표식 행), `results/macos/`, `results/macos/lgpl_ffmpeg/`.

---

## 1. Executive Summary

**상태: macOS = PASS(실험 수행 완료) · Windows = NOT_RUN → U1 = 미완료**

| # | 발견 | 근거 |
|---|---|---|
| 1 | **Symphonia 0.6.1 은 AC-3, WMA(ASF), Opus 를 디코드하지 못한다.** 호환성 48건 중 4건 FAIL(F14 트랙2·X01=AC-3, X02=WMA, X03=Opus). AAC/MP3/FLAC/ALAC/WAV 계열은 전부 성공 | `compatibility.csv` |
| 2 | **Symphonia 는 MP4/M4A/MOV/MKV 의 AAC 에서 priming delay(1024 프레임)를 제거하지 않는다.** FFmpeg 보다 출력이 **앞쪽 약 23.2 ms 가 더 남고 끝 패딩도 남는다**(10초 파일에서 +497 샘플@16 kHz = 31 ms). 앞 1024 프레임을 직접 빼 보면 FFmpeg 와 상관 1.0, 상대 RMS 차이 1e-5 → **디코더 자체는 정확하고 트리밍만 빠진 것** | §10 진단 |
| 3 | **Symphonia 는 기본(default) 트랙 표시를 놓쳐 첫 트랙을 조용히 고른다.** 기본 트랙이 두 번째인 F13B/F14B 에서 Symphonia·Hybrid 는 **오류 없이 성공하지만 엉뚱한 트랙(440 Hz)** 을 냈다. MP4 에서는 언어 정보도 없다. 오류가 없으므로 Hybrid 의 fallback 이 잡지 못한다 | §9 |
| 4 | **Hybrid fallback 은 미지원 형식 5/5 에서 정상 동작**했지만, Symphonia 가 "미지원"과 "손상"을 구분하지 못해 **잘린/빈/손상 파일에서도 fallback 이 발생**한다(F17B, F18, 빈 파일). F18 은 FFmpeg 가 복구했고 나머지는 FFmpeg 도 실패했다 | §7 |
| 5 | FFmpeg 는 **두 빌드(Homebrew GPL, 직접 빌드한 LGPL 최소) 모두 전부 성공**(GPL 48/48, LGPL 42/42), 출력도 240건 중 234건이 바이트 동일(나머지 6건=Opus, 상대 RMS 5e-6). 긴 파일에서 Symphonia 보다 1.2~1.5배 빠르지만 짧은 파일은 **프로세스 기동 40~77 ms** vs Symphonia 1~8 ms | §6, §11 |
| 6 | **설치된 FFmpeg(Homebrew)는 GPLv3 이고 의존 라이브러리 20개 39.5 MB 라 번들 대상이 아니다.** LGPL 전용 최소 빌드를 소스에서 직접 만들어 같은 테스트를 통과시켰다: `ffmpeg` 2.77 MB, `ffprobe` 2.59 MB, 링크 대상은 macOS 시스템 프레임워크뿐. Symphonia 는 바이너리에 **+1.31 MB** | §13, §15 |
| 7 | 최소 FFmpeg 빌드는 **컴포넌트 이름을 틀려도 configure 가 경고하지 않고** 실행 시점에 전부 실패한다(이번에 실제로 겪음) | §17 |
| 8 | 공통 후처리 계층을 쓴 결과 모든 출력이 WAV/16 kHz/mono/S16LE, 클리핑·NaN 0, DC 오프셋 최대 0.00015, **3회 반복 175건 전부 SHA 동일**. FFmpeg 가 직접 다운믹스하면(5.1) 가중치가 달라 진폭이 틀어진다 | §8, §10 |

**잠정 추천은 §19**, 사용자가 정해야 할 사항은 **§20** 이다.

### 16단계 수행 현황

| 단계 | 상태 | | 단계 | 상태 |
|---|---|---|---|---|
| 1 환경 조사 | PASS | | 9 출력 검증기 | PASS |
| 2 문서·라이선스 조사 | PASS (법률 판단 제외) | | 10 호환성 테스트 | macOS PASS / Windows NOT_RUN |
| 3 실험 프로젝트 | PASS | | 11 음질·시간축 | macOS PASS / Windows NOT_RUN |
| 4 픽스처·SHA | macOS PASS / 교차 플랫폼 SHA NOT_RUN | | 12 성능 | macOS PASS / Windows NOT_RUN |
| 5 Symphonia | PASS | | 13 macOS 정리 | PASS |
| 6 FFmpeg | PASS | | 14 Windows 재현 준비 | PASS (자료만) |
| 7 공통 PCM 계층 | PASS | | 15 Windows 테스트 | **NOT_RUN** |
| 8 Hybrid | PASS | | 16 보고서 | PASS |

---

## 2. Tested Environments

**상태: macOS = PASS · Windows = NOT_RUN**

| 항목 | macOS (실행함) | Windows |
|---|---|---|
| 기기 | Apple M5, 10코어, 32 GiB | **NOT_RUN** (Windows 머신 없음) |
| OS | macOS 26.6.2 (25G83), arm64 | NOT_RUN |
| Rust / 컴파일러 | rustc 1.99.1, cargo 1.99.1, Apple clang 21.0.0 | NOT_RUN (Windows 타깃 미설치) |
| 기준 FFmpeg | Homebrew ffmpeg 9.0.2_1 (**GPLv3+**, shared) | — |
| 번들 검증 FFmpeg | 소스 9.0.2 에서 직접 빌드한 LGPL 전용 최소 빌드 (정적) | — |

Wine 이 설치돼 있으나 실제 Windows 가 아니고 arm64 이므로 검증으로 인정하지 않았다. 측정은 **디스크 캐시가 데워진(warm) 상태**, 다른 무거운 작업이 없는 조건에서 했다(일부 실행 중 문서 작성 같은 가벼운 작업이 있었음).

## 3. Dependency Versions

**상태: PASS** (`Cargo.lock` 79 패키지, 직접 의존성은 `=` 고정)

| 크레이트 | 버전 | 라이선스 |
|---|---|---|
| symphonia (`features=["all"]`) + 하위 15개 | 0.6.1 | MPL-2.0 |
| rubato | 5.1.0 | MIT OR Apache-2.0 |
| clap / serde / serde_json / sha2 / libc | 4.6.7 / 1.0.229 / 1.0.154 / 0.11.1 / 0.2.191 | MIT OR Apache-2.0 |
| FFmpeg (기준) | 9.0.2_1 Homebrew | GPL-3.0-or-later |
| FFmpeg (번들 검증) | 9.0.2 소스, SHA-256 `8c385028…e002e` (Homebrew formula 값과 일치) | LGPL-2.1-or-later |

Symphonia 기본 feature 는 MP4/AAC/ALAC/MP3 를 **포함하지 않는다**(`features=["all"]` 필요). Symphonia README 에는 Opus 등이 언급되지만 0.6.1 실제 코덱 목록에 Opus 는 없었다.

## 4. Fixture Manifest

**상태: macOS = PASS · 교차 플랫폼 SHA 일치 = NOT_RUN**

- 45개 합성 픽스처(F01–F18C, F13B, F14B, X01–X05, S01–S06, SR08–SR96, CH06, P*), 총 약 131 MB. `fixtures/manifest.csv` 에 SHA-256·바이트·컨테이너·코덱·샘플레이트·채널·길이·트랙 수 기록.
- **생성 결정성**: 같은 머신에서 `generate.sh` 를 3회 실행, 기존 43개가 모두 바이트 동일(마지막 실행은 이전 manifest 로 `verify-fixtures` = 43 ok, 0 mismatched). 새 manifest 는 45 ok.
- 실제 작품·DRM 자료 없음. DRM 항목은 별도 정책 확인(§18).
- **한계**: 모든 손실 압축 픽스처가 FFmpeg 인코더로 만들어졌다. 다른 인코더/HE-AAC/복잡한 편집 목록은 표본에 없다.
- Windows 에서 같은 SHA 인지는 확인하지 못했다.

## 5. Symphonia Compatibility

**상태: macOS = FAIL (미지원 4건 + 트랙 선택 2건) · Windows = NOT_RUN**

호환성 48건(픽스처×트랙): **44 PASS / 4 FAIL.** 품질 검사까지 합치면 40건 중 38 PASS / 2 FAIL.

| 구분 | 결과 |
|---|---|
| 성공 | WAV(PCM16/24/float32, 8–96 kHz), MP3 CBR/VBR, FLAC, ALAC, AAC-LC(M4A/MP4/MOV/MKV), MKV-FLAC, 다중 트랙(AAC/FLAC), 5.1, 61분 AAC·MP3 |
| FAIL(미지원) | X01·F14 트랙2 = AC-3 (`UnsupportedCodec`), X02 = WMA/ASF (`UnsupportedContainer`), X03 = Opus (`UnsupportedCodec`) |
| FAIL(품질) | F13B, F14B: 기본 트랙이 두 번째일 때 **첫 트랙을 선택**(성공으로 보고됨) |
| 주의(PASS 이나 결함) | AAC 트리밍 누락(§10), MP4 언어 미제공(§9), 잘린 MP3 에서 경고 없는 부분 성공(§16) |

Symphonia 소스 확인: isomp4 는 `elst`/`mdhd.language` 를 파싱만 하고 `Track` 에 반영하지 않는다. mkv 는 `FlagDefault` 요소가 **있고 1일 때만** DEFAULT 로 표시한다(스펙상 요소가 없으면 기본값 1 인데 ffmpeg 가 만든 MKV 는 기본 트랙에서 요소를 생략).

## 6. FFmpeg Compatibility

**상태: macOS = PASS · Windows = NOT_RUN**

| 빌드 | `ffmpeg`(공통 후처리) | `ffmpeg-native`(참고) | 비고 |
|---|---|---|---|
| Homebrew 9.0.2_1 (GPL) | 48/48 PASS | 48/48 PASS | 기준 |
| **직접 빌드 LGPL 최소** | **42/42 PASS** (compat 세트) | 42/42 PASS | 번들 가능성 검증 |

- 두 빌드의 출력 SHA 는 240건 중 **234건 동일**. 다른 6건은 X03(Opus): 샘플 수 동일, 상대 RMS 약 5e-6. Homebrew 는 `--enable-libopus` 라 디코더 구현이 다를 수 있다(어느 쪽이 쓰였는지는 확인하지 않음).
- 직접 빌드는 **처음에 전부 실패**했다(원인: configure 컴포넌트 이름 오류, §17). 수정 후 위 결과.
- `ffmpeg-native`(FFmpeg 가 다운믹스까지 처리)는 5.1(CH06)에서 가중치가 달라 톤 진폭이 0.062(기대 0.05)로 나왔다 → **FFmpeg 에 다운믹스를 맡기면 정책이 FFmpeg 내부 행렬에 종속**된다. 이번 후처리 계층은 이를 피한다.
- 선택하지 않은 스트림은 디코드하지 않는다: `Stream mapping: Stream #0:2 -> #0:0 (flac -> pcm_s16le)` (영상·나머지 오디오 매핑 없음).

## 7. Hybrid Fallback Results

**상태: macOS = PASS(규칙대로 동작) · 설계 한계 = FAIL(§7.3) · Windows = NOT_RUN**

### 7.1 fallback 이 일어난 경우 (Symphonia 오류 → FFmpeg)

| 입력 | Symphonia 오류 | fallback 결과 | 평가 |
|---|---|---|---|
| X01 (AC-3) | UnsupportedCodec | FFmpeg 성공 | 의도대로 |
| F14 트랙2 (AC-3) | UnsupportedCodec | FFmpeg 성공 | 의도대로 |
| X03 (Opus) | UnsupportedCodec | FFmpeg 성공 | 의도대로 |
| X02 (WMA) | UnsupportedContainer | FFmpeg 성공 | 의도대로 |
| **F18** (ftyp 손상) | UnsupportedContainer | **FFmpeg 가 복구해 성공(160000 샘플)** | 손상 파일인데 fallback 됨 |
| **F17B** (moov 없음) | UnsupportedContainer(`missing moov atom`) | FFmpeg 도 ParseFailure | **불필요한 fallback** |
| **빈 파일** | UnsupportedContainer | FFmpeg CorruptInput | **불필요한 fallback** |

### 7.2 fallback 하지 않은 경우

- 존재하지 않는 파일: `IoError`, fallback 없음 (`no fallback: IoError is not an allowed fallback category`).
- F18B(EBML 헤더 손상) / F18C(moov 손상): Symphonia `ParseFailure` → fallback 없이 실패(FFmpeg 도 실패하는 입력).
- **OutOfMemory·변환 실패의 fallback 금지는 규칙으로만 구현**했고 실제 발생시켜 보지는 못했다(NOT_RUN).

### 7.3 설계 한계

1. 사용자 규칙은 "손상 입력은 fallback 하지 않는다"이지만 **Symphonia 가 probe 단계에서 미지원 컨테이너와 손상 컨테이너를 구분하지 못해** 구현만으로는 지킬 수 없다(F17B, F18, 빈 파일). 손상 여부를 가르려면 FFmpeg/ffprobe 에 물어야 한다.
2. **"오류 없는 오동작"은 fallback 이 잡지 못한다**: F13B/F14B(잘못된 기본 트랙), MP4/AAC 트리밍 누락. Hybrid 의 결과는 **파일 형식에 따라 시간축 기준이 달라진다**(MP3/FLAC=기준 일치, AAC=+23 ms, AC-3=FFmpeg 경유).
3. 짧은 파일에서 fallback 비용 = Symphonia probe 시도 + FFmpeg 기동(X01 41 ms vs FFmpeg 단독 42 ms 로 차이 미미).

## 8. PCM Output Validation

**상태: macOS = PASS · Windows = NOT_RUN**

모든 성공 출력(175건, 4모드)에 같은 검증기를 적용:

| 검증 | 결과 |
|---|---|
| WAV 파싱, 16000 Hz, mono, PCM S16LE | 전부 통과 |
| 헤더 크기 vs 실제 데이터 크기 | 전부 일치 |
| NaN/Inf (중간 float) | 0건 |
| 클리핑(포화) / full-scale 샘플 | 0건 / 0건 |
| DC 오프셋 | 최대 0.00015 |
| Peak | 최대 0.551 (입력 0.5 진폭 + 손실 압축) |
| **3회 반복 SHA-256 / 샘플 수 / RMS / Peak / 길이** | **175건 전부 동일 (`DIFFERENT` 0건)** |

다운믹스·리샘플링·포맷 변환·클리핑·샘플 수 계산 정책은 `README.md` 에 기록했다(동일 가중 평균, rubato FFT, 반올림+포화, ceil(N×16000/R)).

## 9. Multi-track Results

**상태: 탐지·명시 선택 = PASS · 기본 트랙/언어 메타데이터(Symphonia) = FAIL · Windows = NOT_RUN**

| 항목 | Symphonia | FFmpeg (ffprobe) |
|---|---|---|
| 트랙 개수 | F13=2, F14=3 ✓ (AC-3 는 `unknown(AudioCodecId(4104))` 로 보이지만 순번은 셈) | F13=2, F14=3 ✓ |
| 트랙별 codec | aac / aac / flac / (AC-3 미식별) | aac / flac / ac3 ✓ |
| **언어** | **MKV ✓(jpn/eng/und), MP4 ✗ `None`** (mdhd 파싱만 하고 미노출) | ✓ (F13 eng/jpn, F14 jpn/eng, AC-3 트랙은 태그 없음) |
| **default disposition** | **✗ 전부 false** (MKV: 요소 생략 시 놓침, MP4: tkhd 미반영) | ✓ (F13 트랙0, F14 트랙0 = true) |
| 트랙 ID | ✓ (`"2"`,`"3"`,`"4"`) | ✓ MP4 `"0x2"`,`"0x3"`, 스트림 인덱스 1,2,3 |
| 사용자가 선택한 트랙 디코드(`--track`) | ✓ 트랙0=440 Hz, 1=880 Hz (톤 검사 PASS), 트랙2=AC-3 미지원 | ✓ 트랙0/1/2 = 440/880/1320 Hz 전부 PASS |
| 불필요한 디코드 | 다른 트랙 패킷은 읽기만 하고 디코드하지 않음 (F13: 964패킷 중 432 디코드, 532 건너뜀; 영상은 디코더 자체가 없음) | `-map`·`-vn` 으로 선택 스트림만 디코드 (verbose 로그로 확인) |
| **기본 트랙 자동 선택** (F13B/F14B) | **✗ 첫 트랙 선택(440 Hz)을 성공으로 보고** | ✓ 880 Hz |

UI 연결용 식별자: 기록의 `decode.tracks[].ordinal`(오디오 트랙 순번, `--track` 입력값), `track_id`(컨테이너 ID), `stream_index`(ffprobe 인덱스, FFmpeg 경로만).
주의: Symphonia 는 식별 가능한 트랙만 나열하므로 식별 불가 트랙이 앞에 있으면 순번이 ffprobe 와 어긋날 수 있다(F14 의 AC-3 는 뒤쪽이라 영향 없음).
**한계**: 기본 트랙이 첫 번째인 F13/F14 에서는 이 결함이 가려졌고, 두 번째로 바꾼 F13B/F14B 에서 드러났다.

## 10. Audio Quality Comparison

**상태: macOS = PASS(허용 오차 기준) / 설명된 편차 있음 · Windows = NOT_RUN**

### 10.1 시간축 (출력 길이 − 공칭 길이, 16 kHz 샘플)

| 형식 | Symphonia | FFmpeg | 비고 |
|---|---|---|---|
| WAV/FLAC/ALAC/MKV-FLAC (무손실) | 0 | 0 | |
| MP3 CBR/VBR (60분 포함) | 0 | 0 | Symphonia 는 LAME 헤더 gapless 적용 |
| **AAC (M4A/MP4/MOV, 10초)** | **+497 (31 ms)** | 0 | |
| AAC 0.5초(F15) | +545 (34 ms) | 0 | |
| AAC 1분 / 10분 / 61분 | +378 / +430 / +726 | 0 / 0 / 0 | **누적 드리프트 없음**: 정렬 lag 는 길이와 무관하게 −372 샘플 |
| AAC in MKV (F11) | +497 | **+125 (7.8 ms)** | FFmpeg 도 MKV 에서는 완전히 제거하지 못함(원인 미조사) |

허용 오차(AAC 1115 샘플 = 70 ms) **안이므로 실험 판정은 PASS** 이지만, 이는 **설명된 체계적 편차**(priming 1024 프레임=23.2 ms + 끝 패딩)이며 자막 시간축에 상수 오프셋으로 들어간다.

**진단 실험**(`U1_DIAG_TRIM_START_FRAMES=1024`, 기본 꺼짐·후보 동작 아님): Symphonia 출력 앞 1024 프레임을 버리면

| 파일 | 길이 차(샘플) | 정렬 lag | 상관 | 상대 RMS 차이 |
|---|---|---|---|---|
| P01A (1분) 트림 전 → 후 | 378 → 6 | −372 → 0 | 0.9927 → 1.0000 | 0.121 → 1e-5 |
| P10A (10분) 트림 전 → 후 | 430 → 59 | −372 → 0 | 0.9890 → 1.0000 | 0.148 → 1e-5 |

→ Symphonia 의 AAC 디코딩 값은 FFmpeg 와 사실상 같고(−100 dB), 차이는 **트리밍 누락**으로 설명된다. 남은 6~59 샘플은 끝 패딩.

### 10.2 디코더 간 비교 (Symphonia vs FFmpeg, 같은 후처리 계층 통과)

- 34건 중 **21건 바이트 동일**: WAV(PCM16/24/float), FLAC, ALAC, MKV-FLAC, 스테레오 특수 6건, 샘플레이트 6건, 5.1.
- 나머지 13건: 손실 압축 11건 + 기본 트랙 선택이 달라 출력이 다른 F13B/F14B 2건. **MP3 는 상대 RMS 7~8e-6**(F04/F05, 바이트는 다름), AAC 는 위 시간축 때문에 lag 0 상관이 낮게 보임(트림 후 1.0).
- 잘린 MP3(F17): Symphonia 95,312 vs FFmpeg 95,730 샘플(FFmpeg 가 마지막 부분 프레임 26 ms 더 디코드), 겹치는 구간 상관 1.0.
- 순음 파일은 주기 때문에 상호상관의 best lag 값이 무의미하다(그래서 스윕 파일로 정렬을 해석했다).

### 10.3 공통 후처리 계층 vs FFmpeg 자체 변환(`ffmpeg-native`)

- 정수비가 아닌 소스(22.05/44.1/96 kHz)에서 두 리샘플러(rubato FFT / swresample)의 순음 출력은 RMS 가 같고 **lag 0 상관 0.98~0.998** — 약 반 샘플 안팎의 서브샘플 정렬 차이로 **추정**한다(직접 측정하지 않음, 어느 쪽이 정확한지 미조사). 16 kHz 기준 반 샘플 = 31 µs.
- 8/16/48 kHz 는 두 경로가 바이트 또는 상관 1.0 으로 일치.

### 10.4 오디오 특수 테스트

| 테스트 | 결과 (공통 정책 = 동일 가중 평균) |
|---|---|
| 정상 좌우(S01), 음량 차이(S02), 서로 다른 주파수(S06) | 기대 진폭 PASS (0.5 / 0.3 / 0.25+0.25) |
| 한쪽 채널만(S03/S04) | 진폭이 **절반(0.25)** 으로 감소 — PASS 이나 −6 dB |
| **위상 반전(S05)** | **완전 상쇄, RMS 0** — 정책의 결과. 신호가 사라지는 사례이며 정책 결정 필요 |
| 8/16/22.05/44.1/48/96 kHz → 16 kHz | 6/6 톤 진폭 PASS, 길이 오차 0 |
| 5.1 (CH06) | 채널 6개 톤 각 0.05 PASS(LFE 포함 동일 가중 — 임의 가중치 미적용). `ffmpeg-native` 는 0.062 로 불일치 |

음질(지각)과 ASR 정확도는 평가하지 않았다.

## 11. Performance Comparison

**상태: macOS = PASS · Windows = NOT_RUN** (플랫폼별 분석만, 타 플랫폼 절대값 비교 없음)

조건: 같은 입력·같은 출력 규격, 3회, 입력을 한 번 읽어 **warm cache** 로 통일, 모드는 (반복→모드) 순서로 번갈아 실행. `RTF = 처리 시간 / 오디오 길이`. 첫 실행과 반복 중앙값은 거의 같았다(웜 상태라 콜드 캐시 차이는 측정하지 못함 → NOT_RUN).

**중앙값 (Homebrew GPL FFmpeg 기준 / 괄호 = LGPL 최소 빌드)**

| 파일 | Symphonia | FFmpeg(공통) | Hybrid | 배속(Sym / FFmpeg) |
|---|---|---|---|---|
| 1분 AAC (P01A) | 0.035 s | 0.059 s (0.031) | 0.034 s | 1691× / 990× (1905×) |
| 10분 AAC (P10A) | 0.332 s | 0.282 s (0.239) | 0.345 s | 1820× / 2147× (2539×) |
| 61분 AAC (F16) | 2.13 s | 1.59 s (1.39) | 2.03 s | 1750× / 2336× (2601×) |
| 1분 MP3 (P01M) | 0.040 s | 0.064 s (0.035) | 0.040 s | 1496× / 930× (1681×) |
| 10분 MP3 (P10M) | 0.397 s | 0.330 s (0.287) | 0.398 s | 1508× / 1818× (2102×) |
| 61분 MP3 (P60M) | 2.44 s | 1.89 s (1.75) | 2.46 s | 1516× / 1934× (2111×) |

- RTF 는 모두 0.0004~0.0011 (실시간보다 **약 1,000~2,600배 빠름**). 61분이 2초 안팎이라 **속도는 어느 후보에서도 병목이 아니다.**
- 긴 파일은 FFmpeg 가 Symphonia 보다 빠르다(1.2~1.5배). 1분 파일은 Homebrew FFmpeg 가 더 느리고(원인은 확인하지 않음), LGPL 정적 빌드는 Symphonia 와 같거나 더 빠르다.
- **짧은 파일의 프로세스 기동 오버헤드**: F15(0.5초) 중앙값 Symphonia 0.7 ms / FFmpeg 40.2 ms, F07 6.7 ms / 43.3 ms, F03 7.8 ms / 76.7 ms (FFmpeg 모드는 ffprobe 와 ffmpeg 두 프로세스).
- CPU: Symphonia 약 100%(단일 코어). FFmpeg 경로는 자식 프로세스 포함 약 200~380%(멀티스레드) → **CPU 시간은 FFmpeg 가 더 많이 쓴다**(F16: Symphonia ≈2.1 s, FFmpeg ≈5.6 s).
- 출력 크기는 모두 동일(61분 = 117.1 MB). 실패 0, 성능 세트에서 fallback 0.

## 12. Memory Usage

**상태: macOS = PASS · Windows = NOT_RUN (Windows 코드는 자기 프로세스 working set 만 측정하도록 작성, 미컴파일)**

| 파일(길이) | Symphonia 자기 RSS | FFmpeg 모드: 자기 RSS + 자식(ffmpeg) RSS | LGPL 최소 자식 RSS |
|---|---|---|---|
| 1분 | 4.5 MB | 3.6 + 17.1 MB | 10.0 MB |
| 10분 | 5.0 MB | 4.1 + 19.5 MB | 12.4 MB |
| 61분 AAC | 5.2 MB | 4.0 + 34.5 MB | 27.4 MB |
| 61분 MP3 | 4.6 MB | 4.2 + 17.2 MB | 10.0 MB |

- **길이에 따라 메모리가 늘지 않는다**(스트리밍). 61분 출력(117 MB)을 메모리에 쌓지 않는다. 61분 AAC 에서 FFmpeg 가 큰 것은 mov 샘플 테이블 때문으로 보이나 확인하지는 않았다.
- 자식 RSS 는 `RUSAGE_CHILDREN` 의 최댓값이라 FFmpeg 모드의 실제 합계는 두 값을 더한 것이 상한이다.
- OutOfMemory 는 시험하지 못했다(할당 실패는 abort, §7.2).

## 13. macOS Packaging

**상태: 항목별 — arm64·의존성·실행 권한·경로 = PASS / Developer ID 서명·공증·Gatekeeper = NOT_RUN / x86_64(Intel) = NOT_RUN**

| 항목 | 결과 |
|---|---|
| 아키텍처 | LGPL 최소 빌드 `ffmpeg`, `ffprobe` = Mach-O 64-bit arm64 단독 (**x86_64/유니버설은 만들지 않음 → NOT_RUN**) |
| 필요한 라이브러리 | `otool -L`: libSystem, CoreFoundation, CoreVideo, CoreMedia 뿐. 서드파티 dylib **없음** |
| 실행 권한 | `-rwxr-xr-x` |
| 서명 | 링커가 붙인 ad-hoc 서명(`adhoc,linker-signed`), `codesign --verify` valid. **Developer ID 서명·hardened runtime·notarization 은 인증서가 없어 NOT_RUN** |
| Gatekeeper/quarantine | NOT_RUN (터미널 하위 프로세스 실행은 앱 실행 검증이 아님) |
| 앱 번들 안 경로 | `…/bundle test 경로/SubBridge.app/Contents/Resources/bin/{ffmpeg,ffprobe}` 에 복사하고 **공백·한글 입력 경로**로 3모드 실행 → 전부 성공(실제 서명된 앱 번들은 아님) |
| 용량 | `ffmpeg` 2,770,472 B, `ffprobe` 2,593,848 B (정적, strip). Symphonia(`all`) = **+1,308,704 B** (302,608 → 1,611,312 B, LTO/strip/opt3 비교용 최소 바이너리) |
| 참고: 설치된 Homebrew 빌드 | 의존 파일 20개 합계 **39.5 MB**(libavcodec 9.7, libx265 7.6, libcrypto 4.9, libSvtAv1Enc 3.1 …), **GPLv3+** → 번들 대상 아님 |

예상 증가량(arm64, 압축 전): **A** Symphonia ≈ +1.3 MB · **B** FFmpeg ≈ +2.8 MB(ffprobe 유지 시 +5.4 MB) · **C** ≈ +4.1 MB(+6.7 MB). 지원 형식을 늘리면 FFmpeg 쪽이 더 커진다(이번 최소 빌드는 테스트에 쓰인 형식 위주: AAC/ALAC/MP3/FLAC/Vorbis/Opus/WMA/AC-3/E-AC-3/PCM/ADPCM).
`ffprobe` 는 이 실험의 트랙 열거에 쓰였다. 없애려면 `ffmpeg -i` 출력 파싱 등 대안이 필요하며 시험하지 않았다.

## 14. Windows Packaging

**상태: NOT_RUN**

하지 못한 것: x86_64 FFmpeg 빌드/다운로드, 필요한 DLL 목록, 실행 경로, 공백·한글 경로, 설치 경로 권한, 바이너리 용량, 백신/SmartScreen 반응.
준비된 것: `scripts/windows_run.ps1`, `scripts/build_ffmpeg_lgpl.sh`(MSYS2 용 플래그 안내 포함), `results/windows/WINDOWS_CHECKLIST.md`(확인 항목 W1–W10).
macOS 에서 한 공백·한글 경로 시험은 Windows 결과를 **대신하지 않는다.**

## 15. License Review

**상태: 사실 기록 = PASS · 법률 판단 = NOT_APPLICABLE(하지 않음)** — 상세는 `licenses/THIRD_PARTY_NOTICES.md`

| 구성요소 | 사실 | 검토가 필요한 조건 |
|---|---|---|
| Symphonia 0.6.1 (16개 크레이트) | MPL-2.0, 수정하지 않고 사용 | 파일 단위 copyleft: MPL 적용 파일 배포 시 소스 제공·고지 |
| 그 외 Rust 의존성 62개 | MIT / Apache-2.0 / Zlib / Unlicense / 0BSD 계열, 라이선스 미표기·GPL 계열 없음 | 고지 포함 |
| FFmpeg Homebrew 빌드 | `--enable-gpl --enable-version3`, libx264/x265 등, `-L` = GPLv3+, nonfree 없음 | 비공개 소스 앱에 번들 부적합(GPL) |
| **FFmpeg LGPL 최소 빌드** | `--disable-gpl --disable-version3 --disable-nonfree`, 외부 라이브러리 없음, `-L` = LGPLv2.1+ | 라이선스 고지, 소스(수정 포함) 제공, 교체/재링크 가능성, sidecar 실행 vs 링크 구조에 따른 차이 |
| 코덱 특허 | 조사하지 않음 | AAC/AC-3/WMA 등 — Symphonia 든 FFmpeg 든 별개로 남는 문제 |

## 16. Known Limitations

- **Windows 미검증.** Windows 전용 코드는 컴파일해 본 적이 없다.
- 픽스처는 합성 신호이고 FFmpeg 인코더 산출물이다. 실제 사용자 파일(다른 인코더, HE-AAC SBR/PS, 가변 프레임, 복잡한 edit list)은 포함되지 않았다.
- **잘린 파일이 조용히 성공한다**: F17(MP3 60% 절단)은 두 디코더 모두 오류·경고 없이 5.96~5.98초만 출력(10초 중). 앱이 컨테이너 선언 길이와 비교해야 알 수 있다.
- 시간축 허용 오차(AAC 70 ms)는 이 실험이 정한 가정이며 넓다. FFmpeg 의 MKV AAC +125 샘플은 원인을 조사하지 않았다.
- 다운믹스는 동일 가중 평균 기준선일 뿐이다(위상 반전 소실, 한쪽 채널 −6 dB, 5.1 LFE 포함).
- 콜드 캐시, Windows 성능, 멀티 인스턴스 부하는 측정하지 않았다.
- LGPL 빌드는 테스트한 형식에 맞춘 최소 구성이다. 다른 형식은 컴포넌트를 추가해야 한다.

## 17. Failed Tests

| # | 항목 | 결과 | 원인/분류 |
|---|---|---|---|
| 1 | Symphonia: X01, F14 트랙2 (AC-3) | FAIL `UnsupportedCodec` | 디코더 없음 |
| 2 | Symphonia: X02 (WMA) | FAIL `UnsupportedContainer` | 디먹서 없음 |
| 3 | Symphonia: X03 (Opus) | FAIL `UnsupportedCodec` | 디코더 없음 |
| 4 | Symphonia·Hybrid: F13B, F14B | FAIL (품질) | 기본 트랙 미인식 → 엉뚱한 트랙, **오류 없이 성공** |
| 5 | ffmpeg-native: CH06 | FAIL (품질) | FFmpeg 자체 5.1→mono 가중치가 동일 가중과 다름 |
| 6 | Hybrid: F17B/F18/빈 파일 | 설계 한계 | 손상·잘림을 `UnsupportedContainer` 로 분류 → 불필요/부적절한 fallback |
| 7 | LGPL 최소 빌드 1차 시도 | **전체 실패** (`Requested output format 'f32le' is not known`) | configure 에서 muxer 이름을 `pcm_f32le` 대신 `f32le` 로 적음. **configure 가 경고하지 않음**. 수정 후 통과, 1차 결과는 `compatibility_run1_misconfigured_muxer.csv` 로 보존 |
| 8 | (실험 중 바로잡은 내 오류) | 기록 | F18 을 `error` 기대로 둔 것은 잘못(FFmpeg 가 복구) → `ok_or_error` 로 바꾸고 F18C 추가. 트랙 ID 가 None 으로 보였던 것은 조회 스크립트의 키 오타였고 기록 데이터는 정상 |

## 18. Unverified Items

| 항목 | 상태 |
|---|---|
| Windows 전부 (빌드, 픽스처 SHA, 호환성, 성능, 번들, 경로, 백신) | **NOT_RUN** |
| 교차 플랫폼 픽스처 SHA 일치 / 출력 비교 | NOT_RUN |
| 콜드 디스크 캐시 성능 | NOT_RUN (`purge` 관리자 권한 필요) |
| OutOfMemory 처리, 실제 I/O 장애(읽기 도중 실패) | NOT_RUN |
| Developer ID 서명·공증·Gatekeeper, x86_64/유니버설 | NOT_RUN |
| DRM 보호 파일 | NOT_APPLICABLE — 생성·우회하지 않음. 별도 정책 확인 필요 |
| 다른 인코더/HE-AAC/실제 영상 파일 | NOT_RUN |
| FFmpeg 를 `libav*` 라이브러리로 직접 링크하는 구조 | NOT_RUN (프로세스 호출만 검증) |
| ffprobe 없이 트랙 열거 | NOT_RUN |
| 코덱 특허, LGPL 준수 방식의 법적 판단 | NOT_APPLICABLE (하지 않음) |
| 음질(지각)·ASR 정확도 영향 | NOT_RUN |

## 19. Recommendation

**상태: 잠정 의견 (Windows NOT_RUN 이므로 확정 불가). 최종 선택은 사용자 결정.**

**잠정 추천: B — FFmpeg(LGPL 전용 최소 빌드, 별도 프로세스) + 공통 후처리 계층(Rust).** 근거:

1. **지원 범위**: 이 실험에서 FFmpeg 는 전부 성공, Symphonia 는 미지원 4건(AC-3·WMA·Opus). 영상 파일(MKV/MP4)에서 AC-3/Opus 오디오는 흔하다. 다만 실제 사용자 파일의 형식 분포는 이번 실험으로 알 수 없다.
2. **정확도**: FFmpeg 는 MP4 AAC gapless 를 적용하고(0 샘플 편차), 트랙 언어/기본 표시를 제공하며, 손상 헤더도 복구한다(F18). Symphonia 는 위 세 가지에서 결함이 확인됐고, 그중 **기본 트랙 오선택은 오류 없이 지나간다.**
3. **Hybrid 의 이점이 작다**: 속도는 병목이 아니고(61분 2초), 짧은 파일의 프로세스 기동 비용(40~77 ms)도 UI 지연으로는 미미하다. 그런데 Hybrid 도 AC-3/Opus 때문에 **FFmpeg 를 결국 번들해야** 하므로 FFmpeg 의 용량·LGPL 부담은 어차피 진다. 그 위에 Symphonia 를 얹으면 +1.3 MB, MPL 고지, 코드 경로 두 개, **형식별로 다른 시간축 기준**과 "오류 없는 오동작"이 따라온다.
4. **번들 가능성은 확인됨(macOS)**: LGPL 최소 빌드 2.8 MB, 시스템 프레임워크만 의존, 같은 테스트 통과.

**추천을 뒤집을 수 있는 조건**
- 제품 범위가 **AAC/MP3/FLAC/WAV 소스로 한정**되고 LGPL 번들 부담을 피해야 한다면 A(Symphonia 단독)가 성립한다. 단 (a) MP4 AAC 의 1024 프레임 트리밍을 직접 구현, (b) 트랙 기본값·언어를 자체 파싱, (c) 미지원 형식 안내 UI 가 필요하다.
- 법률 검토에서 LGPL FFmpeg 번들이 불가하면 B/C 모두 부적합 → A 또는 사용자 설치 FFmpeg 연동.
- Windows 검증에서 FFmpeg 번들에 문제가 나오면 재평가.

**어느 안을 택하든 유지할 것**: 공통 후처리 계층(다운믹스·리샘플·S16), 출력 검증기, 잘린 파일 길이 검사, 트랙 선택 UI 에 `ordinal+track_id` 사용.

## 20. Decision Required

**상태: 사용자 결정 대기 (이 보고서는 결정하지 않음)**

| # | 결정 | 필요한 이유 |
|---|---|---|
| D1 | 후보 채택: A / B / C | §19 |
| D2 | 입력 형식 범위: AC-3·E-AC-3·Opus·WMA 지원이 필수인가 | A 의 성립 여부를 가름 |
| D3 | LGPL FFmpeg 번들의 **법률 검토** 진행 여부와 배포 구조(sidecar vs 링크) | §15, 법률 판단은 이 실험 밖 |
| D4 | **Windows 검증 방법**: Windows 머신 제공 / `x86_64-pc-windows-msvc` 타깃 설치 승인(컴파일 확인만 가능) / CI | U1 완료 조건 |
| D5 | 다운믹스 정책(동일 평균 / 위상·에너지 기반 채널 선택 / LFE 처리) | 위상 반전 소실, −6 dB, 5.1 |
| D6 | 시간축 허용 기준(특히 AAC 23 ms 상수 오프셋 허용 여부) | A 선택 시 필수 |
| D7 | 트랙 선택 UX 요구(기본 트랙·언어 표시 필요 여부) | Symphonia 결함 영향 |
| D8 | 잘린/손상 파일 정책(경고만 / 거부 / 자동 복구) | §16, §7.3 |
| D9 | 본 프로젝트 문서(`docs/`, ADR) 반영 여부 — **별도 승인 대기, 이번에 수정하지 않음** | 작업 규칙 |
