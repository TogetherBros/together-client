# Together FE — Claude 작업 가이드

## 브랜치 전략

- `main` — 릴리즈 브랜치. 직접 커밋 금지, 항상 머지로만 반영
- `feat/<name>` — 새 기능
- `fix/<name>` — 버그 수정
- `refactor/<name>` — 리팩토링
- `chore/<name>` — 빌드/설정/문서
- 브랜치 이름은 짧고 영문 소문자 kebab-case

## 커밋 규칙

- **작은 단위로 자주 커밋** — 논리적 변경 1개 = 커밋 1개
- 파일 여러 개를 수정해도 같은 목적이면 한 커밋으로 묶어도 됨
- 커밋 메시지 형식: `<type>: <한국어 설명>`
  - type: `feat` / `fix` / `chore` / `refactor` / `docs` / `style`
- 커밋 메시지는 **무엇**이 아닌 **왜**에 초점

## 버전 업 & 릴리즈 절차

1. 작업 브랜치에서 `package.json` + `src-tauri/tauri.conf.json` 버전 동시 bump
2. `chore: bump version to x.x.x` 커밋
3. `main`으로 `--no-ff` 머지
4. 어노테이티드 태그 생성: `git tag -a vX.X.X -m "vX.X.X - <변경 요약>"`
5. `git push origin main --tags`

버전 규칙 (Semantic Versioning):
- `patch` (1.0.x) — 버그 수정, 소규모 fix
- `minor` (1.x.0) — 새 기능 추가
- `major` (x.0.0) — 호환성 깨지는 변경

## 구현 후 QA 절차

구현이 끝나면 반드시 QA를 수행한다.

1. **TypeScript 타입 체크**: `npx tsc --noEmit`
2. **Rust 컴파일 체크**: `cargo check --manifest-path src-tauri/Cargo.toml`
3. **코드 리뷰 항목**:
   - 버그: 상태 불일치, 레이스 컨디션, null/undefined 미처리
   - 메모리 누수: 이벤트 리스너 미정리, setInterval 미해제
   - 디버그 잔재: `console.log`, `eprintln!`, TODO/FIXME
   - 데드코드: import 안 된 컴포넌트/함수/파일
   - React key 안정성
4. 이슈 발견 시 → 수정 → 커밋 → QA 재수행

## 코딩 스타일

- 주석은 **왜(Why)**가 명확할 때만. 코드가 말해주는 What은 생략
- 에러 핸들링은 시스템 경계(외부 API, 사용자 입력)에서만
- 가능하지 않은 시나리오에 대한 방어 코드 추가하지 않음
- 추상화는 실제로 3번 이상 반복될 때만 도입

## 기술 스택 요약

| 영역 | 기술 |
|------|------|
| 프레임워크 | Tauri v2 (Rust + WebView) |
| 프론트엔드 | React 18 + TypeScript + Vite |
| 실시간 통신 | WebSocket / STOMP (`@stomp/stompjs`) |
| 상태 관리 | React useState/useRef (no external store) |
| 캐릭터 창 | 유저별 독립 Tauri WebviewWindow (desktop pet 방식) |
| 알림 | Web Audio API (programmatic, no audio file) |
| 업데이트 | `tauri-plugin-updater` 자동 업데이트 |

## 주요 파일 구조

```
src/
  App.tsx                    # 루트 컴포넌트, 화면 전환 및 전체 상태
  char-main.tsx              # 캐릭터 창 진입점
  hooks/
    useRoom.ts               # WebSocket/STOMP 연결, 방 상태 관리
    useNotificationSound.ts  # 채팅 알림 소리
  components/
    SplashScreen.tsx
    LobbyScreen.tsx
    CharacterSelectScreen.tsx
    ChatScreen.tsx
    CharacterOverlay.tsx     # 개별 캐릭터 창 UI
  utils/
    position.ts              # 캐릭터 창 위치 계산
src-tauri/src/
  lib.rs                     # Tauri 커맨드, 트레이, 캐릭터 창 생성/관리
```

## 화면 전환 흐름

```
splash → lobby → character-select → overlay (캐릭터 창 활성)
                                  → chat    (채팅창 열림)
```

- `overlay` ↔ `chat` 은 서로 전환 가능
- 방 나가기 → `lobby`로 복귀
