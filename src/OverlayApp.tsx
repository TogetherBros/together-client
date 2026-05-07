import { useEffect, useState } from 'react';
import { listen, emitTo } from '@tauri-apps/api/event';
import { UserState } from './types';
import CharacterCard from './components/CharacterCard';

export default function OverlayApp() {
  const [users, setUsers] = useState<UserState[]>([]);
  const [bubbleMessages, setBubbleMessages] = useState<Record<string, string>>({});

  useEffect(() => {
    emitTo('main', 'overlay-ready', {}).catch(console.error);
    const unU = listen<UserState[]>('users-updated', e => setUsers(e.payload));
    const unB = listen<Record<string, string>>('bubble-updated', e => setBubbleMessages(e.payload));
    return () => { unU.then(f => f()); unB.then(f => f()); };
  }, []);

  return (
    <div style={{ position: 'fixed', inset: 0, pointerEvents: 'none', background: 'transparent', overflow: 'hidden' }}>
      {users.map((user, i) => (
        <CharacterCard
          key={user.userId}
          user={user}
          index={i}
          total={users.length}
          bubbleMessage={bubbleMessages[user.userId]}
        />
      ))}
    </div>
  );
}
