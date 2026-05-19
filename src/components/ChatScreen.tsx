import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { UserState, ChatMessage, RoomConfig, GameState, GameType } from '../types';
import { characterImages } from '../characters';
import MiniGameSheet from './MiniGameSheet';

interface Props {
  config: RoomConfig;
  users: UserState[];
  messages: ChatMessage[];
  gameState: GameState | null;
  onSendChat: (text: string) => void;
  onSendActivity: (activity: string) => void;
  onLeave: () => void;
  onBackToOverlay: () => void;
  onStartGame: (type: GameType) => void;
  onJoinGame: (gameId: string) => void;
  onSubmitWord: (word: string) => void;
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

function useTurnTimer(deadline: string | undefined) {
  const [remaining, setRemaining] = useState(0);
  useEffect(() => {
    if (!deadline) { setRemaining(0); return; }
    const tick = () => setRemaining(Math.max(0, Math.ceil((new Date(deadline).getTime() - Date.now()) / 1000)));
    tick();
    const id = setInterval(tick, 500);
    return () => clearInterval(id);
  }, [deadline]);
  return remaining;
}

function GameStatusBar({ gameState, myUserId, onJoin }: { gameState: GameState; myUserId: string; onJoin: (id: string) => void }) {
  const remaining = useTurnTimer(gameState.status === 'playing' ? gameState.turnDeadline : undefined);
  const isMyTurn = gameState.currentTurnUserId === myUserId;
  const amParticipant = gameState.participants.includes(myUserId);

  if (gameState.status === 'waiting') {
    return (
      <div className="game-status-bar game-status-waiting">
        <span className="game-status-icon">🔤</span>
        <span className="game-status-text">끝말잇기 대기 중… ({gameState.participants.length}명 참여)</span>
        {!amParticipant && (
          <button className="game-join-btn" onClick={() => onJoin(gameState.gameId)}>참여하기</button>
        )}
      </div>
    );
  }

  return (
    <div className={`game-status-bar${isMyTurn ? ' game-status-myturn' : ''}`}>
      <span className="game-status-icon">🔤</span>
      <div className="game-status-info">
        <span className="game-status-turn">
          {isMyTurn ? '내 차례!' : `${gameState.currentTurnNickname} 차례`}
        </span>
        <span className="game-status-word">
          <span className="game-next-char">「{gameState.nextChar}」</span>로 시작하는 단어
        </span>
      </div>
      <span className={`game-timer${remaining <= 5 ? ' game-timer-urgent' : ''}`}>{remaining}초</span>
    </div>
  );
}

function GameInviteCard({ msg, myUserId, onJoin }: { msg: ChatMessage; myUserId: string; onJoin: (id: string) => void }) {
  const gameId = msg.gameData?.gameId ?? '';
  return (
    <div className="game-invite-card">
      <span className="game-invite-icon">🎮</span>
      <div className="game-invite-body">
        <span className="game-invite-title">끝말잇기 시작!</span>
        <span className="game-invite-sub">{msg.nickname}님이 게임을 시작했어요</span>
      </div>
      {msg.userId !== myUserId && (
        <button className="game-join-btn" onClick={() => onJoin(gameId)}>참여하기</button>
      )}
    </div>
  );
}

function GameWordMsg({ msg }: { msg: ChatMessage }) {
  return (
    <div className="game-word-msg">
      <span className="game-word-nick">{msg.gameData?.submitterNickname}</span>
      <span className="game-word-text">{msg.gameData?.word}</span>
      {msg.gameData?.nextChar && (
        <span className="game-word-next">→ 「{msg.gameData.nextChar}」</span>
      )}
    </div>
  );
}

function GameOverCard({ msg }: { msg: ChatMessage }) {
  const reasonText: Record<string, string> = {
    WRONG_WORD: '틀린 단어',
    TIMEOUT: '시간 초과',
    DUPLICATE: '중복 단어',
    NO_WORD: '단어 없음',
  };
  const reason = msg.gameData?.reason ? reasonText[msg.gameData.reason] ?? '' : '';
  return (
    <div className="game-over-card">
      <span className="game-over-icon">🏆</span>
      <div className="game-over-body">
        <span className="game-over-title">게임 종료</span>
        {msg.gameData?.winnerNickname && (
          <span className="game-over-winner">{msg.gameData.winnerNickname}님 우승!</span>
        )}
        {msg.gameData?.loserNickname && reason && (
          <span className="game-over-reason">{msg.gameData.loserNickname} — {reason}</span>
        )}
      </div>
    </div>
  );
}

export default function ChatScreen({
  config,
  users,
  messages,
  gameState,
  onSendChat,
  onSendActivity,
  onLeave,
  onBackToOverlay,
  onStartGame,
  onJoinGame,
  onSubmitWord,
}: Props) {
  const [input, setInput] = useState('');
  const [confirmLeave, setConfirmLeave] = useState(false);
  const [showGameSheet, setShowGameSheet] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const elapsed = useElapsed(config.joinedAt);

  const isMyGameTurn = gameState?.status === 'playing' && gameState.currentTurnUserId === config.deviceId;

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
    if (isMyGameTurn) {
      onSubmitWord(text);
    } else {
      onSendChat(text);
    }
    setInput('');
  };

  const inputPlaceholder = isMyGameTurn
    ? `「${gameState!.nextChar}」로 시작하는 단어를 입력하세요`
    : '메시지를 입력하세요...';

  return (
    <div className="chat-screen">
      {showGameSheet && (
        <MiniGameSheet onSelect={onStartGame} onClose={() => setShowGameSheet(false)} />
      )}

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
          <button
            className="chat-game-btn"
            onClick={() => setShowGameSheet(v => !v)}
            title="미니게임"
            aria-label="미니게임"
          >🎮</button>
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

      {gameState && (
        <GameStatusBar gameState={gameState} myUserId={config.deviceId} onJoin={onJoinGame} />
      )}

      <div className="chat-messages">
        {messages.length === 0 && (
          <p className="chat-empty">아직 메시지가 없어요. 먼저 인사해보세요!</p>
        )}
        {messages.map((msg, i) => {
          if (msg.msgType === 'game-invite') {
            return <GameInviteCard key={`gi-${i}`} msg={msg} myUserId={config.deviceId} onJoin={onJoinGame} />;
          }
          if (msg.msgType === 'game-word') {
            return <GameWordMsg key={`gw-${i}`} msg={msg} />;
          }
          if (msg.msgType === 'game-over') {
            return <GameOverCard key={`go-${i}`} msg={msg} />;
          }

          const isMine = msg.userId === config.deviceId;
          return (
            <div
              key={`${msg.userId}-${msg.sentAt}-${i}`}
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

      <div className={`chat-input-row${isMyGameTurn ? ' chat-input-game' : ''}`} onClick={() => inputRef.current?.focus()}>
        <input
          ref={inputRef}
          className="chat-input"
          type="text"
          placeholder={inputPlaceholder}
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
          {isMyGameTurn ? '제출' : '전송'}
        </button>
      </div>
    </div>
  );
}
