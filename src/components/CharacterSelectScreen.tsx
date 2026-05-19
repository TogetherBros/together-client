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
  roomCode: string;
  takenCharacters: Character[];
  characterError?: string | null;
  onClearError?: () => void;
  onConfirm: (character: Character) => void;
  onBack: () => void;
  isConnected?: boolean;
}

export default function CharacterSelectScreen({ roomCode, takenCharacters, characterError, onClearError, onConfirm, onBack, isConnected = true }: Props) {
  const [selected, setSelected] = useState<Character | null>(null);
  const [serverTaken, setServerTaken] = useState<Set<Character>>(new Set());
  const [confirming, setConfirming] = useState(false);

  // 진입 시 서버에서 현재 선택된 캐릭터 목록 조회
  useEffect(() => {
    fetch(`https://api.togetherbros.uk/api/room/${roomCode}/characters`)
      .then(r => r.json())
      .then((chars: Character[]) => setServerTaken(new Set(chars)))
      .catch(() => {});
  }, [roomCode]);

  const takenSet = new Set([...takenCharacters, ...serverTaken]);

  // 선택한 캐릭터가 다른 유저에게 선점되면 선택 해제
  useEffect(() => {
    if (selected && takenSet.has(selected)) {
      setSelected(null);
    }
  }, [takenCharacters, serverTaken]);

  // 에러 발생 시 confirming 해제
  useEffect(() => {
    if (characterError) setConfirming(false);
  }, [characterError]);

  const handleSelect = (type: Character) => {
    if (takenSet.has(type)) return;
    setSelected(type);
    onClearError?.();
  };

  const handleConfirm = async () => {
    if (!selected || confirming) return;
    setConfirming(true);
    try {
      await onConfirm(selected);
    } finally {
      setConfirming(false);
    }
  };

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
                onClick={() => handleSelect(type)}
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

        {characterError && (
          <p className="char-error-msg">{characterError}</p>
        )}

        <div className="char-select-actions">
          <button className="char-back-btn" onClick={onBack} disabled={confirming}>뒤로</button>
          <button
            className="join-btn char-confirm-btn"
            onClick={handleConfirm}
            disabled={!selected || confirming || !isConnected}
          >
            {confirming ? '입장 중...' : !isConnected ? '연결 중...' : '입장하기'}
          </button>
        </div>
      </div>
    </div>
  );
}
