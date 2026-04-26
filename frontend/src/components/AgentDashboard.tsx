import { useState } from 'react'
import { Zap, Plus, Send, Loader2, Clock, TrendingUp, X, Copy } from 'lucide-react'

interface Task {
  id: string
  prompt: string
  bounty_sats: number
  stake_sats: number
  status: string
  escrow_invoice?: string
  payment_hash?: string
  result?: string
}

interface AgentStats {
  balance_sats: number
  pending_sats: number
  total_earned: number
}

export default function AgentDashboard() {
  const [tasks, setTasks] = useState<Task[]>([])
  const [stats] = useState<AgentStats>({ balance_sats: 0, pending_sats: 0, total_earned: 0 })
  const [showCreate, setShowCreate] = useState(false)
  const [creating, setCreating] = useState(false)
  const [newTask, setNewTask] = useState({ prompt: '', bounty_sats: 100, stake_sats: 0 })
  const [pendingInvoice, setPendingInvoice] = useState<Task | null>(null)
  const [paying, setPaying] = useState(false)

  const createTask = async () => {
    if (!newTask.prompt.trim()) return

    setCreating(true)
    try {
      const res = await fetch('/api/tasks', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(newTask),
      })
      
      const data = await res.json()
      
      setPendingInvoice({
        id: data.task_id,
        prompt: newTask.prompt,
        bounty_sats: data.amount_sats,
        stake_sats: 0,
        status: 'pending_payment',
        escrow_invoice: data.escrow_invoice,
        payment_hash: data.payment_hash,
      })
      
      setShowCreate(false)
      setNewTask({ prompt: '', bounty_sats: 100, stake_sats: 0 })
    } catch (error) {
      console.error('Failed to create task:', error)
    } finally {
      setCreating(false)
    }
  }

  const payInvoice = async () => {
    if (!pendingInvoice?.escrow_invoice) return
    
    setPaying(true)
    
    await new Promise(resolve => setTimeout(resolve, 2000))
    
    setTasks([...tasks, { ...pendingInvoice, status: 'funded' }])
    setPendingInvoice(null)
    setPaying(false)
  }

  const copyInvoice = (invoice: string) => {
    navigator.clipboard.writeText(invoice)
  }

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'funded':
        return <span className="status-badge status-funded">{status}</span>
      case 'claimed':
        return <span className="status-badge status-claimed">{status}</span>
      case 'completed':
        return <span className="status-badge status-completed">{status}</span>
      case 'pending_payment':
        return <span className="status-badge status-pending">{status}</span>
      default:
        return <span className="status-badge status-default">{status}</span>
    }
  }

  return (
    <div className="max-w-4xl mx-auto animate-fade-in-up">
      <div className="flex items-center justify-between mb-10">
        <div>
          <div className="flex items-center gap-3 mb-1">
            <Zap className="w-6 h-6 text-amber-400" />
            <h2 className="text-3xl font-bold tracking-tight">Agent Dashboard</h2>
          </div>
          <p className="text-void-border font-mono text-sm">Post tasks and manage your workflow</p>
        </div>
        <button
          onClick={() => setShowCreate(true)}
          className="btn-primary flex items-center gap-2"
        >
          <Plus className="w-5 h-5" />
          New Task
        </button>
      </div>

      <div className="grid grid-cols-3 gap-4 mb-10">
        <div className="terminal-panel p-5 animate-fade-in-up stagger-1">
          <div className="flex items-center gap-2 text-void-border mb-3 font-mono text-sm">
            <Zap className="w-4 h-4 text-amber-400" />
            Balance
          </div>
          <div className="text-2xl font-bold font-mono">{stats.balance_sats.toLocaleString()}</div>
          <div className="text-xs text-void-border/50 mt-1">sats available</div>
        </div>
        <div className="terminal-panel p-5 animate-fade-in-up stagger-2">
          <div className="flex items-center gap-2 text-void-border mb-3 font-mono text-sm">
            <Clock className="w-4 h-4 text-cyan-400" />
            Pending
          </div>
          <div className="text-2xl font-bold font-mono">{stats.pending_sats.toLocaleString()}</div>
          <div className="text-xs text-void-border/50 mt-1">sats in escrow</div>
        </div>
        <div className="terminal-panel p-5 animate-fade-in-up stagger-3">
          <div className="flex items-center gap-2 text-void-border mb-3 font-mono text-sm">
            <TrendingUp className="w-4 h-4 text-success" />
            Total Earned
          </div>
          <div className="text-2xl font-bold amber-text font-mono">{stats.total_earned.toLocaleString()}</div>
          <div className="text-xs text-void-border/50 mt-1">sats lifetime</div>
        </div>
      </div>

      {showCreate && (
        <div className="fixed inset-0 bg-void/80 backdrop-blur-sm flex items-center justify-center z-50 animate-fade-in-up">
          <div className="terminal-panel p-6 w-full max-w-lg mx-4 glow-amber">
            <div className="flex items-center justify-between mb-6">
              <h3 className="text-xl font-bold amber-text">Create New Task</h3>
              <button
                onClick={() => setShowCreate(false)}
                className="text-void-border hover:text-void-surface transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2 font-mono text-void-border">Task Description</label>
                <textarea
                  value={newTask.prompt}
                  onChange={(e) => setNewTask({ ...newTask, prompt: e.target.value })}
                  placeholder="What do you need done?"
                  rows={4}
                  className="terminal-input w-full resize-none"
                />
              </div>
              
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium mb-2 font-mono text-void-border">Bounty (sats)</label>
                  <input
                    type="number"
                    value={newTask.bounty_sats}
                    onChange={(e) => setNewTask({ ...newTask, bounty_sats: parseInt(e.target.value) || 0 })}
                    className="terminal-input w-full"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium mb-2 font-mono text-void-border">Stake Required</label>
                  <input
                    type="number"
                    value={newTask.stake_sats}
                    onChange={(e) => setNewTask({ ...newTask, stake_sats: parseInt(e.target.value) || 0 })}
                    className="terminal-input w-full"
                    placeholder="Optional"
                  />
                </div>
              </div>
            </div>
            
            <div className="flex gap-3 mt-6">
              <button
                onClick={() => setShowCreate(false)}
                className="btn-secondary flex-1"
              >
                Cancel
              </button>
              <button
                onClick={createTask}
                disabled={creating || !newTask.prompt.trim()}
                className="btn-primary flex-1 flex items-center justify-center gap-2"
              >
                {creating ? <Loader2 className="w-5 h-5 animate-spin" /> : <Plus className="w-5 h-5" />}
                Create Task
              </button>
            </div>
          </div>
        </div>
      )}

      {pendingInvoice && (
        <div className="terminal-panel p-6 mb-6 glow-cyan animate-slide-in-left">
          <div className="flex items-center gap-2 mb-4">
            <Zap className="w-6 h-6 text-amber-400" />
            <h3 className="text-lg font-semibold amber-text">Fund Task</h3>
          </div>
          
          <p className="text-void-border mb-4 font-mono text-sm">
            Pay <span className="text-amber-400">{pendingInvoice.bounty_sats}</span> sats to escrow. Task will be available once payment is confirmed.
          </p>
          
          <div className="bg-void-deep p-4 rounded-lg mb-4 font-mono text-xs text-void-border break-all border border-void-border">
            {pendingInvoice.escrow_invoice}
          </div>
          
          <div className="flex gap-3">
            <button
              onClick={() => copyInvoice(pendingInvoice.escrow_invoice || '')}
              className="btn-secondary flex items-center gap-2"
            >
              <Copy className="w-4 h-4" />
              Copy Invoice
            </button>
            <button
              onClick={payInvoice}
              disabled={paying}
              className="btn-primary flex items-center gap-2"
            >
              {paying ? <Loader2 className="w-5 h-5 animate-spin" /> : <Send className="w-5 h-5" />}
              Pay with Wallet
            </button>
          </div>
        </div>
      )}

      <div>
        <h3 className="text-xl font-bold mb-6 flex items-center gap-2">
          <Clock className="w-5 h-5 text-cyan-400" />
          Your Tasks
        </h3>
        
        {tasks.length === 0 ? (
          <div className="terminal-panel p-12 text-center">
            <Zap className="w-12 h-12 mx-auto mb-4 text-void-border opacity-50" />
            <p className="text-void-border mb-2">No tasks yet</p>
            <p className="text-sm text-void-border/50 font-mono">Create a task to get started</p>
          </div>
        ) : (
          <div className="space-y-4">
            {tasks.map((task) => (
              <div
                key={task.id}
                className="terminal-panel p-5 hover:border-cyan-400/30 transition-all duration-300"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1 pr-6">
                    <p className="text-void-surface mb-3">{task.prompt}</p>
                    {task.result && (
                      <div className="mt-2 p-3 bg-success/10 border border-success/20 rounded text-sm">
                        <strong className="text-success">Result:</strong> {task.result}
                      </div>
                    )}
                  </div>
                  <div className="text-right ml-4">
                    <div className="flex items-center gap-1 text-amber-400 mb-2">
                      <Zap className="w-4 h-4" />
                      <span className="font-bold font-mono">{task.bounty_sats}</span>
                    </div>
                    {getStatusBadge(task.status)}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
