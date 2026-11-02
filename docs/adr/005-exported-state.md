# ADR-005: `Exported`를 수명주기 상태가 아닌 내보내기 상태로 모델링

- 상태: **채택** (Q8: 사용자가 「상관없음」으로 위임 → 권고안 채택, 2026-11-02)
- 날짜: 2026-11-02

## 맥락
요구사항은 Recording / Paused / Interrupted / Processing / Editing / **Exported** / Completed를 상태로 정의하고, `Exported`와 `Completed`를 절대 같게 취급하지 말라고 한다. SRT를 내보냈다고 오디오를 삭제하면 안 되고, 사용자가 명시적으로 완료를 선택하고 최종 자막이 정상 저장된 경우에만 오디오를 삭제한다.

문제: 내보낸 뒤에도 사용자는 계속 편집한다. `Exported`를 선형 수명주기의 한 단계로 두면
- 편집할 때마다 Exported ↔ Editing을 오가야 하고,
- 내보낸 SRT가 현재 내용과 다른지 구분하지 못하고,
- 내보내기를 여러 번(원문/번역/이중, 다른 형식) 하는 경우를 표현하기 어렵다.

## 선택지
1. 요구사항 그대로 `Exported`를 상태로 둔다.
2. **수명주기 상태(Ready…Editing, Completed)와 내보내기 상태를 두 축으로 분리**한다.

## 결정
선택지 2.

- 수명주기: Ready · Recording · Paused · Interrupted · Processing · Editing · Completed
- 내보내기 상태 (저장하지 않고 계산): 
  - **없음**: Export 기록 없음
  - **최신**: 가장 최근 Export의 `revision_at_export == Project.revision`
  - **오래됨**: 그 외 (내보낸 뒤 수정됨)
- 사용자에게는 편집기에 「Exported」 배지로 보여 준다 (요구사항의 용어를 유지).

## 불변식
1. 내보내기는 상태 전이를 일으키지 않고 **오디오를 삭제하지 않는다.**
2. `Completed` 진입 조건에는 Export 기록이 포함되지 않는다. 내보내지 않고도 완료할 수 있고, 내보냈다고 완료되지 않는다.
3. 오디오 삭제 트리거는 `Completed` 전이뿐이다. 완료 시 마지막 내보내기가 「오래됨」이면 경고만 한다.

## 결과
- 「Exported ≠ Completed」가 구조적으로 강제된다.
- DATA_MODEL의 `Project.revision`, `Export.revision_at_export`가 이 결정에 필요하다.
- 요구사항의 상태 목록에서 `Exported`가 빠지는 것처럼 보이지만 사용자 경험상 같은 정보를 더 정확하게 제공한다. 사용자가 상태 목록 그대로를 원하면(Q8) 선택지 1로 되돌릴 수 있으나, 위 세 문제를 감수해야 한다.
