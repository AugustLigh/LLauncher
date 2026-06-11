import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';

/**
 * Show a desktop notification. Failures (no permission, no notification
 * daemon) are silently ignored — notifications are a nice-to-have.
 */
export async function notify(title, body) {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      granted = (await requestPermission()) === 'granted';
    }
    if (granted) {
      sendNotification({ title, body });
    }
  } catch {
    // ignore
  }
}
