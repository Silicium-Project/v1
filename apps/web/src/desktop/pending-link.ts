import { ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { isDesktop, takePendingLink } from '@/platform/desktop'

/** The code the window must confirm now, when a `silicium://link` link opened the application. */
export const requestedCode = ref<string | null>(null)

/** Event sent by the shell (apps/desktop/src-tauri/src/deep_link.rs). */
const EVENT = 'silicium-link'

/**
 * Listens for link requests from the shell. `onRequest` runs for each one so the window can
 * show the confirmation. A request raised before the window was ready waits in the shell.
 */
export async function watchLinkRequests(onRequest: () => void): Promise<void> {
  if (!isDesktop()) return
  const show = (code: string): void => {
    requestedCode.value = code
    onRequest()
  }
  try {
    // Listen first, then take the waiting request: nothing can slip between the two.
    await listen<string>(EVENT, (event) => show(event.payload))
    const waiting = await takePendingLink()
    if (waiting) show(waiting)
  } catch (failure) {
    console.error('link request not watched', failure)
  }
}
