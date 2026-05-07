import { UserState } from '../types';
import { characterImages } from '../characters';
import { getCharacterPosition } from '../utils/position';

interface Props {
  user: UserState;
  index: number;
  total: number;
  bubbleMessage?: string;
}

export default function CharacterCard({ user, index, total, bubbleMessage }: Props) {
  const { x, y } = getCharacterPosition(index, total);

  const isTyping = user.activity === '입력 중...';
  const displayText = bubbleMessage ?? user.activity;
  const showBubble = !!bubbleMessage || user.activity !== '';

  return (
    <div
      className="character-pos"
      style={{ transform: `translate(${x}px, ${y}px)` }}
    >
      <div className="character-card" style={{ animationDelay: `${index * 0.4}s` }}>
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
          src={characterImages[user.character]}
          alt={user.character}
          className="character-card-img"
          draggable={false}
        />
        <div className="character-nickname">{user.nickname}</div>
      </div>
    </div>
  );
}
