/** Scheme of the desktop application (apps/desktop: tauri.conf.json plugins.deep-link, src/link.rs). */
export const DESKTOP_SCHEME = 'silicium'

/**
 * Link that opens the desktop application on a link code. The browser asks the user to allow
 * the application; the application then shows the account before anything is linked.
 */
export function desktopLinkUrl(code: string): string {
  return `${DESKTOP_SCHEME}://link?code=${encodeURIComponent(code)}`
}
