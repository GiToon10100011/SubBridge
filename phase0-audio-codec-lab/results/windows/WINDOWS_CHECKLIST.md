# Windows x86_64 검증 체크리스트 — 상태: **부분 실행 (W1–W5·W7·W8 PASS, W6·W9·W10 부분)**

2026-11-16 에 실제 Windows 10 x86_64 한 대에서 호환성 세트(39개 픽스처)를 실행했다. 같은 날 LGPL 후보 빌드(BtbN 정적 빌드)로 호환성 세트를 다시 돌리고, 번들 관련 사실을 기록하고, 일부 출력 WAV 를 받아 샘플 단위로 비교했다.
**1·10·61분 성능 세트(W8)도 실행했다**(`-PerfOnly`, Gyan GPL 빌드, 웜 캐시, 자식 프로세스 메모리·CPU 는 측정 못 함). W9 는 후보 정적 빌드만 봤고 우리가 쓸 최소 빌드가 아니다. W10 은 SmartScreen 경로를 시험하지 못했다.
따라서 **U1 은 아직 완료로 선언하지 않는다.** 아래 PASS 는 "실행한 범위 안에서" 라는 뜻이다.

## 실행 환경 (`environment.json` 에서 옮김)

| 항목 | 값 |
|---|---|
| OS | Windows 10 Pro, build 19045 |
| CPU / RAM | Intel Core i5-7400 @ 3.00 GHz / 7.9 GB |
| Rust | rustc 1.100.0 (0ef1bc386 2026-11-09), MSVC 도구 사용 (VS Build Tools C++ 워크로드를 실행 중에 설치) |
| FFmpeg (비교 기준, `results/windows/`) | Gyan.FFmpeg Essentials 9.0.1 (winget 설치). `-L` = **GNU GPL v3**, `--enable-gpl --enable-version3` |
| FFmpeg SHA-256 | ffmpeg.exe `72A489EC…E6AA3`, ffprobe.exe `19202B23…EC52F` (`ffmpeg_sha256.txt`) |
| FFmpeg (LGPL 후보, `results/windows_lgpl/`) | BtbN `ffmpeg-n9.0-latest-win64-lgpl-9.0.zip` → `ffmpeg version n9.0.2-57-gd838f9f9ae-20261115`(9.0.2 릴리스가 아니라 그 뒤 57 커밋 시점의 자동 빌드), 정적 |
| LGPL 후보 SHA-256 | ffmpeg.exe `1D5264A5…373CF2`, ffprobe.exe `00FD1F2A…768236`, zip `3D51F62B9431E2BDC20FB01E34BDD0FB92184EBCBE100B9FCC761F81526A53DA` (zip 은 사용자가 `Get-FileHash` 로 계산. BtbN 이 게시한 체크섬과 **대조하지 않았다** [UNVERIFIED]) |

**Gyan 빌드는 비교 기준용(a)이다. LGPL 번들 검증용(b)이 아니므로 W9 의 근거로 쓰지 않는다.**
`environment.json`·`ffmpeg_sha256.txt`·`path_test.jsonl` 에 들어 있던 Windows 사용자 이름은 `<user>` 로 지웠다 (원본 zip 은 랩 밖에만 있다). `results/windows_lgpl/` 도 같다.

## 확인 항목

