# ADR-001: macOS 오디오 캡처 API

- 상태: **개정** — Q3(최소 macOS 26) 반영. 구현 방식(직접 구현 vs cpal)은 Phase 0에서 확정
- 날짜: 2026-11-02 (최초 제안 → 같은 날 개정)
- 개정 이력: 최초안은 「탭 주 경로 + ScreenCaptureKit 대체 경로」였다. 최소 macOS가 26으로 정해져 대체 경로가 필요 없어졌다.

## 맥락
앱이 재생하는 소리를 별도 드라이버 없이 받아야 한다 (원칙 1). 권한 부담이 작고, 앱 단위 선택과 자기 소리 제외가 가능해야 한다. 최소 macOS는 26이다 (Q3).

## 선택지
| 방식 | 장점 | 단점 |
|---|---|---|
| **Core Audio Process Tap** | 오디오 권한만 필요 ✅. 특정 프로세스 포함/제외 가능 ✅ | 권한 상태 확인 API 없음, 거부 시 무음 버퍼 ✅ |
| cpal 0.17 루프백 | 순수 Rust, 크로스플랫폼 인터페이스 ✅ (내부는 Core Audio 탭) | 앱 단위 선택 지원 여부 ❓ |
| ScreenCaptureKit | 화면과 함께 캡처 가능 | 화면 녹화 권한 필요, Sequoia 매달 재승인 ◐, 포크에서 병렬 캡처 크래시 보고 ◐ |
| BlackHole 등 가상 드라이버 | — | 별도 설치 필요 → 원칙 1 위반 |

## 결정
**Core Audio Process Tap 단독.** ScreenCaptureKit는 MVP에서 쓰지 않는다 — 원래 macOS 13~14.3 호환을 위한 대체 경로였는데 Q3로 필요 없어졌다. 단 Phase 0에서 탭이 (권한 UX, 장치 변경, 특정 앱 등) 실패하면 재도입한다. 가상 드라이버는 쓰지 않는다.

Phase 0에서 cpal 루프백을 쓸지, 탭을 직접 구현(objc2 또는 swift-rs)할지 비교해 최종 결정한다.

## 결과
- `CaptureBackend` 인터페이스 뒤에 구현을 숨긴다 → Windows/Linux 추가 시 영향 없음.
- 권한 확인이 불가능하므로 「테스트 소리 + 무음 감지」 온보딩이 필요하다.
- 출력 장치 변경 시 재구성 로직이 필요하다 (R7).
- Q6 결정: **앱 단위** (ARCHITECTURE §12). 열린 질문: 브라우저 헬퍼 프로세스 묶기 ❓, Mac App Store 샌드박스 호환 ❓.

## 근거
Apple Core Audio Taps 문서, AudioCap 샘플, Apple 지원 가이드, cpal 0.17 소스, screenpipe#2084, TidBITS Sequoia 기사 (RISK_ANALYSIS 및 PLAN 출처 목록).
