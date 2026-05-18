import { useCallback, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

export function useNotificationSound() {
  const mutedRef = useRef(false);

  useEffect(() => {
    const unlisten = listen<boolean>('mute-changed', ({ payload }) => {
      mutedRef.current = payload;
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  const play = useCallback(() => {
    if (mutedRef.current) return;
    try {
      const ctx = new AudioContext();
      const now = ctx.currentTime;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.type = 'sine';
      osc.frequency.setValueAtTime(880, now);
      osc.frequency.setValueAtTime(1108, now + 0.1);
      gain.gain.setValueAtTime(0, now);
      gain.gain.linearRampToValueAtTime(0.25, now + 0.015);
      gain.gain.exponentialRampToValueAtTime(0.001, now + 0.45);
      osc.start(now);
      osc.stop(now + 0.45);
      osc.onended = () => ctx.close();
    } catch {}
  }, []);

  return play;
}
