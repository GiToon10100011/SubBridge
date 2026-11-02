# SubBridge 기술 스택

> 근거 표시: ✅ 1차 출처 / ◐ 2차 출처 / ❓ 미확인 (Phase 0 실측). 조사 시점 2026-11-02.
> 개발 장비: Apple M5 · 32GB · macOS 26.6.2 (직접 확인).

## 1. 선택

| 영역 | 선택 | 비고 |
|---|---|---|
| 앱 프레임워크 | Tauri 2 · Rust (tokio) · React + TypeScript + Vite | 같은 구성의 선례: Vibe, uniCaptions, my-translator |
| UI 상태·목록 | Zustand · TanStack Virtual | 수천 줄 가상 스크롤 |
| 파형 | wavesurfer.js v7 + Rust에서 미리 계산한 peaks | 긴 오디오를 WebView에서 디코딩하지 않는다 |
| 오디오 재생 | Rust 재생 엔진 (cpal/rodio), UI는 커서만 표시 | WebView 코덱 차이를 피하고 구간 반복을 샘플 단위로 정확하게 |
| 캡처 | macOS 26+: Core Audio Process Tap 단독 (ADR-001) · Windows: WASAPI (+앱 단위) · Linux: PipeWire | OS별 모듈로 분리 |
| 음성 인식 | whisper.cpp (whisper-rs) + 내장 Silero VAD · Apple SpeechAnalyzer (macOS 26 — Phase 0 결과에 따라 MVP 편입) | |
| 번역 | OpenAI 호환 Provider 1종 → llama-server 사이드카(기본) · Ollama(선택) · 클라우드(선택) · Apple Translation (Phase 0 결과에 따라 MVP 편입) · 자체 번역 엔진 (형태 미정, Q1) | ADR-002 |
| 오디오 처리 | rubato (리샘플) · hound (WAV) | FFmpeg는 MVP 제외 (아래 §3) |
| 저장 | 프로젝트별 SQLite (rusqlite, WAL) + 앱 전역 DB | ADR-003 |
| 자막 형식 | SRT (MVP) · VTT (v0.2) | Exporter 인터페이스 |
| 배포 | macOS Developer ID 서명 + 공증, 직접 배포 · 최소 macOS 26 | Mac App Store 샌드박스에서 탭이 되는지 ❓ |

## 2. 대안 비교

| 결정 | 선택 | 대안 | 선택 이유 / 대안의 단점 |
|---|---|---|---|
| 프레임워크 | Tauri 2 | Electron | Tauri는 메모리·번들이 작고 Rust로 실시간 오디오를 직접 다룬다. Electron은 Linux 렌더링이 일정하다는 장점이 있으나 오디오 코어를 네이티브 모듈로 따로 짜야 한다 |
| macOS 캡처 | Core Audio Process Tap | ScreenCaptureKit | 탭은 오디오 권한만 필요하고 앱 단위 캡처·자기 프로세스 제외가 가능하다. SCK는 화면 녹화 권한이 필요하고 Sequoia에서 매달 재승인 ◐. 최소 macOS가 26이라 OS 호환용 대체 경로가 필요 없어 MVP에서 제외 (Phase 0에서 탭이 실패하면 재도입) |
| LLM 런타임 | llama-server 사이드카 | Ollama 필수 | Ollama는 별도 설치가 필요해 원칙 1과 충돌한다. 사이드카는 LLM이 죽어도 캡처가 유지된다 |
| 저장 | SQLite 단일 원본 | 파일(JSON+SRT) | 자동 저장, 트랜잭션, 부분 갱신, undo 이력이 필요하다. 파일 기반은 원본이 둘로 갈린다 |
| 재생 | Rust 엔진 | WebView `<audio>` | WebView 코덱 차이(특히 Linux WebKitGTK), 샘플 단위 구간 반복 불가 |
| 오디오 처리 | rubato + hound | FFmpeg | 캡처가 이미 PCM을 준다. FFmpeg는 LGPL/GPL 빌드 관리와 바이너리 크기만 늘린다 |

## 3. FFmpeg를 MVP에서 제외한 이유와 재검토 조건

캡처가 PCM을 직접 제공하므로 디코딩이 필요 없다. 리샘플과 WAV 기록은 순수 Rust 크레이트로 충분하다.
**「영상·오디오 파일 불러오기」 기능이 로드맵에 들어오면 다시 검토한다** (v0.5+).

## 4. 오픈소스 라이선스

| 구성 요소 | 라이선스 | 상업 배포 | 비고 |
|---|---|---|---|
| whisper.cpp | MIT | 가능 | |
| whisper-rs | Unlicense | 가능 | 2025-07 Codeberg로 이전 ✅. GitHub 저장소는 갱신하지 않는다 |
| llama.cpp | MIT | 가능 | 고지 의무. `llama-cpp-2` 크레이트는 semver를 따르지 않으므로 버전 고정 |
| Ollama | MIT(저장소) ◐ | 선택 Provider로만 | 데스크톱 앱 쪽 라이선스 논란 보도 ◐ — 번들하지 않는다 |
| Vibe | MIT ◐ | 코드 일부 차용 가능 | 라이선스 파일 직접 확인 필요 |
| wavesurfer.js | BSD-3 | 가능 | |
| rubato | MIT | 가능 | |
| hound | Apache-2.0 | 가능 | |
| FFmpeg | LGPL/GPL | 빌드에 따라 다름 | MVP 제외 |
| TeroSubtitler | MPL-2.0 | 파일 단위 카피레프트 | Pascal — 통합하지 않고 UX만 참고 |

