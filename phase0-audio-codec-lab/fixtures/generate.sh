#!/usr/bin/env bash
# U1 픽스처 생성 스크립트. 합성 신호(사인파, 스윕)만 사용한다. 실제 작품의 오디오·대사는 쓰지 않는다.
#
# 사용: FFMPEG=/path/to/ffmpeg ./generate.sh
# 결과는 ./generated/ 에 만들어진다. 생성에 쓴 ffmpeg 버전은 generated/GENERATOR.txt 에 기록한다.
#
# 주의: 손실 압축(MP3/AAC/Opus 등) 파일은 ffmpeg 버전이 달라지면 바이트가 달라질 수 있다.
#       그래서 Windows에서는 이 스크립트로 다시 만들지 말고, macOS에서 만든 generated/ 와 manifest.csv 를
#       그대로 복사한 뒤 `u1lab verify-fixtures` 로 SHA-256 이 같은지 확인한다.
set -euo pipefail

FFMPEG="${FFMPEG:-ffmpeg}"
cd "$(dirname "$0")"
OUT=generated
mkdir -p "$OUT"

BX=(-fflags +bitexact -flags:a +bitexact -flags:v +bitexact -map_metadata -1)
FF() { "$FFMPEG" -nostdin -hide_banner -loglevel error -y "$@"; }

# 기본 스테레오 신호 (L 440Hz, R 660Hz, 진폭 0.5)
st() { echo "aevalsrc=0.5*sin(2*PI*440*t)|0.5*sin(2*PI*660*t):s=$1:d=${2:-10}:c=stereo"; }
# 두 채널에 같은 사인파
dual() { echo "aevalsrc=0.5*sin(2*PI*$1*t)|0.5*sin(2*PI*$1*t):s=$2:d=${3:-10}:c=stereo"; }
VID="testsrc=size=160x120:rate=10:duration=10"

{ echo "ffmpeg: $("$FFMPEG" -version | head -1)"; echo "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"; } > "$OUT/GENERATOR.txt"

# ---- F01~F03 WAV
FF -f lavfi -i "aevalsrc=0.4*sin(2*PI*440*t)+0.2*sin(2*PI*880*t):s=44100:d=10:c=mono" "${BX[@]}" -c:a pcm_s16le "$OUT/F01_wav_pcm16_mono_44k1.wav"
FF -f lavfi -i "$(st 48000)" "${BX[@]}" -c:a pcm_s24le "$OUT/F02_wav_pcm24_stereo_48k.wav"
FF -f lavfi -i "$(st 48000)" "${BX[@]}" -c:a pcm_s16le "$OUT/F03_wav_pcm16_stereo_48k.wav"
# ---- F04~F08
FF -f lavfi -i "$(st 44100)" "${BX[@]}" -c:a libmp3lame -b:a 128k "$OUT/F04_mp3_cbr128_stereo_44k1.mp3"
FF -f lavfi -i "$(st 44100)" "${BX[@]}" -c:a libmp3lame -q:a 2 "$OUT/F05_mp3_vbr_q2_stereo_44k1.mp3"
FF -f lavfi -i "$(st 44100)" "${BX[@]}" -c:a flac "$OUT/F06_flac_stereo_44k1.flac"
FF -f lavfi -i "$(st 44100)" "${BX[@]}" -c:a aac -b:a 128k "$OUT/F07_m4a_aac_lc_stereo_44k1.m4a"
FF -f lavfi -i "$(st 44100)" "${BX[@]}" -c:a alac "$OUT/F08_m4a_alac_stereo_44k1.m4a"
# ---- F09~F12 영상 컨테이너 (영상은 160x120 testsrc)
VOPT=(-c:v libx264 -preset ultrafast -pix_fmt yuv420p)
FF -f lavfi -i "$VID" -f lavfi -i "$(st 44100)" -map 0:v -map 1:a "${VOPT[@]}" -c:a aac -b:a 128k -shortest "${BX[@]}" "$OUT/F09_mp4_h264_aac.mp4"
FF -f lavfi -i "$VID" -f lavfi -i "$(st 44100)" -map 0:v -map 1:a "${VOPT[@]}" -c:a aac -b:a 128k -shortest "${BX[@]}" -f mov "$OUT/F10_mov_h264_aac.mov"
FF -f lavfi -i "$VID" -f lavfi -i "$(st 44100)" -map 0:v -map 1:a "${VOPT[@]}" -c:a aac -b:a 128k -shortest "${BX[@]}" "$OUT/F11_mkv_h264_aac.mkv"
FF -f lavfi -i "$VID" -f lavfi -i "$(st 44100)" -map 0:v -map 1:a "${VOPT[@]}" -c:a flac -shortest "${BX[@]}" "$OUT/F12_mkv_h264_flac.mkv"
# ---- F13 MP4 다중 오디오 트랙 / F14 MKV 다중 오디오 트랙
FF -f lavfi -i "$VID" -f lavfi -i "$(dual 440 44100)" -f lavfi -i "$(dual 880 44100)" \
   -map 0:v -map 1:a -map 2:a "${VOPT[@]}" -c:a aac -b:a 128k -shortest \
   -metadata:s:a:0 language=eng -metadata:s:a:1 language=jpn -disposition:a:0 default -disposition:a:1 0 \
   "${BX[@]}" "$OUT/F13_mp4_multitrack_2aac.mp4"
