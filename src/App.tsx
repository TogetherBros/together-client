import { useState, useEffect, useRef } from 'react';
import { flushSync } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import { listen, emit } from '@tauri-apps/api/event';
import { check } from '@tauri-apps/plugin-updater';
import { AppScreen, Character, RoomConfig, UserState } from './types';
import { characterMeta } from './characters';
import { getWindowPosition } from './utils/position';
import { useRoom } from './hooks/useRoom';
import { useNotificationSound } from './hooks/useNotificationSound';

const charLabelMap = Object.fromEntries(characterMeta.map(c => [c.type, c.label]));
import SplashScreen from './components/SplashScreen';
import LobbyScreen from './components/LobbyScreen';
import CharacterSelectScreen from './components/CharacterSelectScreen';
import ChatScreen from './components/ChatScreen';
import './App.css';

// ── 세션 복구 (F5 대응) ────────────────────────────────────────────────────────

const SESSION_KEY = 'together_session';

interface SavedSession {
  config: RoomConfig;
  character: Character;
}

function loadSession(): SavedSession | null {
  try {
    const raw = sessionStorage.getItem(SESSION_KEY);
    return raw ? (JSON.parse(raw) as SavedSession) : null;
  } catch {
    return null;
  }
}

function saveSession(config: RoomConfig, character: Character) {
  sessionStorage.setItem(SESSION_KEY, JSON.stringify({ config, character }));
}

function clearSession() {
  sessionStorage.removeItem(SESSION_KEY);
}

// ── DeviceId ──────────────────────────────────────────────────────────────────

function getDeviceId(): string {
  const key = 'together_device_id';
  const stored = localStorage.getItem(key);
  if (stored) return stored;
  const id = crypto.randomUUID();
  localStorage.setItem(key, id);
  return id;
}

// ── App ───────────────────────────────────────────────────────────────────────

