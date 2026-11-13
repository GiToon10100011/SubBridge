# fixtures — U1 테스트 입력

모든 파일은 **합성 신호(사인파·스윕)** 로 만든 것이다. 실제 작품의 오디오·영상·대사는 하나도 쓰지 않았고,
DRM이 걸린 자료도 없다. 영상 트랙이 필요한 컨테이너 테스트에는 FFmpeg `testsrc`(160x120)를 썼다.

## 파일 구성

| 경로 | 내용 | git |
|---|---|---|
| `catalog.csv` | 픽스처 정의: 기대 결과, 공칭 길이, 기대 RMS/톤, 시험할 트랙 | 추적 |
| `generate.sh` | 위 정의대로 `generated/` 를 만드는 스크립트 (bash + ffmpeg) | 추적 |
| `manifest.csv` | 생성된 파일의 **SHA-256, 바이트 수, ffprobe 요약** (재현성의 기준) | 추적 |
| `generated/` | 실제 오디오 파일 43개 (약 125 MB) + `GENERATOR.txt` | **무시** (`.gitignore`) |

`generated/` 는 용량 때문에 git에 넣지 않았다. 같은 파일인지는 `manifest.csv` 의 SHA-256 으로 확인한다.

## catalog.csv 열

`id, file, group, description, nominal_duration_s, expected_outcome, lossless, codec_frame_size, expected_rms, expected_tones, tracks_to_test`

- `group`: `basic`(요구서 F01–F18), `fallback`(Symphonia가 지원하지 않는 형식), `special`(스테레오 다운믹스),
  `samplerate`(8/16/22.05/44.1/48/96 kHz), `channels`(5.1), `perf`(성능 측정 전용)
- `expected_outcome`: `ok`(성공해야 함) / `error`(깨끗하게 실패해야 함) / `ok_or_error`(복구 성공도, 깨끗한 실패도 허용하되 한 모드의 반복 실행끼리 결과가 같아야 함)
- `codec_frame_size`: 시간축 허용 오차 계산용 (README.md의 "허용 오차" 참고). 무손실이면 0.
- `expected_tones`: `주파수:진폭`을 `;`로 이어 쓴다. `t1@880:0.5` 는 "오디오 트랙 순번 1의 880 Hz 진폭 0.5".
- `tracks_to_test`: `default` 또는 `0|1|2` (오디오 트랙 순번, 0부터).

## 픽스처 목록

| ID | 내용 | 기대 |
|---|---|---|
| F01 | WAV PCM16 mono 44.1 kHz | ok |
| F02 | WAV PCM24 stereo 48 kHz | ok |
| F03 | WAV PCM16 stereo 48 kHz | ok |
| F04 | MP3 CBR 128k | ok |
| F05 | MP3 VBR q2 | ok |
| F06 | FLAC | ok |
| F07 | M4A AAC-LC | ok |
| F08 | M4A ALAC | ok |
| F09 | MP4 (H.264 + AAC-LC) | ok |
| F10 | MOV (H.264 + AAC-LC) | ok |
| F11 | MKV (H.264 + AAC-LC) | ok |
| F12 | MKV (H.264 + FLAC) | ok |
| F13 | MP4 오디오 2트랙 (440 Hz eng 기본 / 880 Hz jpn) | ok, 트랙 0·1 각각 |
| F14 | MKV 오디오 3트랙 (AAC 440 Hz jpn 기본 / FLAC 880 Hz eng / AC-3 1320 Hz und) | ok, 트랙 0·1·2 각각 |
| F15 | M4A AAC 0.5초 (짧은 파일) | ok |
| F16 | M4A AAC 61분 | ok (perf) |
| F17 / F17B | MP3 / M4A를 바이트 60%에서 자른 것 | ok_or_error |
| F18 / F18B | MP4 `ftyp` 영역 / MKV EBML 헤더를 0으로 덮은 것 | ok_or_error |
| F18C | MP4 끝 4000바이트(moov)를 0으로 덮은 것 | ok_or_error (실제로는 실패해야 정상) |
| X01 | MKV + AC-3 | ok (Symphonia 미지원 → fallback 시험) |
| X02 | WMA (ASF) | ok (같음) |
| X03 | Ogg Opus | ok (같음) |
| X05 | WAV float32 | ok |
| S01–S06 | 스테레오: 동위상 / 레벨 차이 / 왼쪽만 / 오른쪽만 / **위상 반전** / 좌우 다른 주파수 | ok |
| SR08–SR96 | mono 1 kHz 톤, 8/16/22.05/44.1/48/96 kHz | ok |
| CH06 | 5.1(6채널) 채널마다 다른 톤 | ok |
| P01A, P10A, P01M, P10M, P60M | 성능용 AAC 1·10분, MP3 1·10·61분 (스윕 반복) | ok |

F16, P* 는 60초짜리 로그 스윕을 반복한 신호다. 스윕은 비주기적이라 두 디코더 출력의 **정렬(lag)** 을 구할 수 있다.
한 주파수만 쓰는 톤 파일은 주기 때문에 상호상관이 여러 lag에서 1.0이 되어 lag 값이 의미가 없다.

## 재현성

- 생성 시 `-fflags +bitexact -flags:a +bitexact -flags:v +bitexact -map_metadata -1` 로 시간·메타데이터가 파일에 들어가지 않게 했다.
- 같은 머신에서 `generate.sh` 를 두 번 돌려 41개 파일(그 시점의 전부)이 **바이트 단위로 같음**을 확인했다
  (이후 추가된 F18C는 F09에서 파생되므로 F09가 같으면 같다).
- 생성기: Homebrew FFmpeg 9.0.2 (GPL 빌드). 그 버전 기록은 `generated/GENERATOR.txt`.
- **손실 압축 파일은 FFmpeg/인코더 버전이 다르면 바이트가 달라질 수 있다.** 그래서 Windows 에서는
  재생성하지 말고 macOS의 `generated/` 와 `manifest.csv` 를 복사한 뒤 `u1lab verify-fixtures` 로 검증한다.

```sh
# macOS에서 (생성/갱신)
FFMPEG=ffmpeg ./generate.sh
../target/release/u1_audio_codec_lab manifest

# 어느 플랫폼에서든 (검증)
../target/release/u1_audio_codec_lab verify-fixtures     # 43 ok, 0 mismatched 이어야 한다
```

## 한계 (결과 해석 시 주의)

1. **모든 손실 압축 픽스처를 FFmpeg의 인코더(native AAC, libmp3lame, WMA, Opus, AC-3)로 만들었다.**
   Apple/iTunes, Nero, Fraunhofer 등 다른 인코더가 만든 파일, HE-AAC(SBR/PS), 가변 프레임 MP4,
   편집 목록(edit list)이 복잡한 파일은 포함되지 않았다. 실제 사용자 파일에서의 호환성은 이 표본보다 넓을 수 있다.
2. 신호가 단순해서 "디코드가 되는가·시간축이 맞는가"는 보지만 음질(지각 품질)은 평가하지 않는다.
3. 손상 파일(F17–F18C)은 각 한 가지 방식으로만 손상시켰다.
