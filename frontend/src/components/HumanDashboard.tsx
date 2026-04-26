import { useState, useEffect } from 'react'
import { Zap, Send, Loader2, CheckCircle2, XCircle, Wallet, ArrowRight } from 'lucide-react'
import { useSearchParams } from 'react-router-dom'

interface Task {
  id: string
  prompt: string
  bounty_sats: number
  status: string
  worker_invoice?: string
}

export default function HumanDashboard() {
  const [searchParams] = useSearchParams()
  const taskId = searchParams.get('task')
  
  const [task, setTask] = useState<Task | null>(null)
  const [loading, setLoading] = useState(false)
  const [claiming, setClaiming] = useState(false)
  const [submitting, setSubmitting] = useState(false)
  const [result, setResult] = useState('')
  const [lightningAddress, setLightningAddress] = useState('')
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null)

  useEffect(() => {
    if (taskId) {
      fetchTask(taskId)
    }
  }, [taskId])

  const fetchTask = async (id: string) => {
    setLoading(true)
    try {
      const res = await fetch(`/api/tasks/${id}`)
      const data = await res.json()
      setTask(data)
    } catch (error) {
      console.error('Failed to fetch task:', error)
    } finally {
      setLoading(false)
    }
  }

  const claimTask = async () => {
    if (!task || !lightningAddress) {
      setMessage({ type: 'error', text: 'Please enter your Lightning address first' })
      return
    }

    setClaiming(true)
    try {
      const res = await fetch(`/api/tasks/${task.id}/claim`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          worker_pubkey: 'human_' + Date.now(),
          worker_invoice: lightningAddress,
        }),
      })
      
      const data = await res.json()
      
      if (data.claimed) {
        setMessage({ type: 'success', text: 'Task claimed! Complete the work below.' })
        fetchTask(task.id)
      } else if (data.stake_invoice) {
        setMessage({ type: 'error', text: 'This task requires a stake payment' })
      }
    } catch (error) {
      setMessage({ type: 'error', text: 'Failed to claim task' })
    } finally {
      setClaiming(false)
    }
  }

  const submitResult = async () => {
    if (!task || !result.trim()) {
      setMessage({ type: 'error', text: 'Please enter your work result' })
      return
    }

    setSubmitting(true)
    try {
      const res = await fetch(`/api/tasks/${task.id}/submit`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ result }),
      })
      
      const data = await res.json()
      
      if (data.status === 'verifying') {
        setMessage({ type: 'success', text: 'Result submitted! Verifying and processing payment...' })
        pollForCompletion(task.id)
      }
    } catch (error) {
      setMessage({ type: 'error', text: 'Failed to submit result' })
    } finally {
      setSubmitting(false)
    }
  }

  const pollForCompletion = async (id: string) => {
    const interval = setInterval(async () => {
      try {
        const res = await fetch(`/api/tasks/${id}/status`)
        const data = await res.json()
        
        if (data.status === 'completed') {
          clearInterval(interval)
          setMessage({ type: 'success', text: 'Payment sent! Check your Lightning wallet.' })
        } else if (data.status === 'failed') {
          clearInterval(interval)
          setMessage({ type: 'error', text: 'Verification failed. No payment sent.' })
        }
      } catch (error) {
        console.error('Poll error:', error)
      }
    }, 3000)
  }

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'funded':
        return <span className="status-badge status-funded">{status}</span>
      case 'claimed':
        return <span className="status-badge status-claimed">{status}</span>
      case 'completed':
        return <span className="status-badge status-completed">{status}</span>
      case 'verifying':
        return <span className="status-badge status-verifying">{status}</span>
      default:
        return <span className="status-badge status-default">{status}</span>
    }
  }

  return (
    <div className="max-w-2xl mx-auto animate-fade-in-up">
      <div className="text-center mb-10">
        <div className="flex items-center justify-center gap-3 mb-2">
          <Wallet className="w-6 h-6 text-cyan-400" />
          <h2 className="text-3xl font-bold tracking-tight">Human Dashboard</h2>
        </div>
        <p className="text-void-border font-mono text-sm">Complete tasks and earn sats instantly</p>
      </div>

      {!lightningAddress && (
        <div className="terminal-panel p-6 mb-6 animate-slide-in-left glow-cyan">
          <div className="flex items-center gap-2 mb-4">
            <Wallet className="w-5 h-5 text-cyan-400" />
            <h3 className="font-semibold cyan-text">Connect Your Wallet</h3>
          </div>
          <input
            type="text"
            placeholder="your@lightning.address"
            value={lightningAddress}
            onChange={(e) => setLightningAddress(e.target.value)}
            className="terminal-input w-full"
          />
          <p className="text-xs text-void-border/50 mt-2 font-mono">
            Enter your Lightning address (Alby, Blink, etc.) to receive payments
          </p>
        </div>
      )}

      {message && (
        <div className={`flex items-center gap-2 p-4 rounded-lg mb-6 animate-fade-in-up ${
          message.type === 'success' 
            ? 'bg-success/10 text-success border border-success/20' 
            : 'bg-alert/10 text-alert border border-alert/20'
        }`}>
          {message.type === 'success' ? <CheckCircle2 className="w-5 h-5" /> : <XCircle className="w-5 h-5" />}
          <span className="font-mono text-sm">{message.text}</span>
        </div>
      )}

      {loading && (
        <div className="flex items-center justify-center py-12">
          <div className="relative">
            <div className="absolute inset-0 bg-cyan-400/20 blur-lg rounded-full" />
            <Loader2 className="w-8 h-8 text-cyan-400 animate-spin relative" />
          </div>
        </div>
      )}

      {task && !loading && (
        <div className="terminal-panel p-6 animate-fade-in-up">
          <div className="flex items-start justify-between mb-6">
            <div>
              <h3 className="text-lg font-semibold mb-1">Task Details</h3>
              <p className="text-void-border font-mono text-sm">{task.id}</p>
            </div>
            <div className="text-right">
              <div className="flex items-center gap-1 text-amber-400 mb-1">
                <Zap className="w-5 h-5" />
                <span className="text-2xl font-bold font-mono">{task.bounty_sats}</span>
                <span className="text-xs text-void-border">sats</span>
              </div>
              {getStatusBadge(task.status)}
            </div>
          </div>
          
          <p className="text-void-surface mb-6 text-lg">{task.prompt}</p>
          
          {task.status === 'funded' && (
            <button
              onClick={claimTask}
              disabled={claiming || !lightningAddress}
              className="btn-primary w-full flex items-center justify-center gap-2"
            >
              {claiming ? <Loader2 className="w-5 h-5 animate-spin" /> : <Zap className="w-5 h-5" />}
              Claim This Task
            </button>
          )}

          {task.status === 'claimed' && (
            <div className="mt-6 space-y-4">
              <div className="flex items-center gap-2 text-cyan-400 mb-2">
                <ArrowRight className="w-4 h-4" />
                <span className="font-mono text-sm">Submit your work result</span>
              </div>
              <textarea
                value={result}
                onChange={(e) => setResult(e.target.value)}
                placeholder="Enter your solution here..."
                rows={5}
                className="terminal-input w-full resize-none"
              />
              <button
                onClick={submitResult}
                disabled={submitting || !result.trim()}
                className="btn-cyan w-full flex items-center justify-center gap-2"
              >
                {submitting ? <Loader2 className="w-5 h-5 animate-spin" /> : <Send className="w-5 h-5" />}
                Submit & Get Paid
              </button>
            </div>
          )}
        </div>
      )}

      {!task && !loading && !taskId && (
        <div className="terminal-panel p-12 text-center">
          <Zap className="w-12 h-12 mx-auto mb-4 text-void-border opacity-50" />
          <p className="text-void-border mb-2">No task selected</p>
          <p className="text-sm text-void-border/50 font-mono">Browse the task board to find work</p>
        </div>
      )}
    </div>
  )
}
