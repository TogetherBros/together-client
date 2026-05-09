import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { UserState, ChatMessage, RoomConfig } from '../types';
import { characterImages } from '../characters';

interface Props {
  config: RoomConfig;
  users: UserState[];
  messages: ChatMessage[];
  onSendChat: (text: string) => void;
  onSendActivity: (activity: string) => void;
  onLeave: () => void;
  onBackToOverlay: () => void;
}

function formatTime(iso: string): string {
  const d = new Date(iso);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

function formatElapsed(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  if (h > 0) return `${h}시간 ${m}분 경과`;
  if (m > 0) return `${m}분 ${s}초 경과`;
  return `${s}초 경과`;
}

function useElapsed(joinedAt: number) {
  const [elapsed, setElapsed] = useState(() => Date.now() - joinedAt);
  useEffect(() => {
    const id = setInterval(() => setElapsed(Date.now() - joinedAt), 1000);
    return () => clearInterval(id);
  }, [joinedAt]);
  return elapsed;
}

export default function ChatScreen({
  config,
  users,
  messages,
  onSendChat,
  onSendActivity,
  onLeave,
  onBackToOverlay,
}: Props) {
  const [input, setInput] = useState('');
  const [confirmLeave, setConfirmLeave] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);
  const elapsed = useElapsed(config.joinedAt);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const text = e.target.value;
    setInput(text);
    onSendActivity(text.trim() ? '입력 중...' : '');
  };

  const handleSend = () => {
    const text = input.trim();
    if (!text) return;
    onSendActivity('');
    onSendChat(text);
    setInput('');
  };

  return (
    <div className="chat-screen">
      <div className="chat-header" data-tauri-drag-region>
        <div className="chat-header-left">
          <div className="titlebar-buttons">
            <button
              className="titlebar-btn titlebar-minimize"
              onClick={onBackToOverlay}
              title="창 닫기"
              aria-label="창 닫기"
            />
            <button
              className="titlebar-btn titlebar-close"
              onClick={() => invoke('quit_app').catch(console.error)}
              title="서비스 종료"
              aria-label="서비스 종료"
            />
          </div>
          <div className="chat-header-divider" />
          <span className="chat-room-label">방</span>
          <span className="chat-room-code">{config.roomCode}</span>
          <span className="chat-elapsed">{formatElapsed(elapsed)}</span>
        </div>

        <div className="chat-header-right">
          {confirmLeave ? (
            <div className="chat-leave-confirm">
              <span className="chat-leave-confirm-text">나가시겠어요?</span>
              <button className="chat-leave-confirm-yes" onClick={onLeave}>나가기</button>
              <button className="chat-leave-confirm-no" onClick={() => setConfirmLeave(false)}>취소</button>
            </div>
          ) : (
            <button className="chat-leave-btn" onClick={() => setConfirmLeave(true)}>방 나가기</button>
          )}
        </div>
      </div>

      <div className="chat-users">
        <span className="chat-users-label">온라인 {users.length}명</span>
        <div className="chat-users-list">
          {users.map(u => (
            <div
              key={u.userId}
              className={`chat-user-chip${u.userId === config.deviceId ? ' chat-user-chip-me' : ''}`}
            >
              <img src={characterImages[u.character]} alt={u.character} className="chat-user-img" />
              <span className="chat-user-name">{u.nickname}</span>
              {u.userId === config.deviceId && <span className="chat-me-tag">나</span>}
            </div>
          ))}
        </div>
      </div>

      <div className="chat-messages">
        {messages.length === 0 && (
          <p className="chat-empty">아직 메시지가 없어요. 먼저 인사해보세요!</p>
        )}
        {messages.map((msg, i) => {
          const isMine = msg.userId === config.deviceId;
          return (
            <div
              key={i}
              className={`chat-msg ${isMine ? 'chat-msg-mine' : 'chat-msg-other'}`}
            >
              {!isMine && (
                <img
                  src={characterImages[msg.character]}
                  alt={msg.character}
                  className="chat-msg-avatar"
                />
              )}
              <div className="chat-msg-body">
                {!isMine && <span className="chat-msg-nick">{msg.nickname}</span>}
                <div className="chat-msg-bubble">{msg.message}</div>
                <span className="chat-msg-time">{formatTime(msg.sentAt)}</span>
              </div>
            </div>
          );
        })}
        <div ref={bottomRef} />
      </div>

      <div className="chat-input-row">
        <input
          className="chat-input"
          type="text"
          placeholder="메시지를 입력하세요..."
          maxLength={200}
          value={input}
          onChange={handleInputChange}
          onKeyDown={e => e.key === 'Enter' && handleSend()}
        />
        <button
          className="chat-send-btn"
          onClick={handleSend}
          disabled={!input.trim()}
        >
          전송
        </button>
      </div>
    </div>
  );
}
