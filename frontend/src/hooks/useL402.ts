import { useState, useCallback } from 'react'

interface L402Challenge {
  macaroon: string
  invoice: string
  payment_hash: string
  amount_sats: number
  expires_at: string
}

export function useL402() {
  const [isAuthenticated, setIsAuthenticated] = useState(false)
  const [challenge, setChallenge] = useState<L402Challenge | null>(null)
  const [macaroon, setMacaroon] = useState<string | null>(
    localStorage.getItem('l402_macaroon')
  )

  const checkAuth = useCallback(async () => {
    if (macaroon) {
      try {
        const res = await fetch('/api/l402/verify', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ macaroon, preimage: 'paid' }),
        })
        const data = await res.json()
        if (data.valid) {
          setIsAuthenticated(true)
          return
        }
      } catch (error) {
        console.error('L402 verification failed:', error)
      }
    }
    fetchChallenge()
  }, [macaroon])

  const fetchChallenge = useCallback(async () => {
    try {
      const res = await fetch('/api/tasks')
      if (res.status === 402) {
        const data = await res.json()
        setChallenge(data)
      }
    } catch (error) {
      console.error('Failed to fetch L402 challenge:', error)
    }
  }, [])

  const payForAccess = useCallback(async () => {
    if (!challenge) return
    
    console.log('Opening wallet to pay invoice:', challenge.invoice)
    
    if (typeof (window as any).webln !== 'undefined') {
      try {
        await (window as any).webln.enable()
        const result = await (window as any).webln.sendPayment(challenge.invoice)
        if (result.preimage) {
          setMacaroon(challenge.macaroon)
          localStorage.setItem('l402_macaroon', challenge.macaroon)
          setIsAuthenticated(true)
          setChallenge(null)
        }
      } catch (error) {
        console.error('Payment failed:', error)
      }
    } else {
      console.log('No WebLN wallet available. Copy the invoice manually.')
    }
  }, [challenge])

  const logout = useCallback(() => {
    localStorage.removeItem('l402_macaroon')
    setMacaroon(null)
    setIsAuthenticated(false)
    setChallenge(null)
  }, [])

  return {
    isAuthenticated,
    challenge,
    macaroon,
    checkAuth,
    payForAccess,
    logout,
  }
}
