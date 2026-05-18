import { useEffect, useRef, useState } from 'react';
import { listen, emit } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { UserState } from '../types';
import { characterImages } from '../characters';

const SIZE_SCALE: Record<number, number> = { 1: 0.7, 2: 1.0, 3: 1.35 };

export default function CharacterOverlay() {
  const win = getCurrentWindow();
  // initialization_script로 주입된 유저 데이터에서 userId 우선 사용
  const injectedUser = (window as any).__TAURI_CHAR_USER__ as { userId?: string } | undefined;
  const userId = injectedUser?.userId ?? win.label.replace(/^char_/, '');

  const [user, setUser] = useState<UserState | null>(null);
  const [bubble, setBubble] = useState<string>('');
  const [characterSize, setCharacterSize] = useState<number>(2);
  const [dragEnabled, setDragEnabled] = useState<boolean>(true);

  // userId ref so async callbacks always read current value
  const userIdRef = useRef(userId);

  // Register all listeners first, then emit char-ready to avoid race condition
  const userFoundRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    const unlisteners: (() => void)[] = [];
    let retryTimer: ReturnType<typeof setTimeout> | null = null;

    console.log('[CharOverlay] useEffect 시작, label:', win.label);

    // 1순위: initialization_script로 주입된 데이터 (\uXXXX 이스케이프 → 순수 ASCII)
    const injected = (window as any).__TAURI_CHAR_USER__;
    if (injected && injected.userId) {
      console.log('[CharOverlay] init-script 유저 발견:', injected.userId);
      userFoundRef.current = true;
      setUser(injected as UserState);
    }

    // 2순위: URL 쿼리 파라미터 (?u=...) 폴백
    if (!userFoundRef.current) {
      const uParam = new URLSearchParams(window.location.search).get('u');
      if (uParam) {
        try {
          const me = JSON.parse(uParam) as UserState;
          if (me?.userId) {
            console.log('[CharOverlay] URL 파라미터 유저 발견:', me.userId);
            userFoundRef.current = true;
            setUser(me);
          }
        } catch (e) {
          console.error('[CharOverlay] URL 파라미터 파싱 실패:', e);
        }
      }
    }

    const fetchAndSetUser = async () => {
      try {
        const winLabel = win.label;
        const json = await invoke<string>('get_user_for_window', { label: winLabel });
        console.log('[CharOverlay] get_user_for_window label:', winLabel, '| 결과:', json);
        if (cancelled) return;
        if (json && json !== 'null') {
          const me = JSON.parse(json) as UserState;
          userFoundRef.current = true;
          setUser(me);
        }
      } catch (e) {
        console.error('[CharOverlay] get_user_for_window 실패:', e);
      }
    };

    const setup = async () => {
      await fetchAndSetUser();

      const unsubUsers = await listen<UserState[]>('users-updated', e => {
        if (cancelled) return;
        const me = e.payload.find(u => u.userId === userIdRef.current);
        if (me) {
          userFoundRef.current = true;
          setUser(me);
        }
      });

      const unsubBubble = await listen<Record<string, string>>('bubble-updated', e => {
        if (cancelled) return;
        setBubble(e.payload[userIdRef.current] ?? '');
      });

      const unsubSize = await listen<number>('character-size-changed', e => {
        if (cancelled) return;
        setCharacterSize(e.payload);
      });

      const unsubDrag = await listen<boolean>('drag-enabled', e => {
        if (cancelled) return;
        setDragEnabled(e.payload);
      });

      unlisteners.push(unsubUsers, unsubBubble, unsubSize, unsubDrag);

      // All listeners registered — now signal main window to send initial state
      if (!cancelled) {
        await emit('char-ready', { userId: userIdRef.current });
      }

      // Retry every 800ms until user data arrives (handles missed events, max 10 times)
      let retryCount = 0;
      const retry = async () => {
        if (cancelled || userFoundRef.current || retryCount >= 10) return;
        retryCount++;
        await fetchAndSetUser();
        if (!cancelled && !userFoundRef.current) {
          await emit('char-ready', { userId: userIdRef.current }).catch(() => {});
          retryTimer = setTimeout(retry, 800);
        }
      };
      retryTimer = setTimeout(retry, 800);
    };

    setup().catch(console.error);

    return () => {
      cancelled = true;
      if (retryTimer) clearTimeout(retryTimer);
      unlisteners.forEach(f => f());
    };
  }, []);

  const handleMouseDown = (e: React.MouseEvent) => {
    if (!dragEnabled || e.button !== 0) return;
    e.preventDefault();
    win.startDragging().catch(console.error);
  };

  if (!user) {
    return (
      <div style={{
        position: 'fixed', inset: 0,
        background: 'rgba(255,0,0,0.5)',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        color: 'white', fontSize: '12px', fontWeight: 'bold',
        pointerEvents: 'none',
        zIndex: 9999,
      }}>NO DATA</div>
    );
  }

  const scale = SIZE_SCALE[characterSize] ?? 1.0;
  const isTyping = !bubble && user.activity === '입력 중...';
  const displayText = bubble || user.activity;
  const showBubble = !!bubble || user.activity !== '';

  return (
    <div className="char-window">
      <div style={{ transform: `scale(${scale})`, transformOrigin: 'bottom center' }}>
        {/* pointer-events: auto로 char-content만 마우스 이벤트 수신 (투명 영역은 클릭 투과) */}
        <div
          className="char-content"
          onMouseDown={handleMouseDown}
          style={{ cursor: dragEnabled ? 'grab' : 'default', pointerEvents: 'auto' }}
        >
          {showBubble && (
            <div className={`speech-bubble${bubble ? ' speech-bubble-chat' : ''}${isTyping ? ' speech-bubble-typing' : ''}`}>
              {isTyping ? (
                <span className="typing-dots"><span /><span /><span /></span>
              ) : (
                <span>{displayText}</span>
              )}
            </div>
          )}
          <img
            src={characterImages[user.character]}
            alt={user.character}
            className="char-overlay-img"
            draggable={false}
          />
          <div className="character-nickname">
            {user.nickname}
          </div>
        </div>
      </div>
    </div>
  );
}