## 5. 번역·ASR 모델 라이선스

| 모델 | 라이선스 | 판정 |
|---|---|---|
| Qwen3.5 (0.8B/2B/4B/9B) | Apache-2.0 ◐ | 후보 |
| Gemma 4 (E2B · E4B · 26B MoE · 31B) | Apache-2.0 ◐ | 후보 |
| TranslateGemma | Gemma 이용약관 — 상업 가능, 금지 정책을 하위 사용자에게 전달 ◐ | 후보 (배포 범위에 따라) |
| Hunyuan-MT / HY-MT1.5 | EU·영국·**대한민국**을 적용 지역에서 제외 ◐ | **제외** |
| NLLB-200 | CC-BY-NC | 상업 배포 시 제외 |
| OPUS-MT | CC-BY | 경량 대체 후보 |
| kotoba-whisper v2.0 | Apache-2.0 | 최종 패스 후보 (ggml 변환본 ❓) |
| anime-whisper | ❓ | 라이선스 확인 필요 |

모델 레지스트리에 라이선스 필드를 두고, 앱에서 모델별 라이선스를 표시한다.

## 6. 버전 고정 정책

- `llama-cpp-2`, `whisper-rs`는 정확한 버전으로 고정한다 (semver 보장 없음).
- 모델은 sha256으로 고정하고 다운로드 시 검증한다.
- Tauri 플러그인과 코어는 같은 마이너 버전으로 올린다.
- 모든 외부 라이브러리 업데이트는 Phase 0 벤치(지연·CER)를 다시 돌려 회귀를 확인한다.

## 7. 메모리 가이드 (Q4 기준 약 0.6GB × 파라미터(B) + 문맥) ◐

| RAM | 실시간 ASR | 실시간 번역 | 최종 패스 |
|---|---|---|---|
| 8GB | turbo 양자화 | 2B~4B 또는 Apple Translation | 같은 모델 |
| 16GB | turbo | 4B급 | 9B급 (~6GB) |
| 32GB+ | turbo | 4B~9B | 26B MoE (~16GB+, 활성 파라미터 4B여도 가중치 전부 상주) |

실시간 ASR과 LLM의 GPU 경합은 ❓ — Phase 0에서 측정한다.

## 8. 자체 번역 엔진을 고려한 구조 (Q1, 형태 미정)

사용자가 자체 번역 엔진을 개발할 가능성이 크다고 밝혔다. 엔진 형태가 정해지지 않았으므로 구조로 대비한다.

| 형태 | 연결 방법 | 주의 |
|---|---|---|
| 로컬 모델 (파인튜닝 등) | GGUF로 변환해 llama-server 사이드카에 올린다 (ADR-002) | **기반 모델의 라이선스를 상속한다.** Hunyuan 계열·NLLB 제외, Gemma 계열은 이용약관 조건. 배포 범위(Q1) 전에는 Apache-2.0 계열(Qwen3.5, Gemma 4 ◐)을 기본 후보로 둔다 |
| 자체 서버/API | OpenAI 호환 Provider 또는 전용 Provider로 연결 | 원칙 5(로컬 기본)와 충돌 → 옵트인 + 전송 내용 고지 (RISK_ANALYSIS R9, R12) |

어느 쪽이든 TranslationProvider 계약(PIPELINE §6.1)만 지키면 파이프라인은 바뀌지 않는다. 엔진 교체는 Phase 0 평가셋 점수로 판단한다.

## 9. 최소 사양

- macOS 26 이상 (Q3). Phase 0 개발 장비: Apple M5 · 32GB · macOS 26.6.2.
- **Apple Silicon 전용** (Q4). Intel Mac은 지원하지 않으며 빌드는 arm64 단일이다.
- RAM 8GB 최소 프로파일(turbo 양자화 + 2B~4B 번역 또는 Apple Translation)을 목표로 한다. 개발 장비가 32GB라 8GB는 직접 검증할 수 없다 → Phase 0에서 메모리 상한을 걸어 근사 측정하거나 8GB 기기를 확보한다 ❓.

## 10. 재생 모드 싱크 기술 후보 (ADR-006) — 전부 ❓

| 방식 | 후보 | 비고 |
|---|---|---|
| 오디오 지문 | 구현체 미정 (직접 구현 또는 기존 라이브러리) | **라이선스 확인 필요 ❓**. 같은 영상의 다른 버전(광고·더빙)에서의 견고함 ❓ |
| ASR 텍스트 매칭 | 기존 ASR(whisper.cpp / SpeechAnalyzer) + 퍼지 문자열 매칭 | 별도 모델 불필요. 실시간 ASR 상시 구동 비용 ❓ |

Phase 0 실험 후에 하나 또는 둘 다 채택한다. 라이브러리를 고르면 TECH_STACK §4 라이선스 표에 추가한다.
