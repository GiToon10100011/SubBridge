# THIRD_PARTY_NOTICES — U1 실험이 사용한 제3자 구성요소

> 이 문서는 **사실 기록**이다. 법률 자문이 아니며 "배포해도 된다/안 된다"는 결론을 내리지 않는다.
> 라이선스 조건의 해석과 SubBridge 배포 방식에 대한 판단은 별도로 법률 검토를 거쳐야 한다.
> 아래 라이선스 표기는 각 크레이트의 `Cargo.toml`(`cargo metadata`)과 FFmpeg 자체 출력(`ffmpeg -L`, `-buildconf`)에서 읽은 값이다.

## 1. Rust 의존성 (실험 바이너리 `u1_audio_codec_lab`)

`Cargo.lock` 에 79개 패키지(실험 크레이트 1 + 의존성 78). 그중 Windows 전용(`windows-sys`, `windows-link`, `anstyle-wincon`)은
macOS 빌드에는 들어가지 않는다. 직접 의존성 버전은 `Cargo.toml` 에서 `=` 로 고정했다.

### 직접 의존성

| 크레이트 | 버전 | 라이선스 | 용도 |
|---|---|---|---|
| symphonia (`features = ["all"]`) | 0.6.1 | **MPL-2.0** | 순수 Rust 디먹서/디코더 (경로 A, C의 1순위) |
| rubato | 5.1.0 | MIT OR Apache-2.0 | 16 kHz 리샘플링 (공통 후처리) |
| clap | 4.6.7 | MIT OR Apache-2.0 | CLI |
| serde / serde_json | 1.0.229 / 1.0.154 | MIT OR Apache-2.0 | JSON 기록 |
| sha2 | 0.11.1 | MIT OR Apache-2.0 | SHA-256 |
| libc (unix 전용) | 0.2.191 | MIT OR Apache-2.0 | getrusage (최대 메모리/CPU 측정) |

### Symphonia 하위 크레이트 (모두 MPL-2.0, 0.6.1)

`symphonia-core`, `-common`, `-metadata`, `-bundle-flac`, `-bundle-mp3`, `-codec-aac`, `-codec-alac`, `-codec-adpcm`,
`-codec-pcm`, `-codec-vorbis`, `-format-isomp4`, `-format-mkv`, `-format-ogg`, `-format-riff`, `-format-caf`.
저장소: https://github.com/pdeljanov/Symphonia

MPL-2.0 은 **파일 단위** copyleft 로 알려져 있다(라이선스 본문 3절). 라이선스 본문이 요구하는 것을 사실로 옮기면:
MPL 이 적용된 파일(Covered Software)을 배포할 때 그 소스 형태를 받는 사람이 얻을 수 있게 하고, 라이선스 고지를 유지한다.
이 실험에서는 Symphonia 를 **수정하지 않고** crates.io 에서 받은 그대로 사용했다.
SubBridge 의 비공개 소스 부분에 어떤 영향이 있는지는 여기서 판단하지 않는다.

### 그 밖의 전이 의존성 (요약)

| 라이선스 표기 | 패키지 수 | 예 |
|---|---|---|
| MIT OR Apache-2.0 (또는 Apache-2.0 OR MIT) | 대다수 | anstream, bitflags, clap_*, rustfft, num-*, serde_*, sha2, syn … |
| MIT 단독 | 5 | extended, realfft, strsim, windowfunctions, zmij |
| 0BSD OR Apache-2.0 | 1 | audio-codec-algorithms |
| Unlicense OR MIT | 1 | memchr |
| Zlib OR Apache-2.0 OR MIT | 2 | bytemuck, visibility |
| (MIT OR Apache-2.0) AND Unicode-3.0 | 1 | unicode-ident |
| MPL-2.0 | 16 | symphonia 계열 (위) |

표기가 `(none)` 인 패키지는 없었다. GPL/AGPL/LGPL 로 표기된 Rust 의존성은 없었다.
전체 목록은 `cargo metadata --offline --format-version 1` 로 다시 만들 수 있다.

## 2. FFmpeg

FFmpeg 는 **빌드 옵션에 따라 라이선스가 달라진다**(FFmpeg `LICENSE.md`):

- 기본: LGPL v2.1 이상
- `--enable-version3`: LGPL v3 / GPL v3 쪽으로 이동
- `--enable-gpl`: 일부 GPL 컴포넌트와 libx264/libx265 같은 GPL 라이브러리 사용 → 결과물 전체가 GPL
- `--enable-nonfree`: 재배포 불가 빌드

### 2.1 이 머신에 설치된 FFmpeg (측정·실험에 사용, **번들링 검증에 해당하지 않음**)

| 항목 | 값 |
|---|---|
| 출처 | Homebrew `ffmpeg` 9.0.2_1 (`/opt/homebrew/Cellar/ffmpeg/9.0.2_1`) |
| `ffmpeg -L` | GNU General Public License **version 3 or later** |
| configure 핵심 | `--enable-gpl --enable-version3 --enable-shared --enable-libx264 --enable-libx265 --enable-libvpx --enable-libdav1d --enable-libsvtav1 --enable-libopus --enable-libmp3lame --enable-libvmaf --enable-openssl --enable-videotoolbox --enable-audiotoolbox` |
| `--enable-nonfree` | 없음 (configure 목록에서 확인) |
| 링크된 비시스템 dylib | libav*/libsw* 7개 + libx264, libx265, libvpx, libdav1d, libSvtAv1Enc, libopus, libmp3lame, libvmaf, openssl@4 (libssl/libcrypto), liblzma … |

