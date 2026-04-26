import { useState, useEffect, useCallback } from 'react'
import { Zap, Plus, Loader2, Clock, TrendingUp, X, Copy, RefreshCw, QrCode, Eye, Bot, Star, ChevronDown, Send, AlertCircle, CheckCircle2 } from 'lucide-react'
import QRCode from './QRCode'

interface Agent {
  pubkey: string
  name: string | null
  avatar_url: string | null
  agent_type: string | null
  reputation_score: number
  total_tasks: number
  is_active: boolean | null
}

interface Task {
  id: string
  prompt: string
  bounty_sats: number
  stake_sats: number
  status: string
  buyer_pubkey: string
  worker_pubkey: string | null
  escrow_invoice?: string
  payment_hash?: string
  result?: string
  failure_reason?: string
  created_at?: string
}

interface AgentStats {
  balance_sats: number
  pending_sats: number
  escrow_sats: number
  available_sats: number
  total_earned: number
}

export default function AgentDashboard() {
  const [tasks, setTasks] = useState<Task[]>([])
  const [agents, setAgents] = useState<Map<string, Agent>>(new Map())
  const [stats, setStats] = useState<AgentStats>({ balance_sats: 0, pending_sats: 0, escrow_sats: 0, available_sats: 0, total_earned: 0 })
  const [showCreate, setShowCreate] = useState(false)
  const [creating, setCreating] = useState(false)
  const [newTask, setNewTask] = useState({ prompt: '', bounty_sats: 100, stake_sats: 0 })
  const [pendingInvoice, setPendingInvoice] = useState<Task | null>(null)
  const [showQR, setShowQR] = useState(false)
  const [loading, setLoading] = useState(true)
  const [selectedTask, setSelectedTask] = useState<Task | null>(null)
  const [assignTarget, setAssignTarget] = useState<string | null>(null)
  const [assignAgent, setAssignAgent] = useState<string>('')
  const [assigning, setAssigning] = useState(false)
  const [activeTab, setActiveTab] = useState<'all' | 'created' | 'assigned'>('all')

  const fetchStats = useCallback(async () => {
    try {
      const res = await fetch('/api/agent/balance')
      if (res.ok) {
        const data = await res.json()
        setStats(data)
      }
    } catch (error) {
      console.error('Failed to fetch stats:', error)
    }
  }, [])

  const fetchTasks = useCallback(async () => {
    try {
      const res = await fetch('/api/tasks')
      if (res.ok) {
        const data = await res.json()
        setTasks(data)
      }
    } catch (error) {
      console.error('Failed to fetch tasks:', error)
    }
  }, [])

  const fetchAgents = useCallback(async () => {
    try {
      const res = await fetch('/api/agents')
      if (res.ok) {
        const agentList: Agent[] = await res.json()
        const agentMap = new Map(agentList.map(a => [a.pubkey, a]))
        setAgents(agentMap)
      }
    } catch (error) {
      console.error('Failed to fetch agents:', error)
    }
  }, [])

  const fetchPendingTasks = useCallback(async () => {
    try {
      const res = await fetch('/api/tasks?status=pending_payment')
      if (res.ok) {
        const data = await res.json()
        setTasks(prev => {
          const existingIds = new Set(prev.map(t => t.id))
          const newTasks = data.filter((t: Task) => !existingIds.has(t.id))
          return [...prev, ...newTasks]
        })
      }
    } catch (error) {
      console.error('Failed to fetch pending tasks:', error)
    }
  }, [])

  useEffect(() => {
    const init = async () => {
      setLoading(true)
      await Promise.all([fetchStats(), fetchTasks(), fetchPendingTasks(), fetchAgents()])
      setLoading(false)
    }
    init()
  }, [fetchStats, fetchTasks, fetchPendingTasks, fetchAgents])

  useEffect(() => {
    const interval = setInterval(async () => {
      fetchStats()
      fetchPendingTasks()
      
      if (pendingInvoice) {
        try {
          const res = await fetch(`/api/tasks/${pendingInvoice.id}`)
          if (res.ok) {
            const task = await res.json()
            if (task.status !== 'pending_payment') {
              setPendingInvoice(null)
              setShowQR(false)
            }
          }
        } catch (error) {
          console.error('Failed to check task status:', error)
        }
      }
    }, 10000)
    return () => clearInterval(interval)
  }, [fetchStats, fetchPendingTasks, pendingInvoice])

  const assignTask = async () => {
    if (!assignTarget || !assignAgent) return

    setAssigning(true)
    try {
      const res = await fetch(`/api/tasks/${assignTarget}/assign`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ worker_pubkey: assignAgent }),
      })
      
      if (res.ok) {
        setAssignTarget(null)
        setAssignAgent('')
        fetchTasks()
      }
    } catch (error) {
      console.error('Failed to assign task:', error)
    } finally {
      setAssigning(false)
    }
  }

  const getAgentDisplay = (pubkey: string | null) => {
    if (!pubkey) return null
    const agent = agents.get(pubkey)
    if (!agent) {
      return (
        <div className="flex items-center gap-2 text-text-secondary">
          <Bot className="w-4 h-4" />
          <span className="font-mono text-xs">{pubkey.slice(0, 12)}...</span>
        </div>
      )
    }
    
    return (
      <div className="flex items-center gap-2">
        {agent.avatar_url && (
          <img 
            src={agent.avatar_url} 
            alt={agent.name || 'Agent'}
            className="w-6 h-6 rounded border border-void-border"
            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
          />
        )}
        <span className="text-sm text-text-primary font-medium">
          {agent.name || pubkey.slice(0, 12)}
        </span>
        <div className="flex items-center gap-1">
          <Star className="w-3 h-3 text-amber-400 fill-amber-400" />
          <span className="text-xs text-text-secondary font-mono">
            {(agent.reputation_score * 100).toFixed(0)}%
          </span>
        </div>
      </div>
    )
  }

  const workerAgents = Array.from(agents.values()).filter(a => a.agent_type === 'worker')

  const createTask = async () => {
    if (!newTask.prompt.trim()) return

    setCreating(true)
    try {
      const res = await fetch('/api/tasks', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          ...newTask,
          buyer_pubkey: 'demo_user'
        }),
      })
      
      const data = await res.json()
      
      setPendingInvoice({
        id: data.task_id,
        prompt: newTask.prompt,
        bounty_sats: data.amount_sats,
        stake_sats: 0,
        status: 'pending_payment',
        buyer_pubkey: 'demo_user',
        worker_pubkey: null,
        escrow_invoice: data.escrow_invoice,
        payment_hash: data.payment_hash,
      })
      
      setShowCreate(false)
      setNewTask({ prompt: '', bounty_sats: 100, stake_sats: 0 })
      fetchTasks()
    } catch (error) {
      console.error('Failed to create task:', error)
    } finally {
      setCreating(false)
    }
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

  if (loading) {
    return (
      <div className="flex items-center justify-center py-24">
        <div className="relative">
          <div className="absolute inset-0 bg-amber-400/20 blur-lg rounded-full" />
          <Loader2 className="w-8 h-8 text-amber-400 animate-spin relative" />
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-4xl mx-auto animate-fade-in-up">
      <div className="flex items-center justify-between mb-10">
        <div>
          <div className="flex items-center gap-3 mb-1">
            <Zap className="w-6 h-6 text-amber-400" />
            <h2 className="text-3xl font-bold tracking-tight">Agent Dashboard</h2>
          </div>
          <p className="text-text-primary font-mono text-sm">Post tasks and manage your workflow</p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => { fetchStats(); fetchTasks(); fetchPendingTasks(); }}
            className="btn-secondary flex items-center gap-2"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
          <button
            onClick={() => setShowCreate(true)}
            className="btn-primary flex items-center gap-2"
          >
            <Plus className="w-5 h-5" />
            New Task
          </button>
        </div>
      </div>

      <div className="grid grid-cols-4 gap-4 mb-10">
        <div className="terminal-panel p-5 animate-fade-in-up stagger-1">
            <div className="flex items-center gap-2 text-text-primary mb-3 font-mono text-sm">
            <Zap className="w-4 h-4 text-amber-400" />
            Available
          </div>
          <div className="text-2xl font-bold font-mono">{(stats.available_sats || stats.balance_sats).toLocaleString()}</div>
          <div className="text-xs text-text-dim mt-1">sats to spend</div>
        </div>
        <div className="terminal-panel p-5 animate-fade-in-up stagger-2">
            <div className="flex items-center gap-2 text-text-primary mb-3 font-mono text-sm">
            <Clock className="w-4 h-4 text-cyan-400" />
            In Escrow
          </div>
          <div className="text-2xl font-bold font-mono">{(stats.escrow_sats || 0).toLocaleString()}</div>
          <div className="text-xs text-text-dim mt-1">sats in tasks</div>
        </div>
        <div className="terminal-panel p-5 animate-fade-in-up stagger-3">
            <div className="flex items-center gap-2 text-text-primary mb-3 font-mono text-sm">
            <Zap className="w-4 h-4 text-amber-400" />
            Wallet Balance
          </div>
          <div className="text-2xl font-bold font-mono">{stats.balance_sats.toLocaleString()}</div>
          <div className="text-xs text-text-dim mt-1">sats total</div>
        </div>
        <div className="terminal-panel p-5 animate-fade-in-up stagger-4">
            <div className="flex items-center gap-2 text-text-primary mb-3 font-mono text-sm">
            <TrendingUp className="w-4 h-4 text-green-success" />
            Total Earned
          </div>
          <div className="text-2xl font-bold amber-text font-mono">{stats.total_earned.toLocaleString()}</div>
          <div className="text-xs text-text-dim mt-1">sats lifetime</div>
        </div>
      </div>

      {showCreate && (
        <div className="fixed inset-0 bg-void/80 backdrop-blur-sm flex items-center justify-center z-50 animate-fade-in-up">
          <div className="terminal-panel p-6 w-full max-w-lg mx-4 glow-amber">
            <div className="flex items-center justify-between mb-6">
              <h3 className="text-xl font-bold amber-text">Create New Task</h3>
              <button
                onClick={() => setShowCreate(false)}
                className="text-text-secondary hover:text-text-primary transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2 font-mono text-text-primary">Task Description</label>
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
                  <label className="block text-sm font-medium mb-2 font-mono text-text-primary">Bounty (sats)</label>
                  <input
                    type="number"
                    value={newTask.bounty_sats}
                    onChange={(e) => setNewTask({ ...newTask, bounty_sats: parseInt(e.target.value) || 0 })}
                    className="terminal-input w-full"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium mb-2 font-mono text-text-primary">Stake Required</label>
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
          
          <p className="text-text-primary mb-4 font-mono text-sm">
            Pay <span className="text-amber-400">{pendingInvoice.bounty_sats}</span> sats to escrow. Task will be available once payment is confirmed.
          </p>
          
          <div className="bg-void-deep p-4 rounded-lg mb-4 font-mono text-xs text-text-secondary break-all border border-void-border">
            {pendingInvoice.escrow_invoice}
          </div>
          
          {showQR && pendingInvoice.escrow_invoice && (
            <div className="bg-white p-4 rounded-lg mb-4 inline-block mx-auto w-fit">
              <QRCode value={pendingInvoice.escrow_invoice} size={200} />
            </div>
          )}
          
          <div className="flex gap-3">
            <button
              onClick={() => copyInvoice(pendingInvoice.escrow_invoice || '')}
              className="btn-secondary flex items-center gap-2"
            >
              <Copy className="w-4 h-4" />
              Copy Invoice
            </button>
            <button
              onClick={() => setShowQR(!showQR)}
              className="btn-secondary flex items-center gap-2"
            >
              <QrCode className="w-4 h-4" />
              {showQR ? 'Hide QR' : 'Show QR'}
            </button>
          </div>
        </div>
      )}

      <div>
        <div className="flex items-center justify-between mb-6">
          <h3 className="text-xl font-bold flex items-center gap-2">
            <Clock className="w-5 h-5 text-cyan-400" />
            Your Tasks
          </h3>
          <div className="flex gap-2">
            <button
              onClick={() => setActiveTab('all')}
              className={`px-3 py-1.5 rounded-md text-sm font-mono transition-all ${
                activeTab === 'all'
                  ? 'bg-amber-400/10 text-amber-400 border border-amber-400/20'
                  : 'text-text-secondary hover:text-amber-300 border border-transparent'
              }`}
            >
              All
            </button>
            <button
              onClick={() => setActiveTab('created')}
              className={`px-3 py-1.5 rounded-md text-sm font-mono transition-all ${
                activeTab === 'created'
                  ? 'bg-amber-400/10 text-amber-400 border border-amber-400/20'
                  : 'text-text-secondary hover:text-amber-300 border border-transparent'
              }`}
            >
              Created
            </button>
            <button
              onClick={() => setActiveTab('assigned')}
              className={`px-3 py-1.5 rounded-md text-sm font-mono transition-all ${
                activeTab === 'assigned'
                  ? 'bg-amber-400/10 text-amber-400 border border-amber-400/20'
                  : 'text-text-secondary hover:text-amber-300 border border-transparent'
              }`}
            >
              Assigned
            </button>
          </div>
        </div>
        
        {tasks.length === 0 ? (
          <div className="terminal-panel p-12 text-center">
            <Zap className="w-12 h-12 mx-auto mb-4 text-text-dim opacity-50" />
            <p className="text-text-primary mb-2">No tasks yet</p>
            <p className="text-sm text-text-secondary font-mono">Create a task to get started</p>
          </div>
        ) : (
          <div className="space-y-4">
            {tasks
              .filter(task => {
                if (activeTab === 'created') return task.buyer_pubkey === 'demo_user'
                if (activeTab === 'assigned') return task.worker_pubkey !== null
                return true
              })
              .map((task) => (
                <div
                  key={task.id}
                  className="terminal-panel p-5 hover:border-cyan-400/30 transition-all duration-300"
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1 pr-6">
                      <p className="text-text-primary mb-3">{task.prompt}</p>
                      
                      <div className="flex items-center gap-4 text-sm font-mono mb-3 flex-wrap">
                        {getStatusBadge(task.status)}
                        <div className="flex items-center gap-1 text-text-secondary">
                          <Bot className="w-3 h-3" />
                          <span className="text-xs">Buyer:</span>
                          {getAgentDisplay(task.buyer_pubkey)}
                        </div>
                        {task.worker_pubkey && (
                          <div className="flex items-center gap-1 text-text-secondary">
                            <Bot className="w-3 h-3" />
                            <span className="text-xs">Worker:</span>
                            {getAgentDisplay(task.worker_pubkey)}
                          </div>
                        )}
                      </div>

                      {task.result && (
                        <div className="mt-2 p-3 bg-success/10 border border-success/20 rounded text-sm">
                          <div className="flex items-center gap-2 mb-1">
                            <CheckCircle2 className="w-4 h-4 text-success" />
                            <strong className="text-success">Result:</strong>
                          </div>
                          <p className="text-text-primary font-mono text-xs whitespace-pre-wrap">{task.result}</p>
                        </div>
                      )}
                      {task.failure_reason && task.status === 'failed' && (
                        <div className="mt-2 p-3 bg-alert/10 border border-alert/20 rounded text-sm">
                          <div className="flex items-center gap-2 mb-1">
                            <AlertCircle className="w-4 h-4 text-alert" />
                            <strong className="text-alert">Failed:</strong>
                          </div>
                          <p className="text-text-primary font-mono text-xs">{task.failure_reason}</p>
                        </div>
                      )}
                    </div>
                    <div className="text-right ml-4">
                      <div className="flex items-center gap-1 text-amber-400 mb-2">
                        <Zap className="w-4 h-4" />
                        <span className="font-bold font-mono">{task.bounty_sats}</span>
                      </div>
                      {getStatusBadge(task.status)}
                      {task.status === 'funded' && (
                        <button
                          onClick={() => setAssignTarget(task.id)}
                          className="mt-2 btn-cyan text-xs py-1 px-3 flex items-center gap-1"
                        >
                          <Send className="w-3 h-3" />
                          Assign
                        </button>
                      )}
                      {(task.result || task.failure_reason) && (
                        <button
                          onClick={() => setSelectedTask(task)}
                          className="mt-2 btn-secondary text-xs py-1 px-3 flex items-center gap-1"
                        >
                          <Eye className="w-3 h-3" />
                          View
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              ))}
          </div>
        )}
      </div>

      {assignTarget && (
        <div className="fixed inset-0 bg-void/80 backdrop-blur-sm flex items-center justify-center z-50 animate-fade-in-up">
          <div className="terminal-panel p-6 w-full max-w-md mx-4 glow-cyan">
            <div className="flex items-center justify-between mb-6">
              <h3 className="text-xl font-bold text-cyan-400">Assign Task to Worker</h3>
              <button
                onClick={() => { setAssignTarget(null); setAssignAgent(''); }}
                className="text-text-secondary hover:text-text-primary transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            
            <div className="space-y-4">
              {workerAgents.length > 0 && (
                <>
                  <label className="block text-sm font-medium mb-2 font-mono text-text-primary">Select Agent</label>
                  <div className="space-y-3">
                    {workerAgents.map(agent => (
                      <button
                        key={agent.pubkey}
                        onClick={() => setAssignAgent(agent.pubkey)}
                        className={`w-full p-4 rounded-lg border transition-all text-left ${
                          assignAgent === agent.pubkey
                            ? 'border-cyan-400 bg-cyan-400/10'
                            : 'border-void-border hover:border-cyan-400/30'
                        }`}
                      >
                        <div className="flex items-center gap-3">
                          {agent.avatar_url && (
                            <img 
                              src={agent.avatar_url} 
                              alt={agent.name || 'Agent'}
                              className="w-10 h-10 rounded border border-void-border"
                              onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
                            />
                          )}
                          <div className="flex-1">
                            <div className="flex items-center gap-2">
                              <span className="text-text-primary font-medium">{agent.name || agent.pubkey.slice(0, 12)}</span>
                              {agent.is_active && (
                                <span className="status-badge status-completed text-xs">Active</span>
                              )}
                            </div>
                            <div className="flex items-center gap-1 mt-1">
                              <Star className="w-3 h-3 text-amber-400 fill-amber-400" />
                              <span className="text-xs text-text-secondary font-mono">
                                {(agent.reputation_score * 100).toFixed(0)}% · {agent.total_tasks} tasks
                              </span>
                            </div>
                          </div>
                          <ChevronDown className={`w-5 h-5 transition-transform ${assignAgent === agent.pubkey ? 'rotate-180' : ''}`} />
                        </div>
                      </button>
                    ))}
                  </div>
                  
                  <div className="flex items-center gap-3 py-2">
                    <div className="flex-1 h-px bg-void-border" />
                    <span className="text-xs text-text-dim font-mono">OR</span>
                    <div className="flex-1 h-px bg-void-border" />
                  </div>
                </>
              )}
              
              <div>
                <label className="block text-sm font-medium mb-2 font-mono text-text-primary">Enter Worker Pubkey</label>
                <input
                  type="text"
                  value={assignAgent}
                  onChange={(e) => setAssignAgent(e.target.value)}
                  placeholder="e.g., worker1"
                  className="terminal-input w-full"
                />
                <p className="text-xs text-text-dim mt-1 font-mono">Pubkey of the CLI worker (e.g., worker1)</p>
              </div>
            </div>
            
            <div className="flex gap-3 mt-6">
              <button
                onClick={() => { setAssignTarget(null); setAssignAgent(''); }}
                className="btn-secondary flex-1"
              >
                Cancel
              </button>
              <button
                onClick={assignTask}
                disabled={assigning || !assignAgent.trim()}
                className="btn-cyan flex-1 flex items-center justify-center gap-2"
              >
                {assigning ? <Loader2 className="w-4 h-4 animate-spin" /> : <Send className="w-4 h-4" />}
                Assign
              </button>
            </div>
          </div>
        </div>
      )}

      {selectedTask && (
        <div className="fixed inset-0 bg-void/80 backdrop-blur-sm flex items-center justify-center z-50 animate-fade-in-up">
          <div className="terminal-panel p-6 w-full max-w-2xl mx-4 max-h-[80vh] overflow-y-auto glow-amber">
            <div className="flex items-center justify-between mb-6">
              <div>
                <h3 className="text-xl font-bold amber-text">Task Submission</h3>
                <p className="text-text-secondary font-mono text-sm">{selectedTask.id}</p>
              </div>
              <button
                onClick={() => setSelectedTask(null)}
                className="text-text-secondary hover:text-text-primary transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <div className="flex items-center gap-4 mb-6">
              {getStatusBadge(selectedTask.status)}
              <div className="flex items-center gap-1 text-amber-400">
                <Zap className="w-5 h-5" />
                <span className="text-xl font-bold font-mono">{selectedTask.bounty_sats}</span>
                <span className="text-sm text-text-secondary">sats</span>
              </div>
            </div>

            <div className="mb-6">
              <h4 className="text-sm font-medium text-text-primary mb-2">Prompt</h4>
              <p className="text-text-secondary font-mono bg-void-deep p-4 rounded-lg border border-void-border">
                {selectedTask.prompt}
              </p>
            </div>

            {selectedTask.worker_pubkey && (
              <div className="mb-6">
                <h4 className="text-sm font-medium text-text-primary mb-2">Worker Agent</h4>
                <div className="flex items-center gap-3">
                  {getAgentDisplay(selectedTask.worker_pubkey)}
                </div>
              </div>
            )}

            {selectedTask.result && (
              <div className="mb-6">
                <h4 className="text-sm font-medium text-text-primary mb-2">Submission Result</h4>
                <div className="bg-success/10 border border-success/20 rounded-lg p-4">
                  <p className="text-text-primary font-mono text-sm whitespace-pre-wrap">{selectedTask.result}</p>
                </div>
              </div>
            )}

            {selectedTask.failure_reason && selectedTask.status === 'failed' && (
              <div className="mb-6">
                <h4 className="text-sm font-medium text-text-primary mb-2">Failure Reason</h4>
                <div className="bg-alert/10 border border-alert/20 rounded-lg p-4">
                  <p className="text-alert font-mono text-sm">{selectedTask.failure_reason}</p>
                </div>
              </div>
            )}

            <div className="flex items-center gap-2 text-xs text-text-dim font-mono">
              <Clock className="w-3 h-3" />
              Created: {selectedTask.created_at ? new Date(selectedTask.created_at).toLocaleString() : 'Unknown'}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
