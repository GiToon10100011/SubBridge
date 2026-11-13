# Windows x86_64 검증 체크리스트 — 상태: **NOT_RUN**

이번 실험은 macOS 26.6 / Apple Silicon(arm64) 한 대에서만 실행했다. **Windows 결과는 하나도 없다.**
따라서 Windows 항목은 PASS 가 아니라 NOT_RUN 이며, **U1 은 Windows 검증이 끝나기 전까지 완료로 선언하지 않는다.**

## 왜 못 했나

- 실제 Windows 머신이 없다. Wine(`/opt/homebrew/bin/wine`)은 있지만 실제 Windows 가 아니고 arm64 Mac 이라 검증으로 인정하지 않는다.
- Windows 용 Rust 타깃(`x86_64-pc-windows-msvc`)이 설치돼 있지 않다 (`rustup target list --installed` = `aarch64-apple-darwin` 만).
  설치는 시스템 환경 변경이므로 승인 없이 하지 않았다. 따라서 **Windows 전용 코드는 컴파일조차 해 보지 못했다.**

## 컴파일·검증하지 못한 Windows 전용 코드

| 위치 | 내용 | 알려진 한계 |
|---|---|---|
| `src/metrics.rs` `#[cfg(windows)] resource_usage` | `K32GetProcessMemoryInfo` 로 **자기 프로세스**의 최대 working set 만 조회 | 자식 프로세스(FFmpeg) 최대 메모리와 CPU 시간은 **측정하지 않는다** (`None`). FFmpeg 경로의 메모리는 별도 도구(Process Explorer, 성능 카운터)로 재야 한다 |
| `Cargo.toml` | `libc` 는 `cfg(unix)` 전용 | Windows 에서는 `windows-sys` 등이 전이 의존성으로만 들어온다 |

## 준비물

1. Windows 10/11 x86_64, Rust stable(1.85+, edition 2024) + MSVC 빌드 도구
2. macOS 에서 만든 `fixtures\generated\`(45개)와 `fixtures\manifest.csv` 를 이 랩의 같은 위치로 복사 — **재생성하지 말 것**
3. FFmpeg/ffprobe 실행 파일. 아래 둘을 구분해 기록한다:
   - (a) 비교 기준용: 아무 정상 빌드. `ffmpeg -version`, `-L`, `-buildconf` 를 `results\windows\environment.json` 에 남긴다
   - (b) **번들 가능성 검증용: LGPL 전용 빌드.** `scripts\build_ffmpeg_lgpl.sh` 를 MSYS2 MinGW64 셸에서
     `--target-os=mingw32 --arch=x86_64` 를 추가해 실행. 다운로드한 바이너리를 쓴다면 출처 URL, 버전, SHA-256, 라이선스(`-L`)를 반드시 기록한다.
     **설치돼 있는 바이너리를 쓴 것은 번들링 검증으로 인정하지 않는다.**

## 실행

```powershell
scripts\windows_run.ps1 -Ffmpeg C:\ffmpeg\bin\ffmpeg.exe -Ffprobe C:\ffmpeg\bin\ffprobe.exe
python scripts\assemble_results.py      # results\*.csv 갱신 (Windows 행이 NOT_RUN 에서 실제 값으로 바뀜)
```

스크립트가 하는 일: 환경 기록 → `cargo build --release --locked` → `verify-fixtures`(실패 시 중단) → 호환성 매트릭스(4모드×3회)
→ 성능 매트릭스(1·10·61분) → 공백·한글 경로 테스트. 수동 명령은 다음과 같다.

```powershell
cargo build --release --locked
.\target\release\u1_audio_codec_lab.exe verify-fixtures
.\target\release\u1_audio_codec_lab.exe matrix --set compat --ffmpeg <ffmpeg.exe> --ffprobe <ffprobe.exe> --results results\windows
.\target\release\u1_audio_codec_lab.exe run --mode hybrid --input <입력> --output <출력.wav> --ffmpeg <ffmpeg.exe> --ffprobe <ffprobe.exe> --json
```

## 확인 항목 (실행 후 채운다)

| # | 항목 | 기준 | 결과 |
|---|---|---|---|
| W1 | 릴리스 빌드 | `cargo build --release --locked` 성공, 경고 목록 기록 | NOT_RUN |
| W2 | 픽스처 SHA-256 | `verify-fixtures` = 45 ok, 0 mismatched | NOT_RUN |
| W3 | 호환성 | macOS 와 같은 판정 (4모드 각 45건), 다른 판정은 원인 분류 | NOT_RUN |
| W4 | 출력 규격 | 모든 출력이 WAV/16000/mono/S16LE, 헤더·데이터 크기 일치 | NOT_RUN |
| W5 | Windows 간 반복 | 같은 입력 3회 → 출력 SHA 동일 | NOT_RUN |
| W6 | 크로스 플랫폼 샘플 일치 | 같은 모드의 macOS·Windows 출력 SHA 비교. 다르면 샘플 수·상관계수로 허용 범위 판단 (바이트 일치를 요구하지 않음) | NOT_RUN |
| W7 | 공백·한글 경로 | `path_test.jsonl` 3모드 성공 (PowerShell 인코딩 문제와 구분해 기록) | NOT_RUN |
| W8 | 성능 | 1·10·61분 ×3회. 플랫폼 내부 분석만 한다 (macOS 와 절대값 비교 안 함) | NOT_RUN |
| W9 | FFmpeg 번들 | LGPL 빌드의 크기, 의존 DLL 목록(`dumpbin /dependents`), `-L`, `-buildconf`, 외부 라이브러리 | NOT_RUN |
| W10 | 바이러스 백신/SmartScreen | 번들 FFmpeg 실행 시 차단·지연 여부 기록 | NOT_RUN |

## Windows 에서 특히 볼 점 (macOS 결과에서 나온 위험)

- Symphonia: MP4/AAC 에 priming delay 트리밍이 없는 현상은 OS 와 무관하므로 같은 결과(+497 샘플@16 kHz)가 나와야 한다. 다르면 이유를 조사.
- 출력 파일 삭제: 성능 세트는 해시를 계산한 뒤 출력을 지운다. 백신 등이 파일을 잡고 있으면 삭제가 실패할 수 있다.
- 줄바꿈: CSV 를 CRLF 로 체크아웃해도 `lines()` 가 처리하지만, 해시 대상인 오디오 파일은 절대 변환되면 안 된다 (`.gitattributes` 로 binary 지정 권장).
- 긴 경로(260자 초과): 이번 실험은 시험하지 않았다.
