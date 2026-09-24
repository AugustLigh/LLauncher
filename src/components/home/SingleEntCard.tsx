import { useState } from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { SingleEnt } from '../../bindings';
import './SingleEntCard.css';

export interface SingleEntCardProps {
  singleEnt?: SingleEnt | null;
}

export default function SingleEntCard({ singleEnt }: SingleEntCardProps) {
  const [hovering, setHovering] = useState(false);

  if (!singleEnt || !singleEnt.version_url) return null;

  const handleClick = () => {
    if (singleEnt.jump_url) {
      openUrl(singleEnt.jump_url);
    }
  };

  return (
    <div className="single-ent-card" onClick={handleClick}>
      <img
        className="single-ent-card__image"
        src={singleEnt.version_url}
        alt="Promotion"
        draggable={false}
      />
      {singleEnt.button_url && (
        <img
          className="single-ent-card__button"
          src={hovering && singleEnt.button_hover_url ? singleEnt.button_hover_url : singleEnt.button_url}
          alt="Action"
          draggable={false}
          onMouseEnter={() => setHovering(true)}
          onMouseLeave={() => setHovering(false)}
        />
      )}
    </div>
  );
}
