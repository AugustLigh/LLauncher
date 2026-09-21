// @ts-nocheck

import './SystemWarning.css';

export default function SystemWarning({ message, type = 'error' }: any) {
  return (
    <div className={`system-warning ${type === 'warn' ? 'system-warning--warn' : ''}`}>
      <span className="system-warning__icon">{type === 'warn' ? '\u26A0' : '\u2716'}</span>
      <span className="system-warning__text">{message}</span>
    </div>
  );
}
