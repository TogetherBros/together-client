import { useEffect, useState } from 'react';
import { listen, emitTo } from '@tauri-apps/api/event';
import { Character } from '../types';
import { characterImages } from '../characters';

export interface CharUpdatePayload {
  userId: string;
  nickname: string;
  character: Character;
  isMe: boolean;
  activity: string;
  bubbleMessage: string | null;
}

interface CharState {
  nickname: string;
  character: Character;
  isMe: boolean;
  activity: string;
  bubbleMessage: string | null;
}

export default function CharacterOverlay() {
  const userId = (window as any).__CHAR_USER_ID__ as string;
  const [state, setState] = useState<CharState | null>(null);

  useEffect(() => {
    const unlisten = listen<CharUpdatePayload>('char-update', ({ payload }) => {
      if (payload.userId !== userId) return;
      const { userId: _id, ...rest } = payload;
      setState(rest);
    });

    emitTo('main', 'char-ready', { userId }).catch(console.error);

    return () => { unlisten.then(f => f()); };
  }, [userId]);

  if (!state) return null;

  const { nickname, character, isMe, activity, bubbleMessage } = state;
  const isTyping = !bubbleMessage && activity === '입력 중...';
  const displayText = bubbleMessage ?? activity;
  const showBubble = !!bubbleMessage || activity !== '';

  return (
    <div
      className={`char-overlay-root${isMe ? ' char-overlay-me' : ''}`}
      data-tauri-drag-region
    >
      {showBubble && (
        <div className={`speech-bubble${bubbleMessage ? ' speech-bubble-chat' : ''}${isTyping ? ' speech-bubble-typing' : ''}`}>
          {isTyping ? (
            <span className="typing-dots"><span /><span /><span /></span>
          ) : (
            <span>{displayText}</span>
          )}
        </div>
      )}
      <img
        src={characterImages[character]}
        alt={character}
        className="char-overlay-img"
        draggable={false}
      />
      <div className="character-nickname">
        {isMe && <span className="me-badge">나</span>}
        {nickname}
      </div>
    </div>
  );
}
