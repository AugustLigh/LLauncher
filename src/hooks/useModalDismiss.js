import { useEffect, useRef } from "react";
const stack = [];
const selector =
  'button:not(:disabled), a[href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), summary, [tabindex]:not([tabindex="-1"])';
export default function useModalDismiss(onClose, enabled = true) {
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    if (!enabled) return;
    const token = {};
    stack.push(token);
    const previous = document.activeElement;
    const dialogs = document.querySelectorAll('[role="dialog"]');
    const dialog = dialogs[dialogs.length - 1];
    const nodes = () =>
      dialog
        ? [...dialog.querySelectorAll(selector)].filter(
            (el) => el.getClientRects().length,
          )
        : [];
    const initial = dialog?.querySelector("[data-initial-focus]") || nodes()[0];
    if (dialog) {
      dialog.tabIndex = -1;
      (initial || dialog).focus();
    }
    const handle = (e) => {
      if (stack[stack.length - 1] !== token) return;
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopImmediatePropagation();
        close.current?.();
        return;
      }
      if (e.key === "Tab" && dialog) {
        const items = nodes();
        if (!items.length) {
          e.preventDefault();
          dialog.focus();
          return;
        }
        const first = items[0],
          last = items[items.length - 1];
        if (!dialog.contains(document.activeElement)) {
          e.preventDefault();
          first.focus();
        } else if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener("keydown", handle, true);
    return () => {
      const index = stack.indexOf(token);
      if (index >= 0) stack.splice(index, 1);
      document.removeEventListener("keydown", handle, true);
      if (dialog && previous?.isConnected) previous.focus();
    };
  }, [enabled]);
}