function App() {
  const restoredSession = useRef(loadSession()).current;

  const [screen, setScreen] = useState<AppScreen>(restoredSession ? 'chat' : 'splash');
  const [config, setConfig] = useState<RoomConfig | null>(restoredSession?.config ?? null);
  const [selectedCharacter, setSelectedCharacter] = useState<Character | null>(restoredSession?.character ?? null);
  const [pendingRejoin, setPendingRejoin] = useState<Character | null>(restoredSession?.character ?? null);

  const [deviceId] = useState<string>(getDeviceId);
  const [updateInfo, setUpdateInfo] = useState<{ version: string } | null>(null);
  const [updateState, setUpdateState] = useState<'idle' | 'downloading' | 'done'>('idle');
  const [lobbyError, setLobbyError] = useState<string | null>(null);
  const screenRef = useRef(screen);
  screenRef.current = screen;

  const { users, messages, bubbleMessages, error, characterError, clearCharacterError, activityRef, takenCharacters, isConnected, sendChat, sendActivity, joinWithCharacter } =
    useRoom(config);

  const usersRef = useRef<UserState[]>(users);
  const bubbleRef = useRef<Record<string, string>>(bubbleMessages);
  usersRef.current = users;
  bubbleRef.current = bubbleMessages;

  // ── 세션 복구: WS 연결되면 자동 재입장 ────────────────────────────────────
  useEffect(() => {
    if (!pendingRejoin || !isConnected) return;
    const char = pendingRejoin;
    setPendingRejoin(null);
    joinWithCharacter(char).then((users) => {
      if (!users) {
        clearSession();
        setSelectedCharacter(null);
        setConfig(null);
        setScreen('lobby');
      }
    });
  }, [isConnected, pendingRejoin]);

  useEffect(() => {
    if (restoredSession) {
      invoke('show_main_window').catch(console.error);
    }
  }, []);

  // ── 세션 저장 ─────────────────────────────────────────────────────────────
  useEffect(() => {
    if ((screen === 'chat' || screen === 'overlay') && config && selectedCharacter) {
      saveSession(config, selectedCharacter);
    }
  }, [screen, config, selectedCharacter]);

  // ── 알림 소리 ──────────────────────────────────────────────────────────────
  const playNotification = useNotificationSound();
  const msgCountRef = useRef(0);
  useEffect(() => {
    if (messages.length > msgCountRef.current) {
      const last = messages[messages.length - 1];
      if (last && last.userId !== config?.deviceId) playNotification();
    }
    msgCountRef.current = messages.length;
  }, [messages, playNotification, config]);

  // ── 스플래시 타임아웃 ─────────────────────────────────────────────────────
  useEffect(() => {
    if (screen !== 'splash') return;
    const t = setTimeout(() => setScreen('lobby'), 1800);
    return () => clearTimeout(t);
  }, []);

  // ── 업데이트 확인 (로비 진입 시 1회) ─────────────────────────────────────
  const updateChecked = useRef(false);
  useEffect(() => {
    if (screen !== 'lobby' || updateChecked.current || import.meta.env.DEV) return;
    updateChecked.current = true;
    check().then(update => {
      if (update?.available) setUpdateInfo({ version: update.version });
    }).catch(() => {});
  }, [screen]);

  // ── 서버 에러 → 로비 ──────────────────────────────────────────────────────
  useEffect(() => {
    if (!error) return;
    clearSession();
    setLobbyError(error);
    setConfig(null);
    setScreen('lobby');
    invoke('leave_overlay').catch(console.error);
  }, [error]);

  // ── 앱 감지 (3초마다) ─────────────────────────────────────────────────────
  useEffect(() => {
    if (!config) return;
    const poll = async () => {
      if (activityRef.current === '입력 중...') return;
      try {
        const app = await invoke<string>('get_active_app');
        activityRef.current = app;
      } catch {}
    };
    poll();
    const interval = setInterval(poll, 3000);
    return () => clearInterval(interval);
  }, [config, activityRef]);

  // ── 캐릭터 창 동기화 ──────────────────────────────────────────────────────
  useEffect(() => {
    if (!config || screen === 'splash' || screen === 'lobby' || screen === 'character-select') return;
    if (users.length === 0) return;

    const usersJson = JSON.stringify(users);
    const positions = users.map((_u, i) => getWindowPosition(i, users.length));
    invoke('sync_char_windows', {
      userIds: users.map((u: UserState) => u.userId),
      positions,
      usersJson,
    }).catch(console.error);

    emit('users-updated', users).catch(console.error);
    invoke('update_overlay_users', {
      userIds: users.map((u: UserState) => u.userId),
      userLabels: users.map((u: UserState) => `${u.nickname} (${charLabelMap[u.character] ?? u.character})`),
    }).catch(console.error);
  }, [users, config, screen]);

  // ── 말풍선 브로드캐스트 ───────────────────────────────────────────────────
  useEffect(() => {
    if (!config || screen === 'splash' || screen === 'lobby') return;
    emit('bubble-updated', bubbleMessages).catch(console.error);
  }, [bubbleMessages, config, screen]);

  // ── 캐릭터 창 준비 완료 → 최신 상태 즉시 전송 ───────────────────────────
  useEffect(() => {
    const unlisten = listen('char-ready', () => {
      const s = screenRef.current;
      if (s === 'splash' || s === 'lobby') return;
      emit('users-updated', usersRef.current).catch(console.error);
      emit('bubble-updated', bubbleRef.current).catch(console.error);
    });
    return () => { unlisten.then(f => f()); };
  }, [deviceId]);

  // ── 트레이 "열기" / 앱 재실행 / macOS 독 클릭 ───────────────────────────
  useEffect(() => {
    const unlisten = listen('open-chat', () => {
      const s = screenRef.current;
      flushSync(() => {
        if (s === 'overlay' || s === 'chat') setScreen('chat');
        else if (s === 'splash') setScreen('lobby');
      });
      invoke('show_main_window').catch(console.error);
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  // ── 트레이 "방 나가기" ────────────────────────────────────────────────────
  useEffect(() => {
    const unlisten = listen('leave-overlay', () => {
      clearSession();
      flushSync(() => {
        setConfig(null);
        setSelectedCharacter(null);
        setScreen('lobby');
      });
      invoke('show_main_window').catch(console.error);
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  // ── 핸들러 ────────────────────────────────────────────────────────────────

  const handleUpdate = async () => {
    setUpdateState('downloading');
    try {
      const update = await check();
      if (update?.available) {
        await update.downloadAndInstall();
        setUpdateState('done');
      }
    } catch {
      setUpdateState('idle');
    }
  };

  const handleJoin = (c: Omit<RoomConfig, 'deviceId' | 'joinedAt'>) => {
    setLobbyError(null);
    const full: RoomConfig = { ...c, deviceId, joinedAt: Date.now() };
    setConfig(full);
    setScreen('character-select');
  };

  const handleCharacterConfirm = async (character: Character) => {
    const newUsers = await joinWithCharacter(character);
    if (!newUsers) return;
    setSelectedCharacter(character);
    const positions = newUsers.map((_u, i) => getWindowPosition(i, newUsers.length));
    const overlayAvailable = await invoke<boolean>('sync_char_windows', {
      userIds: newUsers.map((u: UserState) => u.userId),
      positions,
      usersJson: JSON.stringify(newUsers),
    }).catch(() => false);
    setScreen(overlayAvailable ? 'overlay' : 'chat');
    if (overlayAvailable) invoke('enter_overlay').catch(() => {});
  };

  const handleCharacterBack = () => {
    setConfig(null);
    setScreen('lobby');
  };

  const handleLeave = async () => {
    clearSession();
    setSelectedCharacter(null);
    setConfig(null);
    setScreen('lobby');
    await invoke('leave_overlay').catch(console.error);
  };

  const handleBackToOverlay = async () => {
    setScreen('overlay');
    await invoke('enter_overlay').catch(console.error);
  };

  // ── 렌더 ──────────────────────────────────────────────────────────────────

  return (
    <div className="app">
      {screen === 'splash' && <SplashScreen />}
      {screen === 'lobby' && <LobbyScreen onJoin={handleJoin} errorMessage={lobbyError} />}
      {screen === 'character-select' && config && (
        <CharacterSelectScreen
          roomCode={config.roomCode}
          takenCharacters={takenCharacters}
          characterError={characterError}
          onClearError={clearCharacterError}
          onConfirm={handleCharacterConfirm}
          onBack={handleCharacterBack}
          isConnected={isConnected}
        />
      )}
      {screen === 'lobby' && updateInfo && (
        <div className="update-banner">
          {updateState === 'done' ? (
            <span className="update-banner-text">✓ 설치 완료 — 앱을 재시작해주세요</span>
          ) : (
            <>
              <span className="update-banner-text">새 버전 {updateInfo.version} 출시!</span>
              <button
                className="update-banner-btn"
                onClick={handleUpdate}
                disabled={updateState === 'downloading'}
              >
                {updateState === 'downloading' ? '다운로드 중...' : '지금 업데이트'}
              </button>
              <button className="update-banner-dismiss" onClick={() => setUpdateInfo(null)}>✕</button>
            </>
          )}
        </div>
      )}
      {screen === 'chat' && config && (
        <ChatScreen
          config={config}
          users={users}
          messages={messages}
          onSendChat={sendChat}
          onSendActivity={sendActivity}
          onLeave={handleLeave}
          onBackToOverlay={handleBackToOverlay}
        />
      )}
    </div>
  );
}

export default App;
