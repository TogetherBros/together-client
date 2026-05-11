import { useState, useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { Character } from '../types';
import { characterImages, characterMeta } from '../characters';

function TitlebarButtons() {
  const win = getCurrentWindow();
  return (
    <div className="titlebar-buttons">
      <button
        className="titlebar-btn titlebar-minimize"
        onClick={() => win.hide()}
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
  );
}

interface Props {
  takenCharacters: Character[];
  onConfirm: (character: Character) => void;
  onBack: () => void;
}

export default function CharacterSelectScreen({ takenCharacters, onConfirm, onBack }: Props) {
  const [selected, setSelected] = useState<Character | null>(null);
  const takenSet = new Set(takenCharacters);

  // 선택한 캐릭터가 입장 중에 다른 유저에게 선점되면 선택 해제
  useEffect(() => {
    if (selected && takenSet.has(selected)) {
      setSelected(null);
    }
  }, [takenCharacters]);

  return (
    <div className="lobby">
      <div className="titlebar" data-tauri-drag-region>
        <TitlebarButtons />
      </div>
      <div className="lobby-card char-select-card">
        <div className="char-select-header">
          <h2 className="char-select-title">캐릭터 선택</h2>
          <p className="char-select-subtitle">사용할 캐릭터를 골라주세요</p>
        </div>

        <div className="character-grid char-select-grid">
          {characterMeta.map(({ type, label }) => {
            const isTaken = takenSet.has(type);
            const isSelected = selected === type;
            return (
              <button
                key={type}
                className={`character-btn${isSelected ? ' selected' : ''}${isTaken ? ' taken' : ''}`}
                onClick={() => { if (!isTaken) setSelected(type); }}
                disabled={isTaken}
                title={isTaken ? '이미 사용 중인 캐릭터입니다' : label}
              >
                <img
                  src={characterImages[type]}
                  alt={label}
                  className="char-select-img"
                  draggable={false}
                />
                <span className="char-select-label">{label}</span>
                {isTaken && <span className="char-taken-badge">사용 중</span>}
              </button>
            );
          })}
        </div>

        <div className="char-select-actions">
          <button className="char-back-btn" onClick={onBack}>뒤로</button>
          <button
            className="join-btn char-confirm-btn"
            onClick={() => { if (selected) onConfirm(selected); }}
            disabled={!selected}
          >
            입장하기
          </button>
        </div>
      </div>
    </div>
  );
}
