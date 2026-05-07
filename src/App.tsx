import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, emitTo } from '@tauri-apps/api/event';
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

  // 서버 에러 → 로비
  useEffect(() => {
    if (!error) return;
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

  const handleJoin = async (c: Omit<RoomConfig, 'deviceId' | 'joinedAt'>) => {
    const full: RoomConfig = { ...c, deviceId, joinedAt: Date.now() };
    setConfig(full);
    setScreen('overlay');
    await invoke('enter_overlay').catch(console.error);
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

  // 트레이 "열기" → 채팅 화면
  useEffect(() => {
    const unlisten = listen('open-chat', () => {
      const s = screenRef.current;
      if (s === 'overlay' || s === 'chat') setScreen('chat');
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  // 트레이 "방 나가기"
  useEffect(() => {
    const unlisten = listen('leave-overlay', () => {
      setConfig(null);
      setScreen('lobby');
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  return (
    <div className="app">
      {screen === 'splash' && <SplashScreen />}
      {screen === 'lobby' && <LobbyScreen onJoin={handleJoin} />}
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