| # | 항목 | 기준 | 결과 | 근거 |
|---|---|---|---|---|
| W1 | 릴리스 빌드 | `cargo build --release --locked` 성공, 경고 목록 기록 | **PASS** | 빌드 성공(약 1분 14초), 경고 출력 없음 (사용자 콘솔 출력 기준). 처음에는 `link.exe` 없음으로 실패 → VS Build Tools 설치 후 재실행. 이전에 한 번도 컴파일하지 못한 `cfg(windows)` 의 `resource_usage` 도 컴파일됨 |
| W2 | 픽스처 SHA-256 | `verify-fixtures` = 45 ok, 0 mismatched | **PASS** | 45 ok, 0 mismatched (LGPL 재실행에서도 45 ok) |
| W3 | 호환성 | macOS 와 같은 판정 (4모드 각 **42건 = 39 픽스처**), 다른 판정은 원인 분류 | **PASS** | (픽스처, 트랙, 모드) 로 짝지은 168행에서 판정·사용 디코더·fallback·오류 분류 차이 0건. LGPL 후보 빌드로 다시 돌려도 같다(W9). 이 항목의 이전 문구 "각 45건"은 틀렸다: 45 는 `verify-fixtures` 대상이고 호환성 매트릭스는 39개 |
| W4 | 출력 규격 | 모든 출력이 WAV/16000/mono/S16LE, 헤더·데이터 크기 일치 | **PASS** | 위반 0건 (`audio_quality.csv`) |
| W5 | Windows 간 반복 | 같은 입력 3회 → 출력 SHA 동일 | **PASS** | 불안정 그룹 0건 |
| W6 | 크로스 플랫폼 샘플 일치 | 같은 모드의 macOS·Windows 출력 비교 | **PARTIAL (표본 측정)** | 12개 픽스처 48개 파일: 샘플 수 모두 같음, 41개가 ±1 LSB 로 다르고 7개는 동일. 원인 미확인, 허용 기준 미정. 아래 설명 |
| W7 | 공백·한글 경로 | `path_test.jsonl` 3모드 성공 | **PASS (수동 실행)** | 아래 설명 |
| W8 | 성능 | 1·10·61분 ×3회. 플랫폼 내부 분석만 | **PASS (범위 한정)** | 6개 픽스처 × 4모드 × 3회 = 72행 전부 성공. Gyan GPL 빌드 한정. 아래 설명 |
| W9 | FFmpeg 번들 | LGPL 빌드의 크기, 의존 DLL 목록, `-L`, `-buildconf`, 외부 라이브러리 | **PARTIAL** | 후보 **정적** LGPL 빌드의 사실과 호환성 재실행(PASS)은 확인. 크기는 최소 빌드를 대표하지 않고, DLL 목록은 근사치, 최소 빌드는 만들지 않았다. 아래 설명 |
| W10 | 바이러스 백신/SmartScreen | 번들 FFmpeg 실행 시 차단·지연 여부 | **PARTIAL (증거 약함)** | 문제를 못 봤을 뿐 SmartScreen 경로는 시험하지 못했다. 아래 설명 |

### W8 — 성능 세트 (Gyan GPL 빌드 한정)

실행: `scripts\windows_run.ps1 -Ffmpeg "$g\ffmpeg.exe" -Ffprobe "$g\ffprobe.exe" -PerfOnly` (2026-11-16, 같은 Windows 10 머신). cargo 빌드 0.78 s, `verify-fixtures` 45 ok / 0 mismatched. 결과는 `*_perf` 접미사로 `results/windows/` 에 있다(`performance.csv`, `runs_perf.jsonl`, `compatibility_perf.csv`, `audio_quality_perf.csv`, `decoder_compare_perf.csv`, `environment_perf.json`, `ffmpeg_sha256_perf.txt`). 사용자 이름은 `<user>` 로 지웠다. 합본은 `python3 scripts/assemble_results.py` 로 다시 만들었다(Windows 성능 세트의 `PERF_SET` 표식 행이 실제 행으로 바뀜).

- **성공:** 72행(F16 61분 AAC, P01A·P10A, P01M·P10M·P60M × symphonia / ffmpeg / ffmpeg-native / hybrid × 3회) 전부 `success=true`, fallback 0. 성능 세트 호환성 판정 24/24 PASS, 출력 규격 위반 0, 3회 반복 출력 동일 24/24. 출력 바이트 수는 24개 (파일, 모드) 모두 macOS 와 같다.
- **시간(중앙값, 같은 머신 안에서만 해석):** 61분 AAC symphonia 5.47 s / ffmpeg 4.94 s / ffmpeg-native 2.82 s / hybrid 5.43 s, 61분 MP3 7.52 / 4.76 / 3.58 / 7.51 s. 1분 파일은 Symphonia 0.09–0.13 s, FFmpeg 모드 0.21–0.24 s. 전체 표는 보고서 §11. 가장 느린 1회는 8.0 s. 첫 실행과 반복 중앙값의 비는 0.90–1.06 이라 웜 캐시 측정이다.
- **메모리:** 자기 프로세스 working set 만 있다. 9,720–11,308 KB 이고 길이에 따라 거의 늘지 않는다(1분 최대 10,236 / 10분 10,456 / 61분 11,308 KB). FFmpeg 자식의 메모리와 CPU 시간은 null.
- **크로스 플랫폼 SHA:** 성능 세트 24쌍의 출력 SHA 는 **모두 macOS 와 다르다.** 성능 세트는 출력 파일을 지워서 W6 식 샘플 비교를 못 했다. 호환성 세트의 차이(±1 LSB)와 같은 종류로 보이지만 **[UNVERIFIED]**.
- **하지 않은 것:** LGPL 후보·최소 빌드로 긴 파일 재기, 콜드 캐시, 자식 프로세스 메모리·CPU, 성능 출력의 샘플 비교(matrix 의 `--keep-outputs` 필요, `windows_run.ps1` 에는 노출 안 됨), 성능 세트 출력 삭제 성공 여부(받은 zip 의 `outputs` 가 비어 있었지만 사용자가 뺐는지 알 수 없다).

