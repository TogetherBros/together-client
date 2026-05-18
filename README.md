# Together

> 친구들과 함께, 바탕화면 위에서

<p align="center">
  <img src="https://raw.githubusercontent.com/TogetherBros/together-client/main/image/together_logo.png" width="120" alt="Together Logo" />
</p>

친구들의 캐릭터가 내 바탕화면 위에서 살아 움직이는 **데스크탑 펫 오버레이 앱**입니다.  
지금 뭘 하고 있는지 실시간으로 보이고, 말풍선으로 채팅도 할 수 있어요.

---

## 다운로드

[**최신 버전 다운로드 →**](https://github.com/TogetherBros/together-client/releases/latest)

| 운영체제 | 파일 |
|---|---|
| Windows | `.msi` 또는 `.exe` |
| macOS (Intel) | `.dmg` (x64) |
| macOS (Apple Silicon) | `.dmg` (aarch64) |
| Linux | `.deb` 또는 `.AppImage` |

---

## 설치 방법

**Windows**  
다운로드한 `.msi` 또는 `.exe` 파일을 실행하고 안내에 따라 설치하세요.

**macOS**  
`.dmg` 파일을 열고 Together 아이콘을 Applications 폴더로 드래그하세요.  
처음 실행 시 "개발자를 확인할 수 없음" 메시지가 뜨면, **우클릭 → 열기**로 실행하세요.

**Linux**  
- `.deb`: `sudo dpkg -i together_*.deb`
- `.AppImage`: 파일에 실행 권한 부여 후 바로 실행

---

## 사용 방법

1. 앱을 실행하면 닉네임을 입력하는 화면이 나타납니다
2. **방 만들기**를 누르면 6자리 코드가 생성됩니다 — 친구에게 공유하세요
3. 친구는 **방 입장**에 코드를 입력하면 바로 참여됩니다
4. 캐릭터를 고른 뒤 입장하면 친구들의 캐릭터가 바탕화면 위에 나타납니다

---

## 주요 기능

**바탕화면 데스크탑 펫**  
친구들 캐릭터가 각자 독립된 창으로 화면 하단에 떠오릅니다. 드래그해서 원하는 위치로 이동할 수 있어요.

**실시간 활동 표시**  
지금 VS Code, Chrome, 게임 등 어떤 앱을 쓰고 있는지 말풍선에 자동으로 표시됩니다.

**채팅 & 알림 소리**  
메시지를 보내면 캐릭터 머리 위 말풍선에 잠깐 떠오릅니다.  
새 메시지가 오면 알림 소리가 울리며, 트레이에서 전체 알림을 끌 수 있어요.

**캐릭터 크기 조절**  
트레이 메뉴에서 작게 · 기본 · 크게 세 단계로 캐릭터 크기를 조절할 수 있습니다.

**트레이 상주**  
채팅 창을 닫아도 앱은 시스템 트레이에 계속 살아있습니다.  
트레이 아이콘을 클릭하면 열기 · 방 나가기 · 종료 메뉴가 나타납니다.

---

## 캐릭터

입장 시 10종 중 하나를 직접 선택합니다. 같은 방 안에서 같은 캐릭터는 나오지 않아요.

<table align="center">
  <tr>
    <td align="center">
      <img src="https://raw.githubusercontent.com/TogetherBros/together-client/main/image/together_dog.png" width="80" /><br/>강아지
    </td>
    <td align="center">
      <img src="https://raw.githubusercontent.com/TogetherBros/together-client/main/image/together_cat.png" width="80" /><br/>고양이
    </td>
    <td align="center">
      <img src="https://raw.githubusercontent.com/TogetherBros/together-client/main/image/together_bunny.png" width="80" /><br/>토끼
    </td>
    <td align="center">
      <img src="https://raw.githubusercontent.com/TogetherBros/together-client/main/image/together_bear.png" width="80" /><br/>곰
    </td>
    <td align="center">
      <img src="https://raw.githubusercontent.com/TogetherBros/together-client/main/image/together_bird.png" width="80" /><br/>새
    </td>
  </tr>
  <tr>
    <td align="center">
      <img src="https://raw.githubusercontent.com/TogetherBros/together-client/main/image/together_bee.png" width="80" /><br/>꿀벌
    </td>
    <td align="center">
      <img src="https://raw.githubusercontent.com/TogetherBros/together-client/main/image/together_monkey.png" width="80" /><br/>원숭이
    </td>
    <td align="center">
      <img src="https://raw.githubusercontent.com/TogetherBros/together-client/main/image/together-koala.png" width="80" /><br/>코알라
    </td>
    <td align="center">
      <img src="https://raw.githubusercontent.com/TogetherBros/together-client/main/image/together_kakatu.png" width="80" /><br/>카카투
    </td>
    <td align="center">
      <img src="https://raw.githubusercontent.com/TogetherBros/together-client/main/image/together-dolphin.png" width="80" /><br/>돌핀
    </td>
  </tr>
</table>

---

## 자동 업데이트

새 버전이 출시되면 앱 실행 시 자동으로 감지하여 로비 화면에 안내가 표시됩니다.  
**지금 업데이트** 버튼을 누르면 백그라운드에서 다운로드 후 재시작하면 완료됩니다.

---

## 기술 스택

- [Tauri v2](https://tauri.app/) (Rust + WebView)
- React 18 + TypeScript
- WebSocket (STOMP)
