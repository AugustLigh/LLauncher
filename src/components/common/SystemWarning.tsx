import { ReactNode } from 'react';
import './SystemWarning.css';

export interface SystemWarningProps {
  message?: ReactNode;
  type?: 'error' | 'warn';
}

export default function SystemWarning({ message, type = 'error' }: SystemWarningProps) {
  return (
    <div className={`system-warning ${type === 'warn' ? 'system-warning--warn' : ''}`}>
      <span className="system-warning__icon">{type === 'warn' ? '\u26A0' : '\u2716'}</span>
      <span className="system-warning__text">{message}</span>
    </div>
  );
}