### W6 — 표본 측정, 원인 미확인

- **1차(SHA 비교):** 출력이 있는 (픽스처, 트랙, 모드) **151쌍 중 87쌍은 출력 SHA 가 다르다** (symphonia 18/34, ffmpeg 23/39, ffmpeg-native 23/39, hybrid 23/39). 87쌍 모두 샘플 수가 같고, 레벨(RMS·peak)·DC 차이는 부동소수점 정밀도 수준이다.
- **2차(샘플 단위, 이번에 추가):** 사용자가 Windows 의 `results\windows\outputs` 에서 12개 픽스처(CH06, F01–F05, F07, F09–F11, F15, X05)의 4모드 `_tdef.wav` **48개**를 보냈고, `scripts/compare_wavs.py` 로 macOS 출력과 샘플 단위로 비교했다 (`w6_sample_compare.csv`).
  - 48개 모두 **샘플 수가 같다.** 41개가 DIFFERENT, 7개가 IDENTICAL (F01 의 4모드 전부, F02·F03·X05 의 ffmpeg-native).
  - DIFFERENT 41개는 전부 **최대 |차이| = 1 LSB** (S16). 다른 샘플은 전체의 0.06–0.41 %, RMS 차이 0.024–0.064 LSB, 신호 대 차이 101–111 dB.
  - 모드별로 다른 파일: ffmpeg 11/12, hybrid 11/12, symphonia 11/12, ffmpeg-native 8/12.
- **해석:** ±1 LSB 는 들리는 차이로 볼 근거가 없다. 그러나 **비트 동일은 아니다.** FFmpeg 를 쓰지 않는 symphonia 단독 경로도 다르고 ffmpeg-native 도 일부 다르다 → FFmpeg 버전 차이(macOS 9.0.2 / Windows 9.0.1)만으로는 설명되지 않는다.
- 바이트가 다른 원인은 **추정**이다 (Rust 쪽 부동소수점/SIMD 경로 차이: 디코드 또는 rubato 리샘플). rustc 버전도 다르다(1.99.1 / 1.100.0), CPU 구조도 다르다(arm64 / x86_64). **[UNVERIFIED]** — 분리해서 확인하지 않았다.
- `decoder_compare.csv` 107행 중 값이 다른 곳은 RMS·상관계수·`rel_diff_rms_best` 의 1e-6 자리 차이뿐이다. 예외 하나: F14B 의 `symphonia~ffmpeg` 최적 lag 가 macOS −1960 / Windows −2016 으로 다르다.
  이 쌍은 두 디코더가 **서로 다른 트랙**을 내보내는 경우이고 `corr_best = 0.000000` 이라 lag 값 자체에 의미가 없다. 플랫폼 차이로 보지 않는다.
- **PASS 로 적지 않는 이유:** (a) 48개(151쌍의 약 32 %)만 직접 비교했고 나머지는 SHA·레벨 통계뿐이다. (b) "±1 LSB 까지는 같다고 본다"는 허용 기준이 정해진 적이 없다 — 사용자가 정할 사항이다. (c) 원인을 모른다.

### W7 — 수동 실행

- 입력 `…\u1 경로 test\입력 파일 (1).m4a` (공백·한글·괄호), 출력 `출력 <모드>.wav` 로 **symphonia / ffmpeg / hybrid 3모드 모두 `success: true`.** 세 실행의 `input_sha256` 이 같고, symphonia 와 hybrid 의 출력 SHA 는 같다.
- `windows_run.ps1` 의 자동 경로 테스트는 **실패했다** (`Copy-Item : 경로에 잘못된 문자가 있습니다`). 원인은 스크립트가 BOM 없는 UTF-8 이라 Windows PowerShell 5.1 이 CP949 로 읽어 한글 리터럴이 깨진 것이다. 프로그램 결함이 아니다.
  사용자가 같은 블록을 콘솔에 직접 붙여 넣어 실행했고, 그 결과가 `path_test.jsonl` 이다.
