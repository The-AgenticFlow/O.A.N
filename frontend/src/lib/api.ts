const API_BASE = '/api'

export async function api<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  })

  if (!res.ok) {
    const error = await res.json().catch(() => ({ error: 'Unknown error' }))
    throw new Error(error.error || `HTTP ${res.status}`)
  }

  return res.json()
}

export const tasks = {
  list: () => api<Task[]>('/tasks'),
  get: (id: string) => api<Task>(`/tasks/${id}`),
  create: (data: CreateTaskRequest) => api<CreateTaskResponse>('/tasks', {
    method: 'POST',
    body: JSON.stringify(data),
  }),
  claim: (id: string, data: ClaimTaskRequest) => api<ClaimTaskResponse>(`/tasks/${id}/claim`, {
    method: 'POST',
    body: JSON.stringify(data),
  }),
  submit: (id: string, result: string) => api<{ status: string }>(`/tasks/${id}/submit`, {
    method: 'POST',
    body: JSON.stringify({ result }),
  }),
  status: (id: string) => api<TaskStatusResponse>(`/tasks/${id}/status`),
}

export interface Task {
  id: string
  prompt: string
  bounty_sats: number
  stake_sats: number
  status: string
  buyer_pubkey: string
  worker_pubkey?: string
  worker_invoice?: string
  result?: string
  created_at: string
}

export interface CreateTaskRequest {
  prompt: string
  bounty_sats: number
  stake_sats?: number
}

export interface CreateTaskResponse {
  task_id: string
  escrow_invoice: string
  payment_hash: string
  amount_sats: number
}

export interface ClaimTaskRequest {
  worker_pubkey: string
  worker_invoice: string
}

export interface ClaimTaskResponse {
  claimed: boolean
  stake_invoice?: string
}

export interface TaskStatusResponse {
  task_id: string
  status: string
  result?: string
  payout_tx?: string
}
