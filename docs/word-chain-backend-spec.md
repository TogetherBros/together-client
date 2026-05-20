# 끝말잇기 백엔드 구현 명세서

## 개요

기존 채팅방 WebSocket 인프라(STOMP) 위에 미니게임 레이어를 추가한다.  
프론트엔드는 `/topic/room/{roomCode}/game` 토픽을 구독하며,  
서버는 게임 이벤트를 해당 토픽으로 브로드캐스트한다.

---

## 1. WebSocket 토픽 & 엔드포인트

### 클라이언트 → 서버 (Publish)

| 엔드포인트 | 설명 |
|-----------|------|
| `/app/room/{roomCode}/game/start` | 게임 생성 (호스트) |
| `/app/room/{roomCode}/game/join` | 게임 참여 |
| `/app/room/{roomCode}/game/word` | 단어 제출 |

### 서버 → 클라이언트 (Subscribe)

| 토픽 | 설명 |
|------|------|
| `/topic/room/{roomCode}/game` | 게임 이벤트 브로드캐스트 (방 전체) |

---

## 2. 메시지 포맷

### 2-1. 게임 생성 요청 (클라 → 서버)

**Destination**: `/app/room/{roomCode}/game/start`

```json
{
  "userId": "string",
  "nickname": "string",
  "gameType": "WORD_CHAIN"
}
```

### 2-2. 게임 참여 요청 (클라 → 서버)

**Destination**: `/app/room/{roomCode}/game/join`

```json
{
  "userId": "string",
  "nickname": "string",
  "gameId": "string (UUID)"
}
```

### 2-3. 단어 제출 요청 (클라 → 서버)

**Destination**: `/app/room/{roomCode}/game/word`

```json
{
  "userId": "string",
  "nickname": "string",
  "gameId": "string (UUID)",
  "word": "string"
}
```

---

## 3. 서버 이벤트 포맷 (공통 구조)

모든 게임 이벤트는 아래 공통 구조를 따른다.  
**Destination**: `/topic/room/{roomCode}/game`

```json
{
  "eventType": "GAME_CREATED | PLAYER_JOINED | GAME_STARTED | WORD_SUBMITTED | GAME_OVER",
  "gameId": "string (UUID)",
  "gameType": "WORD_CHAIN",
  "status": "waiting | playing | ended",
  "hostId": "string",
  "hostNickname": "string",
  "participants": ["userId1", "userId2"],
  "currentTurnUserId": "string",
  "currentTurnNickname": "string",
  "nextChar": "string (한 글자)",
  "turnDeadline": "ISO 8601 string (예: 2024-01-01T00:00:20.000Z)",

  // eventType별 추가 필드 (아래 섹션 참고)
}
```

---

## 4. 이벤트별 상세

### 4-1. `GAME_CREATED` — 게임 생성됨

호스트가 게임을 시작하면 방 전체에 브로드캐스트.  
프론트엔드는 채팅 스트림에 초대 카드를 표시한다.

```json
{
  "eventType": "GAME_CREATED",
  "gameId": "550e8400-e29b-41d4-a716-446655440000",
  "gameType": "WORD_CHAIN",
  "status": "waiting",
  "hostId": "user-uuid-1",
  "hostNickname": "닉네임A",
  "participants": ["user-uuid-1"],
  "currentTurnUserId": "",
  "currentTurnNickname": "",
  "nextChar": "",
  "turnDeadline": ""
}
```

- `participants`에는 호스트 본인이 자동으로 포함
- `status`는 `"waiting"` — 다른 유저의 참여를 기다림
- `currentTurnUserId`, `nextChar`, `turnDeadline`은 빈 값

---

### 4-2. `PLAYER_JOINED` — 유저 참여

새 유저가 참여할 때마다 브로드캐스트.  
프론트엔드는 게임 상태바의 참여 인원 수를 업데이트한다.

```json
{
  "eventType": "PLAYER_JOINED",
  "gameId": "550e8400-...",
  "gameType": "WORD_CHAIN",
  "status": "waiting",
  "hostId": "user-uuid-1",
  "hostNickname": "닉네임A",
  "participants": ["user-uuid-1", "user-uuid-2"],
  "currentTurnUserId": "",
  "currentTurnNickname": "",
  "nextChar": "",
  "turnDeadline": ""
}
```

---

### 4-3. `GAME_STARTED` — 게임 시작

> **트리거 조건**: 참여자가 2명 이상이고 호스트가 별도 시작 신호를 보내거나,  
> 또는 일정 시간(예: 30초) 대기 후 자동 시작. ← **팀 협의 필요**

첫 번째 차례 유저와 시작 단어의 마지막 글자(= `nextChar`)를 전달한다.

```json
{
  "eventType": "GAME_STARTED",
  "gameId": "550e8400-...",
  "gameType": "WORD_CHAIN",
  "status": "playing",
  "hostId": "user-uuid-1",
  "hostNickname": "닉네임A",
  "participants": ["user-uuid-1", "user-uuid-2"],
  "currentTurnUserId": "user-uuid-1",
  "currentTurnNickname": "닉네임A",
  "nextChar": "가",
  "turnDeadline": "2024-01-01T00:00:20.000Z"
}
```

- `nextChar`: 첫 시작 글자 (랜덤 또는 고정값)
- `turnDeadline`: 현재 시각 + 제한 시간(권장 20초)

---

### 4-4. `WORD_SUBMITTED` — 단어 제출됨 (정상)

유저가 유효한 단어를 제출하면 브로드캐스트.  
프론트엔드는 단어를 채팅 스트림에 표시하고 다음 차례로 상태바를 업데이트한다.