- `path_test.jsonl` 안의 경로 문자열(`寃쎈줈`, `異쒕젰` 등)도 깨져 있다. 이것은 PowerShell 이 프로그램의 UTF-8 출력을 CP949 로 읽어 저장한 **캡처 문제**다. 증거는 경로 문자열이 아니라 `success` 와 SHA 일치다.
- 시험하지 않은 것: 260자 초과 경로, 입력 파일명에 한글만 있는 경우 외의 다른 유니코드(이모지·일본어 등).

### W9 — 후보 정적 LGPL 빌드 (PARTIAL)

원자료: `results/windows_lgpl/` (`bundle_check/bundle_check.json`, `ffmpeg_license.txt`, `ffmpeg_buildconf.txt`, `dependents_*.txt`, 그리고 호환성 CSV 들).

| 항목 | 사실 |
|---|---|
| 출처 | `https://github.com/FFmpeg-Builds/FFmpeg-Builds/releases/download/latest/ffmpeg-n9.0-latest-win64-lgpl-9.0.zip` (브라우저로 받아 탐색기에서 압축 해제) |
| 크기 | ffmpeg.exe **132,056,064 B**, ffprobe.exe 131,855,872 B. **정적 빌드(외부 라이브러리 42개 포함)라서 우리가 쓸 최소 빌드(macOS 직접 빌드 2.77 MB)의 크기를 대표하지 않는다. 크기 근거로 쓰지 않는다.** |
| `-L` 본문 | "GNU Lesser General Public License version 3 or later" (`ffmpeg_license.txt`) |
| configure | `--enable-version3` 있음, `--enable-gpl`·`--enable-nonfree`·`--enable-shared` 없음. `--enable-lib*` 외부 라이브러리 42개 (libopus, libmp3lame, libvorbis, libopencore-amr*, libsvtav1 등). 외부 라이브러리 각각의 라이선스는 **조사하지 않았다** |
| Authenticode | 두 파일 모두 `NotSigned` |
| 의존 DLL | `dumpbin` 이 PATH 에 없어 파일 안의 `*.dll` 문자열을 찾았다(70개, `s.dll` 같은 잡음 포함) → **근사치라 믿지 않는다.** 실제 목록은 VS 개발자 명령 프롬프트의 `dumpbin /dependents` 로 다시 봐야 한다 |
| 스크립트 결함 | `bundle_check.json` 의 `license_lines` 와 콘솔 `-L =` 줄에 `-L` 본문 대신 **버전 배너**가 들어갔다(`Invoke-Native` 가 stderr 를 합침). 본문은 `ffmpeg_license.txt` 에 제대로 있다. 스크립트는 `-hide_banner` 를 붙여 고쳤으나 Windows 에서 다시 실행하기 전까지 [UNVERIFIED] |

**호환성 재실행** (`windows_run.ps1 … -SkipPerf -SkipPathTest -ResultsDir results\windows_lgpl`, 위 LGPL 후보 ffmpeg/ffprobe 사용). (픽스처, 트랙, 모드) 로 짝지어 비교했다:

| 비교 | 결과 |
|---|---|
| `compatibility.csv` 168행, 판정·사용 디코더·fallback·오류 분류·기대 결과·성공 횟수 | macOS 와 차이 **0건**, Gyan(GPL) 과도 **0건**. 판정 PASS 164 / FAIL 4 (macOS 와 같다) |
| `audio_quality.csv` 151행 | **Gyan 과 모든 열이 같다**(출력 SHA 포함 → 151개 출력이 바이트 동일). macOS 대비 SHA 불일치 87건은 Gyan 과 같은 수 |
| `decoder_compare.csv` 107행 | Gyan 과 차이 0건 |

→ 이 후보 LGPL 빌드는 이 호환성 세트에서 GPL 빌드와 **같은 출력**을 냈다. 단 최소 빌드가 아니므로 "우리 최소 빌드도 통과한다"는 근거는 아니다.

짧은 파일 시간(같은 PC, 모드별 중앙값): ffmpeg 218.9 ms (Gyan 190.7), ffmpeg-native 211.7 (184.1), symphonia 17.8 (18.0), hybrid 18.1 (18.3). LGPL 정적 exe 가 약 15 % 느렸다. 한 번의 측정이고 원인(132 MB 실행 파일 로드, 백신 검사, 측정 시점 차이)은 분리하지 못했다.

