import { useEffect, useRef, useState, MutableRefObject } from 'react';
import { Client } from '@stomp/stompjs';
import { Character, UserState, ChatMessage, RoomConfig } from '../types';

const ALL_CHARACTERS: Character[] = ['DOG', 'CAT', 'BUNNY', 'BEAR', 'BIRD', 'BEE', 'MONKEY', 'KOALA', 'KAKATU', 'DOLPHIN'];

export { ALL_CHARACTERS };

export interface RoomState {
  users: UserState[];
  messages: ChatMessage[];
  bubbleMessages: Record<string, string>;
  error: string | null;
  characterError: string | null;
  clearCharacterError: () => void;
  activityRef: MutableRefObject<string>;
  takenCharacters: Character[];
  sendChat: (text: string) => void;
  sendActivity: (activity: string) => void;
  joinWithCharacter: (character: Character) => Promise<boolean>;
}

export function useRoom(config: RoomConfig | null): RoomState {
  const [users, setUsers] = useState<UserState[]>([]);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [bubbleMessages, setBubbleMessages] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [characterError, setCharacterError] = useState<string | null>(null);

  const clientRef = useRef<Client | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const activityRef = useRef('접속 중');
  const charRef = useRef<Character | null>(null);
  const hasJoinedRef = useRef(false);
  const configRef = useRef(config);
  const joinResolverRef = useRef<((success: boolean) => void) | null>(null);
  configRef.current = config;

  useEffect(() => {
    if (!config) {
      setUsers([]);
      setMessages([]);
      setBubbleMessages({});
      setError(null);
      setCharacterError(null);
      charRef.current = null;
      hasJoinedRef.current = false;
      return;
    }

    charRef.current = null;
    hasJoinedRef.current = false;

    const { roomCode, deviceId, nickname } = config;
    let errorTimer: ReturnType<typeof setTimeout> | null = null;

    const startInterval = (client: Client) => {
      if (intervalRef.current) clearInterval(intervalRef.current);
      intervalRef.current = setInterval(() => {
        if (client.connected) {
          client.publish({
            destination: `/app/room/${roomCode}/activity`,
            body: JSON.stringify({ userId: deviceId, activity: activityRef.current }),
          });
        }
      }, 3000);
    };

    const client = new Client({
      brokerURL: 'wss://api.togetherbros.uk/ws',
      reconnectDelay: 3000,
      heartbeatIncoming: 10000,
      heartbeatOutgoing: 10000,
      onWebSocketError: () => {
        if (errorTimer) return;
        errorTimer = setTimeout(() => {
          setError('서버에 연결할 수 없습니다. 네트워크를 확인하세요.');
        }, 6000);
      },
      onStompError: (frame) => {
        setError(frame.headers?.message || 'STOMP 연결 오류');
      },
      onConnect: () => {
        if (errorTimer) { clearTimeout(errorTimer); errorTimer = null; }

        // full sync: 입장 성공 시 서버가 개인 큐로 전송
        client.subscribe('/user/queue/room-sync', (msg) => {
          const received: UserState[] = JSON.parse(msg.body);
          setUsers(received);
          if (joinResolverRef.current) {
            joinResolverRef.current(true);
            joinResolverRef.current = null;
          }
        });

        // 캐릭터 중복 에러
        client.subscribe('/user/queue/character-error', (msg) => {
          setCharacterError(msg.body);
          hasJoinedRef.current = false;
          charRef.current = null;
          if (intervalRef.current) { clearInterval(intervalRef.current); intervalRef.current = null; }
          if (joinResolverRef.current) {
            joinResolverRef.current(false);
            joinResolverRef.current = null;
          }
        });

        // delta: 신규 유저 입장
        client.subscribe(`/topic/room/${roomCode}/join`, (msg) => {
          const newUser: UserState = JSON.parse(msg.body);
          setUsers(prev =>
            prev.some(u => u.userId === newUser.userId) ? prev : [...prev, newUser]
          );
        });

        // delta: 유저 퇴장
        client.subscribe(`/topic/room/${roomCode}/leave`, (msg) => {
          const { userId } = JSON.parse(msg.body) as { userId: string };
          setUsers(prev => prev.filter(u => u.userId !== userId));
          setBubbleMessages(prev => {
            const { [userId]: _, ...rest } = prev;
            return rest;
          });
        });

        // delta: activity
        client.subscribe(`/topic/room/${roomCode}/activity`, (msg) => {
          const { userId, activity } = JSON.parse(msg.body) as { userId: string; activity: string };
          setUsers(prev => prev.map(u => u.userId === userId ? { ...u, activity } : u));
        });

        // chat
        client.subscribe(`/topic/room/${roomCode}/chat`, (msg) => {
          const chatMsg: ChatMessage = JSON.parse(msg.body);
          const localMsg: ChatMessage = { ...chatMsg, sentAt: new Date().toISOString() };
          setMessages(prev => [...prev, localMsg]);
          setBubbleMessages(prev => ({ ...prev, [chatMsg.userId]: chatMsg.message }));
          setTimeout(() => {
            setBubbleMessages(prev => {
              const { [chatMsg.userId]: _, ...rest } = prev;
              return rest;
            });
          }, 3500);
        });

        client.subscribe('/user/queue/error', (msg) => {
          setError(msg.body);
        });

        if (hasJoinedRef.current && charRef.current) {
          client.publish({
            destination: `/app/room/${roomCode}/join`,
            body: JSON.stringify({
              userId: deviceId,
              nickname,
              character: charRef.current,
              activity: activityRef.current,
            }),
          });
          startInterval(client);
        }
      },
    });

    client.activate();
    clientRef.current = client;

    return () => {
      if (errorTimer) { clearTimeout(errorTimer); errorTimer = null; }
      if (intervalRef.current) { clearInterval(intervalRef.current); intervalRef.current = null; }
      if (client.connected && hasJoinedRef.current) {
        client.publish({
          destination: `/app/room/${roomCode}/leave`,
          body: JSON.stringify({ userId: deviceId }),
        });
      }
      client.deactivate();
      clientRef.current = null;
    };
  }, [config]);

  const joinWithCharacter = (character: Character): Promise<boolean> => {
    charRef.current = character;
    hasJoinedRef.current = true;
    const client = clientRef.current;
    const cfg = configRef.current;
    if (!client?.connected || !cfg) return Promise.resolve(false);

    return new Promise((resolve) => {
      joinResolverRef.current = resolve;

      client.publish({
        destination: `/app/room/${cfg.roomCode}/join`,
        body: JSON.stringify({
          userId: cfg.deviceId,
          nickname: cfg.nickname,
          character,
          activity: activityRef.current,
        }),
      });

      // activity 인터벌은 join 성공 후 room-sync 핸들러에서 시작
      if (intervalRef.current) clearInterval(intervalRef.current);
      intervalRef.current = setInterval(() => {
        if (client.connected) {
          client.publish({
            destination: `/app/room/${cfg.roomCode}/activity`,
            body: JSON.stringify({ userId: cfg.deviceId, activity: activityRef.current }),
          });
        }
      }, 3000);

      // 5초 내 서버 응답 없으면 타임아웃 처리
      setTimeout(() => {
        if (joinResolverRef.current === resolve) {
          joinResolverRef.current = null;
          resolve(false);
        }
      }, 5000);
    });
  };

  const sendChat = (text: string) => {
    const client = clientRef.current;
    if (!client?.connected || !config || !charRef.current) return;
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
    activityRef.current = activity;
    const client = clientRef.current;
    if (!client?.connected || !config) return;
    client.publish({
      destination: `/app/room/${config.roomCode}/activity`,
      body: JSON.stringify({ userId: config.deviceId, activity }),
    });
  };

  const takenCharacters = users
    .filter(u => u.userId !== config?.deviceId)
    .map(u => u.character);

  const clearCharacterError = () => setCharacterError(null);

  return { users, messages, bubbleMessages, error, characterError, clearCharacterError, activityRef, takenCharacters, sendChat, sendActivity, joinWithCharacter };
}
