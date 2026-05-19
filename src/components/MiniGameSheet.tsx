import { GameType } from '../types';

interface Props {
  onSelect: (type: GameType) => void;
  onClose: () => void;
}

const GAMES: { type: GameType; label: string; desc: string; emoji: string }[] = [
  { type: 'WORD_CHAIN', label: '끝말잇기', desc: '마지막 글자로 이어가기', emoji: '🔤' },
];

export default function MiniGameSheet({ onSelect, onClose }: Props) {
  return (
    <div className="game-sheet-backdrop" onClick={onClose}>
      <div className="game-sheet" onClick={e => e.stopPropagation()}>
        <div className="game-sheet-handle" />
        <p className="game-sheet-title">미니게임</p>
        <div className="game-sheet-list">
          {GAMES.map(g => (
            <button
              key={g.type}
              className="game-sheet-card"
              onClick={() => { onSelect(g.type); onClose(); }}
            >
              <span className="game-sheet-emoji">{g.emoji}</span>
              <div className="game-sheet-info">
                <span className="game-sheet-label">{g.label}</span>
                <span className="game-sheet-desc">{g.desc}</span>
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