**하지 않은 것:** Windows(MSYS2 MinGW64)에서 `scripts/build_ffmpeg_lgpl.sh` 로 최소 빌드를 만들기 → NOT_RUN. 이 스크립트의 Windows 경로는 한 번도 실행하지 않았다. zip 체크섬을 게시값과 대조하기. 외부 라이브러리 라이선스 검토.

### W10 — 첫 실행·백신 (PARTIAL, 증거 약함)

| 항목 | 사실 |
|---|---|
| 서명 | `NotSigned` (두 파일) |
| Zone.Identifier | **ffmpeg.exe = 표시 없음, ffprobe.exe = `ZoneId=3`(인터넷)**. 같은 zip 에서 풀었는데 두 파일이 다르다. **원인은 확인하지 못했다.** |
| 첫 실행 시간 | `-version`: ffmpeg 269.3 ms, ffprobe 221.2 ms. 첫 변환 223.6 ms, 이후 5회 109.5–123 ms (중앙값 113.9). 변환 6회 모두 종료 코드 0, stderr 없음. 첫 실행이 느린 원인(백신의 첫 검사 vs 파일 로드)은 분리하지 못했다 |
| 백신 환경 | 등록된 제품: 알약, AhnLab V3 Lite, Windows Defender. Defender 실시간 보호 `False`, 시그니처 버전 `0.0.0.0` → Defender 는 사실상 꺼져 있을 가능성이 높다(다른 백신이 대신하는 것으로 보임). 이 PC 에서 Defender 탐지 기록 0건은 의미가 작다. 알약·V3 의 탐지/격리 기록은 **조회하지 않았다** |
| 탐색기 더블클릭 | 사용자가 ffmpeg.exe 를 실행했을 때 팝업이 뜨지 않았다. 그러나 이 파일은 인터넷 표시(MOTW)가 **없어서**, 인터넷에서 받은 표시가 붙은 exe 의 SmartScreen 경로를 시험한 것이 아니다 |

- 결론은 "문제를 못 봤다"이지 "문제 없음"이 아니다. 다음은 **시험하지 못했다**: MOTW 가 붙은 실행 파일의 첫 실행(SmartScreen 팝업), Defender 가 켜진 PC, Windows 11, 서명 없는 exe 를 앱과 함께 설치할 때의 설치 경로 권한.
- `Unblock-File` 은 쓰지 않았다(MOTW 를 지우면 W10 시험이 달라진다).

## 스크립트 결함 (고쳤다 — 일부만 Windows 에서 확인했다)

`scripts/windows_run.ps1` 은 첫 실행에서 다음 문제를 보였다. 사용자 승인 뒤 고쳤다.

1. UTF-8 BOM 이 없어 한글 리터럴이 깨진다 → BOM 추가.
2. 프로그램 stdout 을 CP949 로 캡처한다 → `[Console]::OutputEncoding = [Text.Encoding]::UTF8`.
3. `cargo` 가 실패해도 스크립트가 멈추지 않는다 (네이티브 종료 코드를 검사하지 않음) → `$LASTEXITCODE` 검사.
4. 경로 테스트 도중 실패하면 임시 폴더가 남는다 (`try/finally` 없음). 이번에도 `%TEMP%` 에 깨진 이름의 `u1 …` 폴더가 남았고 사용자가 직접 지워야 했다 → `try/finally` 로 정리.
5. (새로 발견) `windows_bundle_check.ps1` 의 `-L` 캡처에 배너가 섞인다 → `-hide_banner` 추가.

**실제로 확인한 것:** 고친 `windows_run.ps1` 을 `-SkipPerf -SkipPathTest -ResultsDir results\windows_lgpl` 로 Windows 에서 실행했고 끝까지 정상 동작했다(cargo 빌드·`verify-fixtures` 45 ok·39개 픽스처 매트릭스, 종료 코드 0). `windows_bundle_check.ps1` 도 실행됐다(위 `-L` 결함은 이때 발견).
**이어서 확인한 것:** `-PerfOnly` 도 Windows 에서 끝까지 정상 종료했다(위 W8).
**아직 확인하지 못한 것 [UNVERIFIED]:** 자동 경로 테스트(한글 리터럴·`try/finally`), cargo 실패 시 중단, 수정한 `-hide_banner`. 이 중 일부는 macOS 에 PowerShell 이 없어 구문 검사도 못 했다.

