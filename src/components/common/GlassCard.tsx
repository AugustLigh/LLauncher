// @ts-nocheck

import './GlassCard.css';

export default function GlassCard({ children, className = '', style = {} }: any) {
  return (
    <div className={`glass-card ${className}`} style={style}>
      {children}
    </div>
  );
}