```json
{
  "eventType": "WORD_SUBMITTED",
  "gameId": "550e8400-...",
  "gameType": "WORD_CHAIN",
  "status": "playing",
  "hostId": "user-uuid-1",
  "hostNickname": "닉네임A",
  "participants": ["user-uuid-1", "user-uuid-2"],
  "currentTurnUserId": "user-uuid-2",
  "currentTurnNickname": "닉네임B",
  "nextChar": "과",
  "turnDeadline": "2024-01-01T00:00:40.000Z",

  "submitterNickname": "닉네임A",
  "word": "사과"
}
```

- `submitterNickname`, `word`: 방금 제출한 유저와 단어
- `nextChar`: 제출된 단어의 마지막 글자 (다음 차례 시작 글자)
- `currentTurnUserId`: 다음 차례 유저로 업데이트

---

### 4-5. `GAME_OVER` — 게임 종료

게임이 끝나면 브로드캐스트.  
프론트엔드는 채팅 스트림에 게임 종료 카드를 표시하고 상태바를 제거한다.

```json
{
  "eventType": "GAME_OVER",
  "gameId": "550e8400-...",
  "gameType": "WORD_CHAIN",
  "status": "ended",
  "hostId": "user-uuid-1",
  "hostNickname": "닉네임A",
  "participants": ["user-uuid-1", "user-uuid-2"],
  "currentTurnUserId": "",
  "currentTurnNickname": "",
  "nextChar": "",
  "turnDeadline": "",

  "winnerNickname": "닉네임A",
  "loserNickname": "닉네임B",
  "reason": "WRONG_WORD | TIMEOUT | DUPLICATE | NO_WORD"
}
```

**`reason` 값 정의**

| 값 | 설명 |
|----|------|
| `WRONG_WORD` | 앞 단어의 마지막 글자로 시작하지 않거나 사전에 없는 단어 |
| `TIMEOUT` | 제한 시간 내 단어를 제출하지 않음 |
| `DUPLICATE` | 이미 이번 게임에서 사용된 단어 |
| `NO_WORD` | (예비) 해당 글자로 시작하는 단어가 없는 경우 |

---

## 5. 단어 유효성 검사 로직

서버에서 단어 제출 시 아래 순서로 검증한다.

```
1. 게임 존재 & 상태 확인 (playing 상태인지)
2. 제출자가 현재 차례인지 확인
3. 단어가 nextChar로 시작하는지 확인
4. 이번 게임에서 이미 사용된 단어인지 확인 (중복 체크)
5. 표준국립국어원 API로 단어 실존 여부 확인
   └ GET https://stdict.korean.go.kr/api/search.do
       ?key={API_KEY}&q={word}&req_type=json&num=1&type1=word
   └ 결과 없으면 WRONG_WORD로 GAME_OVER
6. 모든 검증 통과 → WORD_SUBMITTED 브로드캐스트
```

---

## 6. 타이머 처리 (서버 사이드)

- `GAME_STARTED` / `WORD_SUBMITTED` 전송 시 서버에서 타이머 시작
- 제한 시간(권장 **20초**) 내 단어 미제출 시 서버가 `GAME_OVER` 브로드캐스트
- `reason: "TIMEOUT"`, `loserNickname`: 시간 초과한 유저

---

## 7. 게임 세션 관리

| 항목 | 내용 |
|------|------|
| 게임 ID | UUID v4 |
| 방당 동시 게임 | 1개만 허용 (진행 중 게임 있으면 신규 생성 거부) |
| 참여자 수 | 최소 2명, 최대 방 인원 전체 |
| 게임 중 퇴장 | 해당 유저가 현재 차례이면 즉시 `GAME_OVER` (reason: `TIMEOUT`) |
| 방 전체 퇴장 | 게임 세션 자동 종료 |
| 게임 종료 후 | 서버에서 해당 게임 세션 메모리 해제 |

---

## 8. 사전 API 연동 (표준국립국어원)

```
Base URL : https://stdict.korean.go.kr/api/search.do
Method   : GET
Params   :
  key       = {발급받은 API 키}
  q         = {검색할 단어}
  req_type  = json
  num       = 1
  type1     = word

성공 응답 예시 (단어 존재):
{
  "channel": {
    "total": "5",
    "item": [{ "word": "사과", ... }]
  }
}

단어 없음:
{
  "channel": {
    "total": "0",
    "item": []
  }
}
```

- `channel.total`이 `"0"` 이거나 `item` 배열이 비어있으면 유효하지 않은 단어로 처리
- API 호출 실패(네트워크 에러, 타임아웃) 시: 단어를 **통과**시키는 것을 권장 (게임 경험 우선)

---

## 9. 에러 처리

단어 제출이 유효하지 않을 때는 `GAME_OVER`로 처리하되,  
게임을 종료시키지 않고 단순 오류 알림만 원한다면 별도 에러 토픽 고려 가능.

> **현재 프론트엔드 구현**: 잘못된 단어 → `GAME_OVER` 브로드캐스트로 처리  
> 재시도 허용 방식으로 변경 시 프론트와 협의 필요

---

## 10. 프론트엔드 연동 확인 체크리스트

백엔드 구현 후 아래 시나리오를 순서대로 테스트한다.

- [ ] 게임 생성 → 채팅 스트림에 초대 카드 표시
- [ ] 다른 유저 참여 → 상태바 참여 인원 업데이트
- [ ] 게임 시작 → 상태바에 차례/글자/타이머 표시
- [ ] 내 차례일 때 입력창 placeholder 변경 + 전송 버튼 "제출"로 변경
- [ ] 올바른 단어 제출 → 채팅 스트림에 단어 카드 표시 + 다음 차례로 전환
- [ ] 잘못된 단어 → 게임 종료 카드 표시
- [ ] 타임아웃 → 게임 종료 카드 표시
- [ ] 게임 종료 후 상태바 사라짐