FF -f lavfi -i "$VID" -f lavfi -i "$(dual 440 48000)" -f lavfi -i "$(dual 880 48000)" -f lavfi -i "$(dual 1320 48000)" \
   -map 0:v -map 1:a -map 2:a -map 3:a "${VOPT[@]}" -c:a:0 aac -b:a:0 128k -c:a:1 flac -c:a:2 ac3 -b:a:2 192k -shortest \
   -metadata:s:a:0 language=jpn -metadata:s:a:1 language=eng -metadata:s:a:2 language=und \
   -disposition:a:0 default -disposition:a:1 0 -disposition:a:2 0 \
   "${BX[@]}" "$OUT/F14_mkv_multitrack_aac_flac_ac3.mkv"
# ---- F13B/F14B: 기본(default) 트랙이 **두 번째**인 다중 트랙. 첫 트랙을 고르는 구현은 440 Hz 를, 기본 표시를 따르는 구현은 880 Hz 를 낸다.
FF -f lavfi -i "$VID" -f lavfi -i "$(dual 440 44100)" -f lavfi -i "$(dual 880 44100)" \
   -map 0:v -map 1:a -map 2:a "${VOPT[@]}" -c:a aac -b:a 128k -shortest \
   -metadata:s:a:0 language=eng -metadata:s:a:1 language=jpn -disposition:a:0 0 -disposition:a:1 default \
   "${BX[@]}" "$OUT/F13B_mp4_default_is_second.mp4"
FF -f lavfi -i "$VID" -f lavfi -i "$(dual 440 48000)" -f lavfi -i "$(dual 880 48000)" \
   -map 0:v -map 1:a -map 2:a "${VOPT[@]}" -c:a:0 aac -b:a:0 128k -c:a:1 flac -shortest \
   -metadata:s:a:0 language=und -metadata:s:a:1 language=jpn -disposition:a:0 0 -disposition:a:1 default \
   "${BX[@]}" "$OUT/F14B_mkv_default_is_second.mkv"
