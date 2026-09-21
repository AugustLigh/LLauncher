// @ts-nocheck

import './IconButton.css';

export default function IconButton({ children, onClick, title, className = '' }: any) {
  return (
    <button className={`icon-button ${className}`} onClick={onClick} title={title}>
      {children}
    </button>
  );
}