## 측정 한계

- Windows 의 `peak_rss_children_kb`, `cpu_user_s`, `cpu_sys_s` 는 **측정하지 않았다 (null)**. FFmpeg 자식 프로세스의 메모리·CPU 는 이 데이터로 알 수 없다.
- `peak_rss_self_kb` 는 두 플랫폼에서 측정 방식이 달라(Windows = 최대 working set, macOS = 최대 RSS) 절대값을 비교하지 않는다 (Windows 중앙값 9.8 MB, macOS 3.9 MB).
- 짧은 파일(호환성 세트 전체, 모드별 중앙값): Windows(Gyan) 는 symphonia 18.0 ms / ffmpeg 190.7 ms / hybrid 18.3 ms / ffmpeg-native 184.1 ms, macOS 는 7.2 / 44.1 / 7.6 / 42.5 ms (성공한 모든 행의 중앙값). 디코더 사이 순서는 같다. (이 파일과 보고서의 앞선 판에 적었던 17.2 / 186.6 / 18.1 / 181.4 ms 와 6.5 / 44.0 / 7.6 / 41.9 ms 는 집계 방식을 알 수 없고 다시 계산해도 나오지 않아 위 값으로 바꿨다. Gyan 대 LGPL 비교는 같은 집계로 했고, 집계를 바꿔 봐도(전체 행 / 반복 행만 / 키별 중앙값) LGPL 이 ffmpeg 모드에서 14–17 % 느렸다.)
  Windows 의 FFmpeg 프로세스 기동이 눈에 띄게 크다(F15 0.5초 파일 185 ms, F03 237 ms; macOS 40·77 ms). 원인은 **확인하지 않았다** (큰 GPL 정적 exe, 백신 실시간 검사, 프로세스 생성 비용, i5-7400 의 성능 중 무엇인지 분리하지 못함).
  **다른 하드웨어이므로 플랫폼 간 절대 비교는 하지 않는다.**

## 남은 일

| 항목 | 할 일 | 누가 |
|---|---|---|
| W8 (선택, 증거 보강) | **필수 범위는 끝났다.** 더 보려면: LGPL 후보·최소 빌드로 같은 성능 세트(`-Ffmpeg`/`-Ffprobe` 를 그 exe 로, `-ResultsDir` 를 다르게), FFmpeg 자식 프로세스 메모리·CPU 측정 코드 보강, `--keep-outputs` 로 성능 출력의 샘플 비교 | 사용자 + Claude |
| W10 (선택, 증거 보강) | 인터넷 표시(`ZoneId=3`)가 붙은 `ffprobe.exe` 를 탐색기에서 더블클릭해 SmartScreen/백신 팝업 여부를 적는다. 그리고 ffmpeg.exe 에 표시가 없는 이유를 확인한다(`Get-Content <exe> -Stream Zone.Identifier`). `Unblock-File` 은 쓰지 않는다 | 사용자 |
| W9 (선택, 증거 보강) | VS 개발자 명령 프롬프트의 `dumpbin /dependents`, zip 체크섬을 게시값과 대조. 크기 근거가 필요하면 MSYS2 에서 `scripts/build_ffmpeg_lgpl.sh` 로 최소 빌드(시험된 적 없음) | 사용자 + Claude |
| W6 (선택) | 나머지 103개 출력도 샘플 비교, ±1 LSB 허용 기준 결정, 원인 분리(같은 rustc·같은 입력으로 symphonia 경로만 비교 등) | 사용자 결정 + Claude |

## Windows 에서 특히 볼 점 (macOS 결과에서 나온 위험) — 확인 결과

- Symphonia 의 MP4/AAC priming delay 미트리밍(+497 샘플@16 kHz): Windows 에서도 **같은 값**이 나왔다 (F13B `symphonia~ffmpeg` `diff_samples = 497`). OS 와 무관한 현상이다.
- 출력 파일 삭제: 성능 세트는 실행했지만 삭제 성공 여부는 **확인하지 못했다**(코드가 `remove_dir_all` 오류를 무시하고, 받은 zip 의 `outputs` 가 비어 있었으나 사용자가 일부러 뺐는지 알 수 없다).
- 줄바꿈(CRLF)과 긴 경로: **확인하지 못했다.**
