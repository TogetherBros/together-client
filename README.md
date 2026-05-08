# Together

> 친구들과 함께, 바탕화면 위에서

친구들의 캐릭터가 내 바탕화면 위에서 살아 움직이는 데스크탑 오버레이 앱입니다.  
각자 무슨 앱을 쓰고 있는지 실시간으로 확인하고, 말풍선으로 채팅을 나눌 수 있습니다.

---

## 주요 기능

### 바탕화면 오버레이
- 친구들의 캐릭터가 화면 하단에 떠다니는 애니메이션으로 표시됩니다
- 오버레이는 항상 최상단에 위치하며, 캐릭터 영역 외 클릭은 아래 앱으로 그대로 전달됩니다
- 캐릭터를 클릭 & 드래그해 원하는 위치로 이동할 수 있으며, 위치는 세션 동안 유지됩니다

### 실시간 활동 감지
- 현재 포커스된 앱(VS Code, Chrome, Discord, 게임 등)을 자동으로 감지해 말풍선에 표시합니다
- YouTube · Netflix · Twitch 등 주요 사이트는 이름을 더 구체적으로 표시합니다
- 채팅 입력 중에는 `. . .` 타이핑 인디케이터로 자동 전환됩니다

### 채팅 & 말풍선
- 채팅 메시지를 보내면 해당 캐릭터 머리 위 말풍선에 3.5초간 표시됩니다
- 채팅 창은 트레이 아이콘 → 열기로 언제든 꺼낼 수 있습니다

### 방 시스템
- 6자리 랜덤 코드로 방을 만들고 친구에게 공유하면 입장할 수 있습니다
- 같은 방의 인원은 모두 실시간으로 동기화됩니다
- 캐릭터 중복 자동 방지 — 같은 캐릭터를 쓰는 사람이 있으면 다른 캐릭터로 랜덤 재배정됩니다

### 트레이 아이콘
- 앱을 닫아도 시스템 트레이에 상주합니다
- 트레이 메뉴: **열기** (채팅 화면) · **방 나가기** · **종료**

---

## 캐릭터

| 강아지 | 고양이 | 토끼 | 곰 | 새 | 꿀벌 |
|--------|--------|------|----|----|------|
| DOG | CAT | BUNNY | BEAR | BIRD | BEE |

입장 시 6종 중 랜덤 배정, 같은 방에서 중복 없이 유지됩니다.

---

## 기술 스택

| 영역 | 기술 |
|------|------|
| 데스크탑 프레임워크 | [Tauri v2](https://tauri.app) (Rust) |
| 프론트엔드 | React 18 + TypeScript + Vite |
| 실시간 통신 | STOMP over WebSocket (`@stomp/stompjs`) |
| Windows API | `winapi` — 커서 위치 · LMB 상태 · 활성 앱 감지 · 커서 아이콘 변경 |
| 스타일 | CSS (커스텀, 프레임워크 없음) |

---

## 동작 구조

```
[Main Window]  ←──── STOMP WebSocket ────→  [Backend Server]
      │
      │  Tauri IPC (users-updated, bubble-updated)
      ▼
[Overlay Window]  ← 항상 최상단, 항상 passthrough
   캐릭터 렌더링 (React)
   드래그 감지: Rust 폴링 (GetAsyncKeyState) → drag-move 이벤트 → JS 포지션 업데이트
   커서 변경: SetCursor WinAPI (캐릭터 위: 손 아이콘 / 드래그 중: 이동 아이콘)
```

- **Main Window**: 로비 · 채팅 UI. 백엔드와 STOMP 연결 관리, 오버레이에 상태 전달
- **Overlay Window**: 항상 passthrough 모드 유지 (`set_ignore_cursor_events(true)`) → 윈도우 모드 전환 없음 → 글리치 없음. 드래그는 Rust가 OS 레벨에서 직접 감지

---

## 시작하기

### 요구사항

- Windows 10 / 11
- [Node.js](https://nodejs.org) 18+
- [Rust](https://rustup.rs) stable
- WebView2 런타임 (Windows 11은 기본 내장, Windows 10은 별도 설치 필요)

### 개발 서버 실행

```bash
npm install
npm run tauri dev
```

### 프로덕션 빌드

```bash
npm run tauri build
```

빌드 결과물: `src-tauri/target/release/bundle/`

---

## 환경 설정

`src/hooks/useRoom.ts` 의 `brokerURL` 을 백엔드 서버 주소로 변경하세요.

```ts
brokerURL: 'ws://YOUR_SERVER_IP:PORT/ws'
```

`src-tauri/tauri.conf.json` CSP의 `connect-src` 도 동일한 주소로 맞춰야 합니다.

> HTTPS 환경에서 배포 시 반드시 `wss://` 를 사용하세요.

---

## 프로젝트 구조

```
together-fe/
├── src/
│   ├── App.tsx                  # 메인 앱 · 화면 전환 · 오버레이 동기화
│   ├── OverlayApp.tsx           # 오버레이 전용 렌더러
│   ├── main.tsx                 # 창 분기 (main / overlay)
│   ├── components/
│   │   ├── SplashScreen.tsx     # 스플래시 화면
│   │   ├── LobbyScreen.tsx      # 로비 (닉네임 · 방 코드 입력)
│   │   ├── ChatScreen.tsx       # 채팅 화면
│   │   └── CharacterCard.tsx    # 오버레이 캐릭터 카드
│   ├── hooks/
│   │   └── useRoom.ts           # STOMP 연결 · 유저/채팅/활동 상태 관리
│   ├── utils/
│   │   └── position.ts          # 캐릭터 기본 위치 계산
│   ├── characters.ts            # 캐릭터 이미지 맵
│   └── types.ts                 # 공통 타입 정의
├── src-tauri/
│   ├── src/lib.rs               # Tauri 명령어 · 드래그 모니터 · 앱 감지
│   ├── tauri.conf.json          # 앱 설정 · 보안 CSP
│   └── capabilities/
│       └── default.json         # 권한 설정
└── image/                       # 캐릭터 · 로고 이미지 (PNG)
```
