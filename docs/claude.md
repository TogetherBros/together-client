# Tauri + React + TypeScript — Clean Architecture Rules

> **이 파일을 컨텍스트에 포함하고 코드를 작성할 것**

---

## 폴더 구조

```
src/
├── domain/        # 순수 타입, 인터페이스만. 외부 의존 금지
├── application/   # 비즈니스 로직, hooks
├── infrastructure/# Tauri invoke, WebSocket, OS API
└── ui/            # React 컴포넌트, 스타일만
```

---

## **절대 규칙**

- **`ui/`는 `infrastructure/`를 직접 import 금지** — `application/` 통해서만
- **`domain/`은 React/Tauri import 금지** — 순수 TS만
- **`invoke()`는 반드시 `infrastructure/`에서만 호출**
- **컴포넌트에 비즈니스 로직 작성 금지** — hook으로 분리

---

## 의존성 방향

```
ui → application → infrastructure
         ↓
       domain
```

---

## 네이밍

| 위치 | 규칙 |
|------|------|
| `domain/` | `*.types.ts`, `*.interface.ts` |
| `application/` | `use*.ts` |
| `infrastructure/` | `*.adapter.ts`, `*.client.ts` |
| `ui/` | `*Component.tsx`, `*Screen.tsx` |