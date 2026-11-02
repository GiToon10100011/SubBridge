# SubBridge 데이터 모델

> 개념 모델이다. 컬럼 타입과 인덱스는 구현 단계에서 정한다. 저장소는 프로젝트별 SQLite 한 파일(`project.db`)이 유일한 원본이다 (ADR-003).

## 1. 시간 기준

모든 시간은 **전역 샘플 인덱스(16kHz)** 로 저장한다. 벽시계(`started_at`)는 참고용이다.

- 일시정지 구간은 타임라인에서 빠진다 → SRT 시간 = 「캡처가 진행된 시간」.
- 영상의 재생 위치를 알 수 없으므로, 영상과 어긋나면 편집기의 **전체 이동 · 싱크 맞추기 · 지점별 간격 삽입/삭제** 도구로 보정한다.
- 전체 이동 값은 `Project.global_offset_ms`로 저장하고 내보낼 때 모든 큐에 더한다.
- ms로 바꾸는 것은 표시·내보내기 시점뿐이다.

## 2. 엔티티

### Project
| 필드 | 설명 |
|---|---|
| id | uuid |
| name | |
| source_lang / target_lang | |
| state | Ready / Recording / Paused / Interrupted / Processing / Editing / Completed |
| state_reason | 마지막 전이 사유 |
| **revision** | 편집이 커밋될 때마다 +1. 내보내기 상태 계산의 기준 |
| audio_status | present / deleting / deleted |
| duration_ms | |
| **global_offset_ms** | 편집기 「전체 이동」 값 (기본 0). 내보낼 때 모든 큐에 더한다. 변경하면 `revision`이 오른다 |
| kind | captured(SubBridge가 캡처해 만든 프로젝트) / imported(불러온 SRT) |
| **playback_adjust_ms** | 재생 모드의 수동 보정값 (기본 0, 프로젝트별 저장). 자동 싱크 결과 위에 더한다 |
| engine_profile | 사용한 ASR·번역 모델 구성 |
| schema_version | 마이그레이션용 |
| created_at / updated_at | |

### StateTransition (ProjectState 이력)
`project_id · from · to · reason · at`

### RecordingSession — 연속 캡처 구간 1개 (일시정지하면 끝난다)
`project_id · seq · timeline_start_sample · sample_count · started_at / ended_at · end_reason(stop/pause/crash/sleep/device/disk) · source(kind=app(MVP) · 번들 ID · 앱 이름) · backend(tap/wasapi/pipewire) · native_rate`

### AudioChunk
`session_id · seq · rel_path · timeline_start_sample · sample_count · byte_size · checksum · status(writing/sealed/repaired/deleted)`

### Segment (SubtitleSegment)
| 필드 | 설명 |
|---|---|
| project_id | |
| start_ms / end_ms | 표시용. 원본은 샘플 인덱스에서 계산 |
| order_key | 분할·병합 시 재정렬을 피하는 분수 인덱스 |
| origin | realtime / final / user / import(불러온 SRT) |
| status | interim / confirmed / final / user_edited |
| text | 현재 원문 |
| machine_text | 기계가 낸 원문 (수정 여부 비교용) |
| asr_meta | 모델, logprob, no_speech |
| flags | music / 환각 의심 / 저신뢰 |
| speaker | 화자 구분용 예약 필드 (미사용) |
| deleted | 소프트 삭제 |

### Translation
| 필드 | 설명 |
|---|---|
| segment_id · target_lang | |
| text | |
| origin | realtime / final / retranslate / user |
| status | pending / done / failed / **stale** / user_edited |
| provider · model · prompt_version | 재현성 |
| source_hash | 원문이 바뀌면 불일치 → `stale` 표시 |

별도 테이블로 둔 이유: 언어 확장이 쉽고, 원문 수정 시 번역만 stale로 표시할 수 있다.

### SyncFingerprint (재생 모드용, 지문 방식이 채택될 때만)
`project_id · algo · algo_version · hop_ms · data(blob) · created_at`

- 캡처 중 또는 Processing에서 만든 **짧은 구간별 식별 정보**다. 오디오 자체가 아니며, 재생 모드가 영상 위치를 찾는 데 쓴다 (ADR-006, PIPELINE §10).
- 오디오가 삭제된 뒤에도 재생 모드가 동작하려면 남겨야 한다 `[결정 대기: Q20]`.
- 불러온 SRT(`kind=imported`)에는 없다 → 텍스트 매칭을 쓴다.

### GlossaryTerm (v0.2)
`project_id · source · target · kind(인명/지명/조직/용어) · note`

### EditLog
`project_id · revision · entity · entity_id · op(text/retime/split/merge/delete/insert) · before · after · at` → undo/redo와 「수정됨」 표시.

### Job
`project_id · kind(final_asr/final_mt/retranslate/peaks/export/audio_delete) · params · status · progress · attempts · last_error · checkpoint`

### Export
| 필드 | 설명 |
|---|---|
| project_id | |
| kind | original / translated / bilingual |
| format | srt / vtt |
| path | 사용자가 고른 경로 |
| options | BOM, 줄바꿈, 이중 자막 순서 |
| **revision_at_export** | 내보낸 시점의 Project.revision |
| file_hash | |
| at | |

**내보내기 상태** = 내보낸 적 없음 / **최신**(`revision_at_export == Project.revision`) / **오래됨**(그 외).
이 값은 저장하지 않고 계산한다. `Completed`와는 무관하다.

### 전역 DB (앱 단위)
- **Model**: 파일·크기·sha256·라이선스·설치일
- **RecentProject**: 경로, 마지막 열림

## 3. 무결성 규칙

- 편집 1건 = 트랜잭션 1건 (`Project.revision` 증가 + `EditLog` 기록 포함).
- 전체 이동(`global_offset_ms`) 변경도 편집 1건이다 → `revision`이 오르고, 이미 내보낸 SRT는 「오래됨」이 된다.
- 상태 전이와 오디오 삭제 단계 기록은 `synchronous=FULL`, 일반 편집은 WAL + `NORMAL`.
- `Segment.origin=final` 결과는 `status=user_edited`인 줄을 덮어쓰지 않는다.
- `audio_status`가 `deleted`여도 Segment/Translation 편집은 계속 가능하다. 오디오 의존 기능(재생·구간 반복·재전사)만 꺼진다.
- `Completed`로의 전이는 사용자 명시 동작 + 진행 중 Job 없음 + 무결성 검사 통과일 때만 허용한다. **Export 기록은 이 조건에 포함되지 않는다.**
- 내보내기(Export 기록 생성)는 `audio_status`를 바꾸지 않는다. **오디오를 삭제하지 않는다.** 삭제는 `Completed` 전이 뒤의 `audio_delete` Job뿐이다 (ADR-005).

## 4. 마이그레이션 정책

- `schema_version`을 `Project`에 두고, 열 때 순차 마이그레이션을 적용한다.
- 마이그레이션 전에 `project.db`를 같은 폴더에 `.bak`으로 복사한다.
- 새 버전이 만든 프로젝트를 구버전이 열려고 하면 거부하고 안내한다 (데이터 손상 방지).

## 5. 백업·교환

JSON은 저장 형식이 아니라 **백업·교환용 내보내기**다. 오디오는 백업에서 제외한다.
