import { useState, useEffect, useRef } from 'react';
import { flushSync } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import { listen, emitTo } from '@tauri-apps/api/event';
import { check } from '@tauri-apps/plugin-updater';
import { AppScreen, RoomConfig, UserState } from './types';
import { useRoom } from './hooks/useRoom';
import SplashScreen from './components/SplashScreen';
import LobbyScreen from './components/LobbyScreen';
import ChatScreen from './components/ChatScreen';
import './App.css';

function getDeviceId(): string {
  const key = 'together_device_id';
  const stored = localStorage.getItem(key);
  if (stored) return stored;
  const id = crypto.randomUUID();
  localStorage.setItem(key, id);
  return id;
}

function App() {
  const [screen, setScreen] = useState<AppScreen>('splash');
  const [config, setConfig] = useState<RoomConfig | null>(null);
  const [deviceId] = useState<string>(getDeviceId);
  const [updateInfo, setUpdateInfo] = useState<{ version: string } | null>(null);
  const [updateState, setUpdateState] = useState<'idle' | 'downloading' | 'done'>('idle');
  const [lobbyError, setLobbyError] = useState<string | null>(null);
  const screenRef = useRef(screen);
  screenRef.current = screen;

  const { users, messages, bubbleMessages, error, activityRef, sendChat, sendActivity } =
    useRoom(config);

  const usersRef = useRef<UserState[]>(users);
  const bubbleRef = useRef<Record<string, string>>(bubbleMessages);
  usersRef.current = users;
  bubbleRef.current = bubbleMessages;

  // 스플래시 타임아웃
  useEffect(() => {
    const t = setTimeout(() => setScreen('lobby'), 1800);
    return () => clearTimeout(t);
  }, []);

  // 업데이트 확인 (로비 진입 시 1회)
  const updateChecked = useRef(false);
  useEffect(() => {
    if (screen !== 'lobby' || updateChecked.current) return;
    updateChecked.current = true;
    check().then(update => {
      if (update?.available) setUpdateInfo({ version: update.version });
    }).catch(() => {});
  }, [screen]);

  // 서버 에러 → 로비
  useEffect(() => {
    if (!error) return;
    setLobbyError(error);
    setConfig(null);
    setScreen('lobby');
    invoke('leave_overlay').catch(console.error);
  }, [error]);

  // 앱 감지 (3초마다)
  useEffect(() => {
    if (!config) return;
    const poll = async () => {
      if (activityRef.current === '입력 중...') return; // 타이핑 중엔 덮어쓰지 않음
      try {
        const app = await invoke<string>('get_active_app');
        activityRef.current = app;
      } catch {}
    };
    poll();
    const interval = setInterval(poll, 3000);
    return () => clearInterval(interval);
  }, [config, activityRef]);

  // overlay로 유저 목록 전송
  useEffect(() => {
    if (!config || screen === 'splash' || screen === 'lobby') return;
    emitTo('overlay', 'users-updated', users).catch(console.error);
  }, [users, config, screen]);

  // overlay로 말풍선 전송
  useEffect(() => {
    if (!config || screen === 'splash' || screen === 'lobby') return;
    emitTo('overlay', 'bubble-updated', bubbleMessages).catch(console.error);
  }, [bubbleMessages, config, screen]);

  // overlay 준비 완료 → 최신 상태 즉시 전송 (항상 활성)
  useEffect(() => {
    const unlisten = listen('overlay-ready', () => {
      const s = screenRef.current;
      if (s === 'splash' || s === 'lobby') return;
      emitTo('overlay', 'users-updated', usersRef.current).catch(console.error);
      emitTo('overlay', 'bubble-updated', bubbleRef.current).catch(console.error);
    });
    return () => { unlisten.then(f => f()); };
  }, [deviceId]);

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

  const handleJoin = async (c: Omit<RoomConfig, 'deviceId' | 'joinedAt'>) => {
    setLobbyError(null);
    const full: RoomConfig = { ...c, deviceId, joinedAt: Date.now() };
    setConfig(full);
    const overlayAvailable = await invoke<boolean>('enter_overlay').catch(() => false);
    // Linux 등 오버레이 미지원 환경은 채팅 화면으로 대체
    setScreen(overlayAvailable ? 'overlay' : 'chat');
  };

  const handleLeave = async () => {
    setConfig(null);
    setScreen('lobby');
    await invoke('leave_overlay').catch(console.error);
  };

  const handleBackToOverlay = async () => {
    setScreen('overlay');
    await invoke('enter_overlay').catch(console.error);
  };

  // 트레이 "열기" / 앱 재실행 / macOS 독 클릭 → 상태에 맞는 화면으로
  useEffect(() => {
    const unlisten = listen('open-chat', () => {
      const s = screenRef.current;
      flushSync(() => {
        // 방에 참여 중이면 채팅 화면, 아니면 현재 화면 유지
        if (s === 'overlay' || s === 'chat') setScreen('chat');
        else if (s === 'splash') setScreen('lobby');
      });
      invoke('show_main_window').catch(console.error);
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  // 트레이 "방 나가기"
  useEffect(() => {
    const unlisten = listen('leave-overlay', () => {
      flushSync(() => {
        setConfig(null);
        setScreen('lobby');
      });
      invoke('show_main_window').catch(console.error);
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  return (
    <div className="app">
      {screen === 'splash' && <SplashScreen />}
      {screen === 'lobby' && <LobbyScreen onJoin={handleJoin} errorMessage={lobbyError} />}
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
