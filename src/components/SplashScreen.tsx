import { logoImg } from '../characters';

export default function SplashScreen() {
  return (
    <div className="splash-outer">
      <div className="splash">
        <img src={logoImg} alt="Together" className="splash-logo" />
        <span className="splash-title">Together</span>
      </div>
    </div>
  );
}
