// Copying text out of the launcher.
//
// `navigator.clipboard` is the obvious route and the one that fails: the
// webview is WebKitGTK on Linux, which only exposes the async clipboard API
// in a secure context and rejects the write otherwise — so the "Copy" button
// on the debug-info row silently did nothing for some users (issue #33).
// The `execCommand` path is deprecated but implemented everywhere the
// launcher runs, and it works from a user gesture, which is the only place
// these buttons are ever called from.
export async function copyText(text) {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Fall through to the selection-based copy below.
  }
  return legacyCopy(text);
}

function legacyCopy(text) {
  const previousFocus = document.activeElement;
  const area = document.createElement('textarea');
  area.value = text;
  // Off-screen but still focusable: `display: none` would make the selection
  // empty and the copy a no-op.
  area.setAttribute('readonly', '');
  area.style.position = 'fixed';
  area.style.top = '-1000px';
  area.style.opacity = '0';
  document.body.appendChild(area);
  try {
    area.select();
    area.setSelectionRange(0, area.value.length);
    return document.execCommand('copy');
  } catch {
    return false;
  } finally {
    area.remove();
    previousFocus?.focus({ preventScroll: true });
  }
}