# ---- F15 짧은 오디오 (0.5초)
FF -f lavfi -i "aevalsrc=0.5*sin(2*PI*440*t):s=44100:d=0.5:c=mono" "${BX[@]}" -c:a aac -b:a 64k "$OUT/F15_m4a_aac_short_0s5.m4a"
# ---- 긴 오디오: 60초 스윕을 반복해 1분 / 10분 / 61분을 만든다
BASE="$OUT/.base60.wav"
FF -f lavfi -i "aevalsrc=0.3*sin(2*PI*(300+100*sin(2*PI*t/30))*t)|0.3*sin(2*PI*(400+100*sin(2*PI*t/45))*t):s=44100:d=60:c=stereo" "${BX[@]}" -c:a pcm_s16le "$BASE"
FF -i "$BASE" -t 60 "${BX[@]}" -c:a aac -b:a 64k "$OUT/P01A_m4a_aac_1min.m4a"
FF -stream_loop 9 -i "$BASE" -t 600 "${BX[@]}" -c:a aac -b:a 64k "$OUT/P10A_m4a_aac_10min.m4a"
FF -stream_loop 60 -i "$BASE" -t 3660 "${BX[@]}" -c:a aac -b:a 64k "$OUT/F16_m4a_aac_stereo_61min.m4a"
FF -i "$BASE" -t 60 "${BX[@]}" -c:a libmp3lame -b:a 96k "$OUT/P01M_mp3_1min.mp3"
FF -stream_loop 9 -i "$BASE" -t 600 "${BX[@]}" -c:a libmp3lame -b:a 96k "$OUT/P10M_mp3_10min.mp3"
FF -stream_loop 60 -i "$BASE" -t 3660 "${BX[@]}" -c:a libmp3lame -b:a 96k "$OUT/P60M_mp3_61min.mp3"
rm -f "$BASE"
# ---- F17 잘린 입력 / F18 손상된 컨테이너 (정상 파일에서 결정적으로 만든다)
python3 -I - "$OUT" <<'PY'
import sys, pathlib
o = pathlib.Path(sys.argv[1])
def trunc(src, dst, pct):
    b = (o / src).read_bytes(); (o / dst).write_bytes(b[: len(b) * pct // 100])
def zero(src, dst, a, b_):
    b = bytearray((o / src).read_bytes()); b[a:b_] = bytes(b_ - a); (o / dst).write_bytes(bytes(b))
trunc("F04_mp3_cbr128_stereo_44k1.mp3", "F17_mp3_truncated.mp3", 60)
trunc("F07_m4a_aac_lc_stereo_44k1.m4a", "F17B_m4a_truncated.m4a", 60)
zero("F09_mp4_h264_aac.mp4", "F18_mp4_corrupt_header.mp4", 4, 12)
zero("F12_mkv_h264_flac.mkv", "F18B_mkv_corrupt_header.mkv", 0, 16)
b = bytearray((o / "F09_mp4_h264_aac.mp4").read_bytes()); n = len(b); b[n - 4000:] = bytes(4000)
(o / "F18C_mp4_corrupt_moov.mp4").write_bytes(bytes(b))
PY
# ---- X: Hybrid fallback 시험용 (Symphonia 0.6.1에 없는 코덱·컨테이너)
FF -f lavfi -i "$VID" -f lavfi -i "$(st 48000)" -map 0:v -map 1:a "${VOPT[@]}" -c:a ac3 -b:a 192k -shortest "${BX[@]}" "$OUT/X01_mkv_h264_ac3.mkv"
FF -f lavfi -i "$(st 44100)" "${BX[@]}" -c:a wmav2 -b:a 128k "$OUT/X02_asf_wma.wma"
FF -f lavfi -i "$(st 48000)" "${BX[@]}" -c:a libopus -b:a 96k "$OUT/X03_ogg_opus.opus"
FF -f lavfi -i "$(st 48000)" "${BX[@]}" -c:a pcm_f32le "$OUT/X05_wav_float32_stereo_48k.wav"
# ---- S 스테레오 다운믹스 특수 (48kHz, 풀스케일 0.5 기준)
sx() { echo "aevalsrc=$1|$2:s=48000:d=10:c=stereo"; }
FF -f lavfi -i "$(sx '0.5*sin(2*PI*440*t)' '0.5*sin(2*PI*440*t)')"  "${BX[@]}" -c:a pcm_s16le "$OUT/S01_stereo_inphase.wav"
FF -f lavfi -i "$(sx '0.5*sin(2*PI*440*t)' '0.1*sin(2*PI*440*t)')"  "${BX[@]}" -c:a pcm_s16le "$OUT/S02_stereo_level_diff.wav"
FF -f lavfi -i "$(sx '0.5*sin(2*PI*440*t)' '0')"                    "${BX[@]}" -c:a pcm_s16le "$OUT/S03_stereo_left_only.wav"
FF -f lavfi -i "$(sx '0' '0.5*sin(2*PI*440*t)')"                    "${BX[@]}" -c:a pcm_s16le "$OUT/S04_stereo_right_only.wav"
FF -f lavfi -i "$(sx '0.5*sin(2*PI*440*t)' '-0.5*sin(2*PI*440*t)')" "${BX[@]}" -c:a pcm_s16le "$OUT/S05_stereo_phase_inverted.wav"
FF -f lavfi -i "$(sx '0.5*sin(2*PI*440*t)' '0.5*sin(2*PI*1000*t)')" "${BX[@]}" -c:a pcm_s16le "$OUT/S06_stereo_different_freq.wav"
# ---- SR 샘플레이트 (mono, 1kHz)
for r in 8000 16000 22050 44100 48000 96000; do
  case $r in 8000) n=SR08;; 16000) n=SR16;; 22050) n=SR22;; 44100) n=SR44;; 48000) n=SR48;; 96000) n=SR96;; esac
  FF -f lavfi -i "aevalsrc=0.5*sin(2*PI*1000*t):s=$r:d=10:c=mono" "${BX[@]}" -c:a pcm_s16le "$OUT/${n}_mono_${r}.wav"
done
# ---- CH06 5.1 서라운드 (채널마다 다른 톤)
FF -f lavfi -i "aevalsrc=0.3*sin(2*PI*220*t)|0.3*sin(2*PI*330*t)|0.3*sin(2*PI*440*t)|0.3*sin(2*PI*110*t)|0.3*sin(2*PI*550*t)|0.3*sin(2*PI*660*t):s=48000:d=10:c=5.1" "${BX[@]}" -c:a pcm_s16le "$OUT/CH06_surround_5_1_48k.wav"
echo "done: $(ls "$OUT" | grep -v GENERATOR | wc -l | tr -d ' ') fixtures"
