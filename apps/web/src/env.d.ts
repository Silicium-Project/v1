/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<object, object, unknown>
  export default component
}

import 'vue-router'

declare module 'vue-router' {
  interface RouteMeta {
    /** Requires a signed-in session. */
    auth?: boolean
    /** Translation key of the page title. */
    title?: string
  }
}
