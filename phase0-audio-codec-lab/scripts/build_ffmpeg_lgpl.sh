#!/usr/bin/env bash
# U1: LGPL 전용 최소 FFmpeg(ffmpeg + ffprobe, 정적 링크)를 소스에서 빌드한다.
#
# 목적: 설치된 Homebrew FFmpeg 는 --enable-gpl/--enable-version3 빌드라 앱에 번들할 수 없다.
#       "번들 가능한 FFmpeg"가 실제로 어떤 크기이고 U1 픽스처를 처리할 수 있는지 직접 재서 확인한다.
#
# 결과물은 모두 랩 폴더 안(.ffmpeg-lgpl/)에 생기며 git 에서 무시된다.
#
#   사용: scripts/build_ffmpeg_lgpl.sh            (macOS arm64/x86_64, clang + make 필요)
#   소스: https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz
#   SHA-256: 8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e  (Homebrew formula 값과 일치 확인)
#
# Windows(x86_64): MSYS2 MinGW64 셸에서 같은 스크립트를 실행하고 --target-os=mingw32 --arch=x86_64 를 추가한다.
#                  (이번 실험에서는 NOT_RUN — results/windows/WINDOWS_CHECKLIST.md 참조)
set -euo pipefail

VER=9.0.2
SHA=8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
W="$ROOT/.ffmpeg-lgpl"
mkdir -p "$W/src-dl"

if [ ! -f "$W/src-dl/ffmpeg-$VER.tar.xz" ]; then
  curl -sSL -o "$W/src-dl/ffmpeg-$VER.tar.xz" "https://ffmpeg.org/releases/ffmpeg-$VER.tar.xz"
fi
GOT="$(shasum -a 256 "$W/src-dl/ffmpeg-$VER.tar.xz" | awk '{print $1}')"
[ "$GOT" = "$SHA" ] || { echo "SHA-256 mismatch: $GOT"; exit 1; }
[ -d "$W/ffmpeg-$VER" ] || tar -xf "$W/src-dl/ffmpeg-$VER.tar.xz" -C "$W"

mkdir -p "$W/build" && cd "$W/build"

DEMUX=mov,matroska,mp3,flac,wav,ogg,asf,aac,ac3,eac3,aiff,caf
DEC=aac,aac_latm,alac,mp3,mp3float,flac,vorbis,opus,wmav1,wmav2,wmapro,ac3,eac3,pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le,pcm_f64le,pcm_u8,pcm_s16be,pcm_s24be,adpcm_ima_wav,adpcm_ms
PARSE=aac,aac_latm,ac3,flac,mpegaudio,opus,vorbis
# 주의: configure 의 muxer 이름은 CLI 의 포맷 이름(f32le)이 아니라 pcm_f32le 이다.
#       잘못된 이름은 configure 가 경고 없이 넘기고 실행 시점에 "Requested output format 'f32le' is not known" 로 실패한다.
MUX=wav,pcm_f32le,pcm_s16le
ENC=pcm_s16le,pcm_f32le
FILT=aresample,aformat,anull,abuffer,abuffersink

"$W/ffmpeg-$VER/configure" \
  --disable-gpl --disable-version3 --disable-nonfree \
  --disable-everything --disable-autodetect \
  --disable-doc --disable-debug --disable-network \
  --disable-avdevice --disable-swscale --disable-ffplay \
  --enable-ffmpeg --enable-ffprobe \
  --enable-static --disable-shared \
  --enable-demuxer=$DEMUX --enable-decoder=$DEC --enable-parser=$PARSE \
  --enable-muxer=$MUX --enable-encoder=$ENC \
  --enable-protocol=file,pipe --enable-filter=$FILT

make -j"$(sysctl -n hw.ncpu 2>/dev/null || nproc)" ffmpeg ffprobe
ls -l ffmpeg ffprobe
./ffmpeg -hide_banner -L | head -3
