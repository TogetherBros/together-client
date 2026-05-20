export type Character = 'CAT' | 'DOG' | 'BUNNY' | 'BEAR' | 'BIRD' | 'BEE' | 'MONKEY' | 'KOALA' | 'KAKATU' | 'DOLPHIN';

export interface UserState {
  userId: string;
  nickname: string;
  character: Character;
  activity: string;
}

export interface ChatMessage {
  userId: string;
  nickname: string;
  character: Character;
  message: string;
  sentAt: string;
  msgType?: 'chat';
}

export interface ActivityRequest {
  userId: string;
  activity: string;
}

export interface LeaveRequest {
  userId: string;
}

export interface RoomConfig {
  deviceId: string;
  nickname: string;
  roomCode: string;
  joinedAt: number;
}

export type AppScreen = 'splash' | 'lobby' | 'character-select' | 'overlay' | 'chat';
