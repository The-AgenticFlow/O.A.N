import { useState, useCallback } from 'react'

const STORAGE_KEY_LN = 'oan_human_ln_address'
const STORAGE_KEY_PUBKEY = 'oan_human_pubkey'

export function getOrCreatePubkey(): string {
  let pubkey = localStorage.getItem(STORAGE_KEY_PUBKEY)
  if (!pubkey) {
    pubkey = 'human_' + crypto.randomUUID()
    localStorage.setItem(STORAGE_KEY_PUBKEY, pubkey)
  }
  return pubkey
}

export function useWalletConnection() {
  const [lightningAddress, setLightningAddress] = useState<string>(() => localStorage.getItem(STORAGE_KEY_LN) || '')
  const [isConnected, setIsConnected] = useState<boolean>(() => !!localStorage.getItem(STORAGE_KEY_LN))

  const saveWallet = useCallback((ln: string) => {
    setLightningAddress(ln)
    setIsConnected(true)
    localStorage.setItem(STORAGE_KEY_LN, ln)
  }, [])

  const disconnectWallet = useCallback(() => {
    setLightningAddress('')
    setIsConnected(false)
    localStorage.removeItem(STORAGE_KEY_LN)
    localStorage.removeItem(STORAGE_KEY_PUBKEY)
  }, [])

  return {
    lightningAddress,
    isConnected,
    saveWallet,
    disconnectWallet,
    workerPubkey: getOrCreatePubkey(),
  }
}
