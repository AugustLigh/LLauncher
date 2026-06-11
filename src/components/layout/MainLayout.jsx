import { useState } from 'react';
import './MainLayout.css';

export default function MainLayout({ background, children }) {
  const [bgLoaded, setBgLoaded] = useState(false);
  const [videoFailed, setVideoFailed] = useState(false);

  const videoUrl = !videoFailed ? background?.video_url : '';
  const imageUrl = background?.url;

  return (
    <div className="main-layout">
      <div className="main-layout__background">
        {videoUrl ? (
          <video
            src={videoUrl}
            poster={imageUrl || undefined}
            className={bgLoaded ? 'loaded' : ''}
            autoPlay
            loop
            muted
            playsInline
            onPlaying={() => setBgLoaded(true)}
            onError={() => {
              setVideoFailed(true);
              setBgLoaded(false);
            }}
          />
        ) : (
          imageUrl && (
            <img
              src={imageUrl}
              alt=""
              className={bgLoaded ? 'loaded' : ''}
              onLoad={() => setBgLoaded(true)}
            />
          )
        )}
      </div>
      <div className="main-layout__overlay" />
      <div className="main-layout__content">
        {children}
      </div>
    </div>
  );
}
