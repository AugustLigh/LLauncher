import { useEffect } from 'react';

// Close a modal/overlay with the Escape key. Shared by every dialog so they
// behave consistently and stay usable without a mouse (keyboard, Steam Deck).
// `enabled` lets a caller suppress dismissal while an action is in flight.
export default function useModalDismiss(onClose, enabled = true) {
  useEffect(() => {
    if (!enabled || !onClose) return undefined;
    const onKeyDown = (e) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
      }
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [onClose, enabled]);
}
