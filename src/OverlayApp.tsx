import { useEffect, useRef, useState } from 'react';
import { listen, emitTo } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { UserState } from './types';
import { getCharacterPosition } from './utils/position';
import CharacterCard from './components/CharacterCard';

type Position = { x: number; y: number };

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

  const usersRef = useRef<UserState[]>([]);
  const positionsRef = useRef<Record<string, Position>>({});

  useEffect(() => { usersRef.current = users; }, [users]);
  useEffect(() => { positionsRef.current = positions; }, [positions]);

  // Report zone coords to Rust every 50ms so it knows where to detect drags
  useEffect(() => {
    const id = setInterval(() => {
      const us = usersRef.current;
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
      const userIds = us.map(u => u.userId);
      invoke('update_character_zones', { zones, userIds }).catch(() => {});
    }, 50);
    return () => clearInterval(id);
  }, []);

  // Receive drag-move events emitted by Rust (physical pixel deltas)
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
    return () => { unU.then(f => f()); unB.then(f => f()); unR.then(f => f()); };
  }, []);

  return (
    <div style={{ position: 'fixed', inset: 0, pointerEvents: 'none', background: 'transparent', overflow: 'hidden' }}>
      {users.map((user, i) => (
        <CharacterCard
          key={user.userId}
          user={user}
          index={i}
          position={positions[user.userId] ?? getCharacterPosition(i, users.length)}
          bubbleMessage={bubbleMessages[user.userId]}
        />
      ))}
    </div>
  );
}
