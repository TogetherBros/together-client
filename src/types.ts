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
  msgType?: 'chat' | 'game-invite' | 'game-word' | 'game-over';
  gameData?: {
    gameId?: string;
    word?: string;
    nextChar?: string;
    submitterNickname?: string;
    winnerNickname?: string;
    loserNickname?: string;
    reason?: string;
  };
}

export type GameType = 'WORD_CHAIN';
export type GameStatus = 'waiting' | 'playing' | 'ended';

export interface GameState {
  gameId: string;
  type: GameType;
  status: GameStatus;
  hostId: string;
  hostNickname: string;
  participants: string[];
  currentTurnUserId: string;
  currentTurnNickname: string;
  nextChar: string;
  turnDeadline: string;
}

export interface GameEvent extends GameState {
  eventType: 'GAME_CREATED' | 'PLAYER_JOINED' | 'GAME_STARTED' | 'WORD_SUBMITTED' | 'GAME_OVER';
  submitterNickname?: string;
  word?: string;
  winnerNickname?: string;
  loserNickname?: string;
  reason?: string;
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
