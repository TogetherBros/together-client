import { useEffect, useRef, useState } from 'react';
import { listen, emitTo } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { UserState } from './types';
import { getCharacterPosition } from './utils/position';
import CharacterCard from './components/CharacterCard';

type Position = { x: number; y: number };

const SIZE_SCALE: Record<number, number> = { 1: 0.7, 2: 1.0, 3: 1.35 };

function loadPositions(): Record<string, Position> {
  try { return JSON.parse(sessionStorage.getItem('together-positions') || '{}'); }
  catch { return {}; }
}

function savePositions(pos: Record<string, Position>) {
  sessionStorage.setItem('together-positions', JSON.stringify(pos));
}

export default function OverlayApp() {
  const [users, setUsers] = useState<UserState[]>([]);
  const [bubbleMessages, setBubbleMessages] = useState<Record<string, string>>({});
  const [positions, setPositions] = useState<Record<string, Position>>(loadPositions);
  const [characterSize, setCharacterSize] = useState<number>(2);
  const [hiddenUsers, setHiddenUsers] = useState<Set<string>>(new Set());

  const usersRef = useRef<UserState[]>([]);
  const positionsRef = useRef<Record<string, Position>>({});
  const hiddenUsersRef = useRef<Set<string>>(new Set());

  useEffect(() => { usersRef.current = users; }, [users]);
  useEffect(() => { positionsRef.current = positions; }, [positions]);
  useEffect(() => { hiddenUsersRef.current = hiddenUsers; }, [hiddenUsers]);

  // Report zone coords to Rust every 50ms
  useEffect(() => {
    const id = setInterval(() => {
      const us = usersRef.current.filter(u => !hiddenUsersRef.current.has(u.userId));
      if (us.length === 0) return;
      const dpr = window.devicePixelRatio || 1;
      const ox = Math.round(window.screenX * dpr);
      const oy = Math.round(window.screenY * dpr);
      const zones = us.map((u, i) => {
        const p = positionsRef.current[u.userId] ?? getCharacterPosition(i, us.length);
        return [
          ox + Math.round(p.x * dpr),
          oy + Math.round(p.y * dpr),
          Math.round(100 * dpr),
          Math.round(160 * dpr),
        ];
      });
      invoke('update_character_zones', { zones, userIds: us.map(u => u.userId) }).catch(() => {});
    }, 50);
    return () => clearInterval(id);
  }, []);

  // Drag-move events from Rust
  useEffect(() => {
    const unM = listen<[string, number, number]>('drag-move', e => {
      const [userId, dxPhys, dyPhys] = e.payload;
      const dpr = window.devicePixelRatio || 1;
      const dx = dxPhys / dpr;
      const dy = dyPhys / dpr;
      setPositions(prev => {
        const us = usersRef.current;
        const idx = us.findIndex(u => u.userId === userId);
        const base = prev[userId] ?? getCharacterPosition(idx, us.length);
        const next = { ...prev, [userId]: { x: base.x + dx, y: base.y + dy } };
        savePositions(next);
        return next;
      });
    });
    return () => { unM.then(f => f()); };
  }, []);

  useEffect(() => {
    emitTo('main', 'overlay-ready', {}).catch(console.error);
    const unU = listen<UserState[]>('users-updated', e => setUsers(e.payload));
    const unB = listen<Record<string, string>>('bubble-updated', e => setBubbleMessages(e.payload));
    const unR = listen('reset-positions', () => {
      sessionStorage.removeItem('together-positions');
      setPositions({});
    });
    const unS = listen<number>('character-size-changed', e => setCharacterSize(e.payload));
    const unH = listen<string[]>('hidden-users-changed', e => setHiddenUsers(new Set(e.payload)));
    return () => { unU.then(f => f()); unB.then(f => f()); unR.then(f => f()); unS.then(f => f()); unH.then(f => f()); };
  }, []);

  const scale = SIZE_SCALE[characterSize] ?? 1.0;
  const visibleUsers = users.filter(u => !hiddenUsers.has(u.userId));

  return (
    <div style={{ position: 'fixed', inset: 0, pointerEvents: 'none', background: 'transparent', overflow: 'hidden' }}>
      {visibleUsers.map((user, i) => (
        <CharacterCard
          key={user.userId}
          user={user}
          index={i}
          position={positions[user.userId] ?? getCharacterPosition(i, visibleUsers.length)}
          bubbleMessage={bubbleMessages[user.userId]}
          scale={scale}
        />
      ))}
    </div>
  );
}
