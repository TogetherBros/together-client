import { useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { RoomConfig } from '../types';
import { logoImg } from '../characters';

function generateRoomCode(): string {
  const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
  return Array.from({ length: 6 }, () => chars[Math.floor(Math.random() * chars.length)]).join('');
}

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
  onJoin: (config: Omit<RoomConfig, 'deviceId' | 'joinedAt'>) => void;
}

export default function LobbyScreen({ onJoin }: Props) {
  const [nickname, setNickname] = useState('');
  const [roomCode, setRoomCode] = useState('');

  const canJoin = nickname.trim().length > 0 && roomCode.trim().length === 6;

  const handleJoin = () => {
    if (!canJoin) return;
    onJoin({ nickname: nickname.trim(), roomCode: roomCode.trim().toUpperCase() });
  };

  const handleGenerate = () => setRoomCode(generateRoomCode());

  return (
    <div className="lobby">
      <div className="titlebar" data-tauri-drag-region>
        <TitlebarButtons />
      </div>
      <div className="lobby-card">
        <div className="lobby-header">
          <img src={logoImg} alt="Together" className="logo-img" />
          <h1 className="lobby-title">Together</h1>
          <p className="lobby-subtitle">친구들과 함께, 바탕화면 위에서</p>
        </div>

        <div className="form-group">
          <label className="form-label">닉네임</label>
          <input
            className="form-input"
            type="text"
            maxLength={8}
            placeholder="최대 8자"
            value={nickname}
            onChange={e => setNickname(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleJoin()}
          />
        </div>

        <div className="form-group">
          <label className="form-label">방 코드</label>
          <div className="room-code-row">
            <input
              className="form-input room-code-input"
              type="text"
              maxLength={6}
              placeholder="6자리 코드"
              value={roomCode}
              onChange={e => setRoomCode(e.target.value.toUpperCase())}
              onKeyDown={e => e.key === 'Enter' && handleJoin()}
            />
            <button className="generate-btn" onClick={handleGenerate}>
              새 방 만들기
            </button>
          </div>
        </div>

        <button className="join-btn" onClick={handleJoin} disabled={!canJoin}>
          입장하기
        </button>
      </div>
    </div>
  );
}
