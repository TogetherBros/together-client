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

