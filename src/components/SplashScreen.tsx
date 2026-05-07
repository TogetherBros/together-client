import { getCurrentWindow } from '@tauri-apps/api/window';
import { logoImg } from '../characters';

export default function SplashScreen() {
  const win = getCurrentWindow();
  return (
    <div className="splash">
      <div className="titlebar titlebar-splash" data-tauri-drag-region>
        <div className="titlebar-buttons">
          <button
            className="titlebar-btn titlebar-minimize"
            onClick={() => win.minimize()}
            aria-label="최소화"
          />
          <button
            className="titlebar-btn titlebar-close"
            onClick={() => win.hide()}
            aria-label="닫기"
          />
        </div>
      </div>
      <img src={logoImg} alt="Together" className="splash-logo" />
      <span className="splash-title">Together</span>
    </div>
  );
}
