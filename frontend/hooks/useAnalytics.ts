import { useEffect, ReactNode } from 'react'
import { initAnalytics, trackEvent } from '../lib/analytics'


export default function RootLayout({ children }: { children: ReactNode }) {
  useEffect(() => {
    initAnalytics()
  }, [])

  return children
}
export const useAnalytics = () => {
  const logEvent = (name: string, params?: Record<string, unknown>) => {
    trackEvent(name, params)
  }

  return { logEvent }
}
