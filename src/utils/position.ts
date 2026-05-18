// Returns [windowX, windowY] (top-left of a 240x280 character window)
export function getWindowPosition(index: number, total: number): [number, number] {
  const WINDOW_W  = 240;
  const WINDOW_H  = 280;
  const CHAR_W    = 90;
  const TASKBAR_H = 48;
  const screenW   = window.screen.width;
  const screenH   = window.screen.height;

  const spacing = total > 1
    ? (screenW - CHAR_W * total) / (total + 1)
    : (screenW - CHAR_W) / 2;
  const charCenterX = spacing + index * (CHAR_W + spacing) + CHAR_W / 2;

  return [
    Math.max(0, charCenterX - WINDOW_W / 2),
    Math.max(0, screenH - TASKBAR_H - WINDOW_H),
  ];
}

export function getDefaultCharacterPosition(index: number, total: number) {
  const TASKBAR_HEIGHT = 48;
  const CHAR_WIDTH = 84;
  const screenW = window.screen.width;

  const spacing = (screenW - CHAR_WIDTH) / Math.max(total, 1);
  const x = spacing * index + spacing / 2 - CHAR_WIDTH / 2;
  const y = window.screen.height - TASKBAR_HEIGHT - 180;

  return { x, y };
}

// 간단한 hardcoded 위치: 화면 하단 중앙
export function getCharacterPosition(index: number, total: number) {
  const TASKBAR_HEIGHT = 48;
  const CHAR_WIDTH = 84;
  const screenW = window.screen.width;
  const screenH = window.screen.height;
  
  const spacing = total > 1 ? (screenW - CHAR_WIDTH * total) / (total + 1) : (screenW - CHAR_WIDTH) / 2;
  const x = spacing + index * (CHAR_WIDTH + spacing);
  const y = screenH - TASKBAR_HEIGHT - 150;
  
  return { x, y };
}
