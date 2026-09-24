import { MouseEventHandler, ReactNode } from 'react';
import './IconButton.css';

export interface IconButtonProps {
  children?: ReactNode;
  onClick?: MouseEventHandler<HTMLButtonElement>;
  title?: string;
  className?: string;
}

export default function IconButton({ children, onClick, title, className = '' }: IconButtonProps) {
  return (
    <button className={`icon-button ${className}`} onClick={onClick} title={title}>
      {children}
    </button>
  );
}
