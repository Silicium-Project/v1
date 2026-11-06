import { locale, t } from './index'

/** Date and time in the current locale, or a translated "unknown date" for bad input. */
export function formatDateTime(iso: string): string {
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return t('common.dateUnknown')
  return new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium', timeStyle: 'short' }).format(date)
}

/** Time of day only, in the current locale. */
export function formatTime(iso: string): string {
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return t('common.dateUnknown')
  return new Intl.DateTimeFormat(locale.value, { timeStyle: 'short' }).format(date)
}
