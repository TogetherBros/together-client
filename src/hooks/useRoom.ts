import { useEffect, useRef, useState, MutableRefObject } from 'react';
import { Client } from '@stomp/stompjs';
import { Character, UserState, ChatMessage, RoomConfig } from '../types';

const ALL_CHARACTERS: Character[] = ['DOG', 'CAT', 'BUNNY', 'BEAR', 'BIRD', 'BEE'];

function pickRandom<T>(arr: T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

export interface RoomState {
  users: UserState[];
  messages: ChatMessage[];
  bubbleMessages: Record<string, string>;
  error: string | null;
  activityRef: MutableRefObject<string>;
  sendChat: (text: string) => void;
  sendActivity: (activity: string) => void;
}

export function useRoom(config: RoomConfig | null): RoomState {
  const [users, setUsers] = useState<UserState[]>([]);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [bubbleMessages, setBubbleMessages] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);

  const clientRef = useRef<Client | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const activityRef = useRef('접속 중');
  // 서버에 최종 등록된 캐릭터 (충돌 해소 후 갱신)
  const charRef = useRef<Character>('DOG');

  useEffect(() => {
    if (!config) {
      setUsers([]);
      setMessages([]);
      setBubbleMessages({});
      setError(null);
      return;
    }

    const { roomCode, deviceId, nickname } = config;
    charRef.current = pickRandom(ALL_CHARACTERS); // 랜덤 초기 배정

    const client = new Client({
      brokerURL: 'ws://localhost:8080/ws',
      reconnectDelay: 3000,
      onConnect: () => {
        // ── 유저 목록 구독 ──────────────────────────────
        client.subscribe(`/topic/room/${roomCode}`, (msg) => {
          const received: UserState[] = JSON.parse(msg.body);
          setUsers(received);

          // 다른 사람과 캐릭터 충돌 시 첫 번째 빈 캐릭터로 재배정
          const others = received.filter(u => u.userId !== deviceId);
          const taken = new Set(others.map(u => u.character));

          if (taken.has(charRef.current)) {
            const available = ALL_CHARACTERS.filter(c => !taken.has(c));
            if (available.length > 0) {
              const next = pickRandom(available);
              charRef.current = next;
              client.publish({
                destination: `/app/room/${roomCode}/join`,
                body: JSON.stringify({
                  userId: deviceId,
                  nickname,
                  character: next,
                  activity: activityRef.current,
                }),
              });
            }
          }
        });

        // ── 채팅 구독 ───────────────────────────────────
        client.subscribe(`/topic/room/${roomCode}/chat`, (msg) => {
          const chatMsg: ChatMessage = JSON.parse(msg.body);
          setMessages(prev => [...prev, chatMsg]);

          // 3.5초 동안 캐릭터 머리 위 말풍선 표시
          setBubbleMessages(prev => ({ ...prev, [chatMsg.userId]: chatMsg.message }));
          setTimeout(() => {
            setBubbleMessages(prev => {
              const { [chatMsg.userId]: _, ...rest } = prev;
              return rest;
            });
          }, 3500);
        });

        // ── 에러 구독 ───────────────────────────────────
        client.subscribe('/user/queue/error', (msg) => {
          setError(msg.body);
        });

        // ── 방 입장 ─────────────────────────────────────
        client.publish({
          destination: `/app/room/${roomCode}/join`,
          body: JSON.stringify({
            userId: deviceId,
            nickname,
            character: charRef.current,
            activity: activityRef.current,
          }),
        });

        // ── 활동 주기 전송 (3초) ─────────────────────────
        if (intervalRef.current) clearInterval(intervalRef.current);
        intervalRef.current = setInterval(() => {
          if (client.connected) {
            client.publish({
              destination: `/app/room/${roomCode}/activity`,
              body: JSON.stringify({ userId: deviceId, activity: activityRef.current }),
            });
          }
        }, 3000);
      },
    });

    client.activate();
    clientRef.current = client;

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      if (client.connected) {
        client.publish({
          destination: `/app/room/${roomCode}/leave`,
          body: JSON.stringify({ userId: deviceId }),
        });
      }
      client.deactivate();
      clientRef.current = null;
    };
  }, [config]);

  const sendChat = (text: string) => {
    const client = clientRef.current;
    if (!client?.connected || !config) return;
    client.publish({
      destination: `/app/room/${config.roomCode}/chat`,
      body: JSON.stringify({
        userId: config.deviceId,
        nickname: config.nickname,
        character: charRef.current,
        message: text,
      }),
    });
  };

  const sendActivity = (activity: string) => {
    const client = clientRef.current;
    if (!client?.connected || !config) return;
    client.publish({
      destination: `/app/room/${config.roomCode}/activity`,
      body: JSON.stringify({ userId: config.deviceId, activity }),
    });
  };

  return { users, messages, bubbleMessages, error, activityRef, sendChat, sendActivity };
}
