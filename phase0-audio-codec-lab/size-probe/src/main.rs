//! Symphonia 유무에 따른 릴리스 바이너리 크기 차이를 재기 위한 최소 프로그램.
//! `--features sym` 이면 probe → 기본 오디오 트랙 → 디코드 루프를 실제로 호출해 코드가 최적화로 제거되지 않게 한다.

#[cfg(feature = "sym")]
fn decode_frames(path: &str) -> u64 {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

    let file = std::fs::File::open(path).unwrap();
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut format = symphonia::default::get_probe()
        .probe(&Hint::new(), mss, FormatOptions::default(), MetadataOptions::default())
        .unwrap();
    let track = format.default_track(TrackType::Audio).unwrap();
    let id = track.id;
    let params = track.codec_params.as_ref().unwrap().audio().unwrap().clone();
    let mut dec = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .unwrap();
    let mut frames = 0u64;
    while let Ok(Some(p)) = format.next_packet() {
        if p.track_id != id {
            continue;
        }
        if let Ok(buf) = dec.decode(&p) {
            frames += buf.frames() as u64;
        }
    }
    frames
}

#[cfg(not(feature = "sym"))]
fn decode_frames(path: &str) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_default();
    println!("{}", decode_frames(&path));
}
