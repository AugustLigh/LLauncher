import { CSSProperties, ReactNode } from 'react';
import './GlassCard.css';

export interface GlassCardProps {
  children?: ReactNode;
  className?: string;
  style?: CSSProperties;
}

export default function GlassCard({ children, className = '', style = {} }: GlassCardProps) {
  return (
    <div className={`glass-card ${className}`} style={style}>
      {children}
    </div>
  );
}