이 빌드는 GPL 이므로 비공개 소스 앱에 그대로 넣을 수 있는 형태가 아니다(라이선스 문구 기준의 사실 서술).
이 실험에서는 **디코드 결과 비교용 기준 FFmpeg** 로만 썼다.

### 2.2 이 실험에서 직접 빌드한 LGPL 전용 최소 FFmpeg (번들 가능성 검증용)

| 항목 | 값 |
|---|---|
| 소스 | https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz |
| SHA-256 | `8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e` (Homebrew formula 의 값과 일치 확인) |
| 빌드 | `scripts/build_ffmpeg_lgpl.sh` (Apple clang 21.0.0, macOS arm64) |
| configure | `--disable-gpl --disable-version3 --disable-nonfree --disable-everything --disable-autodetect --disable-network --disable-avdevice --disable-swscale --enable-static --disable-shared` + 필요한 demuxer/decoder/parser/muxer/encoder/protocol/filter 만 지정 |
| `ffmpeg -L` | GNU **Lesser** General Public License **version 2.1** or later |
| 외부 라이브러리 | 없음 (`--disable-autodetect`). macOS 시스템 프레임워크(CoreFoundation, CoreVideo, CoreMedia, libSystem)만 링크 |
| 크기 | `ffmpeg` 2,770,472 B, `ffprobe` 2,593,848 B (정적, strip 됨) |

이 바이너리로 U1 의 호환성 매트릭스를 다시 돌려 설치된 GPL 빌드와 같은 판정(3개 모드 각 40/40 PASS)을 얻었다 → `results/macos/lgpl_ffmpeg/`.
출력 WAV 의 SHA-256 은 240건 중 234건이 두 빌드에서 **바이트 단위로 같았고**, 달라진 6건은 모두 X03(Ogg Opus)이다.
X03 두 출력의 샘플 수는 같고 상대 RMS 차이는 약 5e-6 이었다. Homebrew 빌드는 `--enable-libopus` 라서 디코더 구현이 다를 수 있다
(어느 쪽이 실제로 쓰였는지는 확인하지 않았다).

주의: 최소 빌드는 **컴포넌트 이름을 틀려도 configure 가 경고하지 않는다.** 이번에도 raw PCM muxer 이름을 `f32le`
(CLI 포맷 이름)로 적어 빌드는 성공했지만 실행 시 모든 FFmpeg 경로가 실패했다(`compatibility_run1_misconfigured_muxer.csv`).
최소 빌드를 쓴다면 지원 형식 목록을 테스트로 고정해야 한다.

### 2.3 LGPL v2.1 배포 시 일반적으로 따지는 항목 (체크리스트, 결론 아님)

라이선스 본문(LGPL-2.1)에서 읽을 수 있는 항목이다. 각 항목의 충족 여부와 해석은 법률 검토 대상이다.

1. 라이선스 전문과 FFmpeg 사용 사실의 고지 (앱 안의 고지 화면/문서)
2. 해당 FFmpeg 버전의 **소스 코드 제공** 또는 서면 제공 약속 (수정했다면 수정본 포함)
3. 사용자가 라이브러리를 **교체·재링크**할 수 있게 하는 방법 (동적 링크, 또는 정적 링크 시 재링크용 오브젝트 제공)
4. 별도 실행 파일(sidecar)로 호출하는 구조와 라이브러리로 링크하는 구조에서 위 항목의 적용 방식이 다를 수 있음
5. 앱 라이선스 조건이 LGPL 이 부여한 권리(역공학 허용 등)를 제한하지 않을 것

### 2.4 코덱 특허에 관한 사실 기록

AAC, AC-3, WMA 같은 코덱은 특허 풀이나 라이선스 프로그램이 있는 것으로 알려져 있다. 소스 코드의 저작권 라이선스(LGPL/MPL)와
코덱 특허는 **별개의 문제**이며, Symphonia(순수 Rust)로 디코드해도 FFmpeg 로 디코드해도 특허 문제가 사라지지 않는다.
이 실험은 특허 상태를 조사하지 않았다.

## 3. 시스템·플랫폼

- macOS 시스템 프레임워크 (CoreFoundation, CoreVideo, CoreMedia, libSystem): OS 제공, 재배포하지 않음.
- Windows 빌드 시 `windows-sys`/`windows-link`(MIT OR Apache-2.0) 가 추가된다. **Windows 는 이번 실험에서 NOT_RUN.**

## 4. 확인하지 않은 것

- Windows 용 FFmpeg 빌드(MSYS2/MinGW)의 크기·의존 DLL·라이선스 표기
- FFmpeg 를 라이브러리(`libav*`)로 앱에 직접 링크하는 구조 (이번 실험은 별도 프로세스 호출만 검증)
- 코드 서명·공증(notarization) 이후 번들 안의 FFmpeg 실행 가능 여부
- 코덱 특허 상태
