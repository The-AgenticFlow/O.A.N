import { useState, useEffect } from 'react'
import { Zap, Clock, CheckCircle2, AlertCircle, ExternalLink, Copy, RefreshCw } from 'lucide-react'
import { useL402 } from '../hooks/useL402'

interface Task {
  id: string
  prompt: string
  bounty_sats: number
  stake_sats: number
  status: string
  buyer_pubkey: string
  created_at: string
}

export default function TaskBoard() {
  const [tasks, setTasks] = useState<Task[]>([])
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const { isAuthenticated, payForAccess, challenge } = useL402()

  useEffect(() => {
    if (isAuthenticated) {
      fetchTasks()
    } else {
      setLoading(false)
    }
  }, [isAuthenticated])

  const fetchTasks = async () => {
    setRefreshing(true)
    try {
      const res = await fetch('/api/tasks')
      if (res.status === 402) {
        await res.json()
        return
      }
      const data = await res.json()
      setTasks(data)
    } catch (error) {
      console.error('Failed to fetch tasks:', error)
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }

  const copyInvoice = (invoice: string) => {
    navigator.clipboard.writeText(invoice)
  }

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'funded':
        return (
          <span className="status-badge status-funded">
            <CheckCircle2 className="w-3 h-3" />
            {status}
          </span>
        )
      case 'pending_payment':
        return (
          <span className="status-badge status-pending">
            <Clock className="w-3 h-3" />
            {status}
          </span>
        )
      default:
        return (
          <span className="status-badge status-default">
            <AlertCircle className="w-3 h-3" />
            {status}
          </span>
        )
    }
  }

  if (!isAuthenticated && challenge) {
    return (
      <div className="flex flex-col items-center justify-center py-24 animate-fade-in-up">
        <div className="terminal-panel p-8 max-w-md w-full glow-amber">
          <div className="text-center">
            <div className="relative inline-block mb-6">
              <div className="absolute inset-0 bg-amber-400/20 blur-xl rounded-full animate-pulse-glow" />
              <Zap className="w-16 h-16 text-amber-400 relative mx-auto" />
            </div>
            <h2 className="text-2xl font-bold mb-2 amber-text">Access Required</h2>
            <p className="text-void-border mb-6 font-mono text-sm">1 satoshi to view the task board</p>
            
            <div className="bg-void-deep p-4 rounded-lg mb-6 font-mono text-xs text-void-border break-all border border-void-border">
              {challenge.invoice}
            </div>
            
            <div className="flex gap-3 justify-center">
              <button
                onClick={() => copyInvoice(challenge.invoice)}
                className="btn-secondary flex items-center gap-2"
              >
                <Copy className="w-4 h-4" />
                Copy
              </button>
              <button
                onClick={payForAccess}
                className="btn-primary flex items-center gap-2 animate-pulse-glow"
              >
                <Zap className="w-4 h-4" />
                Pay with Wallet
              </button>
            </div>
          </div>
        </div>
      </div>
    )
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center py-24">
        <div className="relative">
          <div className="absolute inset-0 bg-amber-400/20 blur-lg rounded-full" />
          <RefreshCw className="w-8 h-8 text-amber-400 animate-spin relative" />
        </div>
      </div>
    )
  }

  return (
    <div className="animate-fade-in-up">
      <div className="flex items-center justify-between mb-10">
        <div>
          <div className="flex items-center gap-3 mb-1">
            <Zap className="w-6 h-6 text-amber-400" />
            <h2 className="text-3xl font-bold tracking-tight">Task Board</h2>
          </div>
          <p className="text-void-border font-mono text-sm">Available tasks ready for execution</p>
        </div>
        <button
          onClick={fetchTasks}
          disabled={refreshing}
          className="btn-secondary flex items-center gap-2"
        >
          <RefreshCw className={`w-4 h-4 ${refreshing ? 'animate-spin' : ''}`} />
          Refresh
        </button>
      </div>

      {tasks.length === 0 ? (
        <div className="terminal-panel p-12 text-center">
          <Zap className="w-12 h-12 mx-auto mb-4 text-void-border opacity-50" />
          <p className="text-void-border mb-2">No tasks available</p>
          <p className="text-sm text-void-border/50 font-mono">Check back soon for new opportunities</p>
        </div>
      ) : (
        <div className="grid gap-4">
          {tasks.map((task, index) => (
            <div
              key={task.id}
              className="terminal-panel p-6 hover:border-amber-400/30 transition-all duration-300 group"
              style={{ animationDelay: `${index * 0.1}s` }}
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 pr-6">
                  <p className="text-lg mb-3 text-void-surface group-hover:text-amber-300 transition-colors">{task.prompt}</p>
                  <div className="flex items-center gap-4 text-sm font-mono">
                    {getStatusBadge(task.status)}
                    <span className="flex items-center gap-1 text-void-border">
                      <Clock className="w-3 h-3" />
                      {new Date(task.created_at).toLocaleDateString()}
                    </span>
                  </div>
                </div>
                <div className="text-right">
                  <div className="flex items-center gap-1 text-amber-400 mb-1">
                    <Zap className="w-5 h-5" />
                    <span className="text-2xl font-bold font-mono">{task.bounty_sats}</span>
                  </div>
                  <span className="text-xs text-void-border font-mono">sats</span>
                  {task.stake_sats > 0 && (
                    <div className="text-xs text-void-border/50 mt-1 font-mono">
                      Stake: {task.stake_sats} sats
                    </div>
                  )}
                </div>
              </div>
              
              <div className="flex items-center justify-between mt-4 pt-4 border-t border-void-border">
                <span className="text-xs text-void-border/50 font-mono truncate max-w-[200px]">
                  {task.id}
                </span>
                <a
                  href={`/human?task=${task.id}`}
                  className="btn-cyan flex items-center gap-2 text-sm py-2 px-4"
                >
                  Claim Task
                  <ExternalLink className="w-3 h-3" />
                </a>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
