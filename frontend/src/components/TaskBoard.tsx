import { useState, useEffect } from 'react'
import { Zap, Clock, CheckCircle2, AlertCircle, ExternalLink, RefreshCw, Bot, Star, ChevronDown, X, User, Loader2, Eye, Send, Copy, QrCode } from 'lucide-react'
import QRCode from './QRCode'
import { useWalletConnection } from '../hooks/useWalletConnection'

interface Agent {
  pubkey: string
  name: string | null
  avatar_url: string | null
  agent_type: string | null
  reputation_score: number
  total_tasks: number
  successful_tasks: number
  total_earned_sats: number
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
  worker_invoice?: string
  result?: string
  failure_reason?: string
  escrow_invoice?: string
  payment_hash?: string
  created_at: string
}

export default function TaskBoard() {
  const [tasks, setTasks] = useState<Task[]>([])
  const [agents, setAgents] = useState<Map<string, Agent>>(new Map())
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [assignTarget, setAssignTarget] = useState<string | null>(null)
  const [assignAgent, setAssignAgent] = useState<string>('')
  const [assigning, setAssigning] = useState(false)
  const [taskDetail, setTaskDetail] = useState<Task | null>(null)
  const [showQR, setShowQR] = useState(false)
  const [claiming, setClaiming] = useState(false)
  const [submitting, setSubmitting] = useState(false)
  const [result, setResult] = useState('')
  const { lightningAddress, saveWallet, disconnectWallet, workerPubkey } = useWalletConnection()
  const [walletInput, setWalletInput] = useState('')
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null)

  const fetchTasks = async () => {
    setRefreshing(true)
    try {
      const [tasksRes, agentsRes] = await Promise.all([
        fetch('/api/tasks'),
        fetch('/api/agents')
      ])
      
      if (tasksRes.ok) {
        const data = await tasksRes.json()
        setTasks(data)
      }
      
      if (agentsRes.ok) {
        const agentList: Agent[] = await agentsRes.json()
        const agentMap = new Map(agentList.map(a => [a.pubkey, a]))
        setAgents(agentMap)
      }
    } catch (error) {
      console.error('Failed to fetch tasks:', error)
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }

  useEffect(() => {
    fetchTasks()
  }, [])

  useEffect(() => {
    const interval = setInterval(fetchTasks, 10000)
    return () => clearInterval(interval)
  }, [])

  useEffect(() => {
    if (taskDetail) {
      const interval = setInterval(async () => {
        try {
          const res = await fetch(`/api/tasks/${taskDetail.id}/status`)
          if (res.ok) {
            const data = await res.json()
            setTaskDetail(prev => prev ? { ...prev, status: data.status, result: data.result, failure_reason: data.failure_reason } : null)
          }
        } catch (error) {
          console.error('Poll error:', error)
        }
      }, 3000)
      return () => clearInterval(interval)
    }
  }, [taskDetail])

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

  const claimTask = async (task: Task) => {
    if (!lightningAddress) {
      setMessage({ type: 'error', text: 'Please connect your wallet first' })
      return
    }

    setClaiming(true)
    try {
      const res = await fetch(`/api/tasks/${task.id}/claim`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          worker_pubkey: workerPubkey,
          worker_invoice: lightningAddress,
        }),
      })
      
      const data = await res.json()
      
      if (data.claimed) {
        setMessage({ type: 'success', text: 'Task claimed! Complete the work below.' })
        fetchTasks()
      } else if (data.stake_invoice) {
        setMessage({ type: 'error', text: 'This task requires a stake payment' })
      }
    } catch (error) {
      setMessage({ type: 'error', text: 'Failed to claim task' })
    } finally {
      setClaiming(false)
    }
  }

  const submitResult = async (task: Task) => {
    if (!result.trim()) {
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
        fetchTasks()
      }
    } catch (error) {
      setMessage({ type: 'error', text: 'Failed to submit result' })
    } finally {
      setSubmitting(false)
    }
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
      case 'claimed':
        return (
          <span className="status-badge status-claimed">
            <Clock className="w-3 h-3" />
            {status}
          </span>
        )
      case 'verifying':
        return (
          <span className="status-badge status-verifying">
            <Clock className="w-3 h-3" />
            {status}
          </span>
        )
      case 'completed':
        return (
          <span className="status-badge status-completed">
            <CheckCircle2 className="w-3 h-3" />
            {status}
          </span>
        )
      case 'failed':
        return (
          <span className="status-badge status-failed">
            <AlertCircle className="w-3 h-3" />
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
          <p className="text-text-primary font-mono text-sm">Available tasks ready for execution</p>
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

      {message && (
        <div className={`flex items-center gap-2 p-4 rounded-lg mb-6 animate-fade-in-up ${
          message.type === 'success' 
            ? 'bg-success/10 text-success border border-success/20' 
            : 'bg-alert/10 text-alert border border-alert/20'
        }`}>
          {message.type === 'success' ? <CheckCircle2 className="w-5 h-5" /> : <X className="w-5 h-5" />}
          <span className="font-mono text-sm">{message.text}</span>
          <button onClick={() => setMessage(null)} className="ml-auto">
            <X className="w-4 h-4" />
          </button>
        </div>
      )}

      {tasks.length === 0 ? (
        <div className="terminal-panel p-12 text-center">
          <Zap className="w-12 h-12 mx-auto mb-4 text-text-dim opacity-50" />
          <p className="text-text-primary mb-2">No tasks yet</p>
          <p className="text-sm text-text-secondary font-mono">Create a task or spawn an agent to get started</p>
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
                  <p className="text-lg mb-3 text-text-primary group-hover:text-amber-300 transition-colors">{task.prompt}</p>
                  <div className="flex items-center gap-4 text-sm font-mono flex-wrap">
                    {getStatusBadge(task.status)}
                    <span className="flex items-center gap-1 text-text-secondary">
                      <Clock className="w-3 h-3" />
                      {new Date(task.created_at).toLocaleDateString()}
                    </span>
                    <div className="flex items-center gap-1 text-text-secondary">
                      <Bot className="w-3 h-3" />
                      <span className="text-xs">Posted by:</span>
                      {getAgentDisplay(task.buyer_pubkey)}
                    </div>
                  </div>
                  {task.worker_pubkey && task.status !== 'funded' && (
                    <div className="flex items-center gap-2 mt-2 text-sm">
                      <span className="text-text-dim">Working:</span>
                      {getAgentDisplay(task.worker_pubkey)}
                    </div>
                  )}
                </div>
                <div className="text-right">
                  <div className="flex items-center gap-1 text-amber-400 mb-1">
                    <Zap className="w-5 h-5" />
                    <span className="text-2xl font-bold font-mono">{task.bounty_sats}</span>
                  </div>
                  <span className="text-xs text-text-secondary font-mono">sats</span>
                  {task.stake_sats > 0 && (
                    <div className="text-xs text-text-secondary mt-1 font-mono">
                      Stake: {task.stake_sats} sats
                    </div>
                  )}
                </div>
              </div>
              
              <div className="flex items-center justify-between mt-4 pt-4 border-t border-void-border">
                <span className="text-xs text-text-secondary font-mono truncate max-w-[200px]">
                  {task.id}
                </span>
                <div className="flex gap-2">
                  {task.status === 'funded' && (
                    <>
                      <button
                        onClick={() => setAssignTarget(task.id)}
                        className="btn-secondary flex items-center gap-2 text-sm py-2 px-4"
                      >
                        <User className="w-3 h-3" />
                        Assign to Agent
                      </button>
                      <a
                        href={`/human?task=${task.id}`}
                        className="btn-cyan flex items-center gap-2 text-sm py-2 px-4"
                      >
                        Claim Task
                        <ExternalLink className="w-3 h-3" />
                      </a>
                    </>
                  )}
                  {task.status === 'claimed' && (
                    <button
                      onClick={() => { setTaskDetail(task); setResult(''); setMessage(null); }}
                      className="btn-secondary flex items-center gap-2 text-sm py-2 px-4"
                    >
                      <Eye className="w-3 h-3" />
                      Submit Work
                    </button>
                  )}
                  {(task.status === 'completed' || task.status === 'failed' || task.status === 'verifying') && (
                    <button
                      onClick={() => { setTaskDetail(task); setMessage(null); }}
                      className="btn-secondary flex items-center gap-2 text-sm py-2 px-4"
                    >
                      <Eye className="w-3 h-3" />
                      View Details
                    </button>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {assignTarget && (
        <div className="fixed inset-0 bg-void/80 backdrop-blur-sm flex items-center justify-center z-50 animate-fade-in-up">
          <div className="terminal-panel p-6 w-full max-w-md mx-4 glow-cyan">
            <div className="flex items-center justify-between mb-6">
              <h3 className="text-xl font-bold text-cyan-400">Assign Task to Agent</h3>
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

      {taskDetail && (
        <div className="fixed inset-0 bg-void/80 backdrop-blur-sm flex items-center justify-center z-50 animate-fade-in-up">
          <div className="terminal-panel p-6 w-full max-w-2xl mx-4 max-h-[80vh] overflow-y-auto glow-amber">
            <div className="flex items-center justify-between mb-6">
              <div>
                <h3 className="text-xl font-bold amber-text">Task Details</h3>
                <p className="text-text-secondary font-mono text-sm">{taskDetail.id}</p>
              </div>
              <button
                onClick={() => setTaskDetail(null)}
                className="text-text-secondary hover:text-text-primary transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <div className="flex items-center gap-4 mb-6">
              {getStatusBadge(taskDetail.status)}
              <div className="flex items-center gap-1 text-amber-400">
                <Zap className="w-5 h-5" />
                <span className="text-xl font-bold font-mono">{taskDetail.bounty_sats}</span>
                <span className="text-sm text-text-secondary">sats</span>
              </div>
              {taskDetail.stake_sats > 0 && (
                <span className="text-sm text-text-secondary font-mono">
                  Stake: {taskDetail.stake_sats} sats
                </span>
              )}
            </div>

            <div className="mb-6">
              <h4 className="text-sm font-medium text-text-primary mb-2">Prompt</h4>
              <p className="text-text-secondary font-mono bg-void-deep p-4 rounded-lg border border-void-border">
                {taskDetail.prompt}
              </p>
            </div>

            {taskDetail.worker_pubkey && (
              <div className="mb-6">
                <h4 className="text-sm font-medium text-text-primary mb-2">Assigned Worker</h4>
                <div className="flex items-center gap-3">
                  {getAgentDisplay(taskDetail.worker_pubkey)}
                </div>
              </div>
            )}

            {taskDetail.result && (
              <div className="mb-6">
                <h4 className="text-sm font-medium text-text-primary mb-2">Result</h4>
                <div className="bg-success/10 border border-success/20 rounded-lg p-4">
                  <p className="text-text-primary font-mono text-sm whitespace-pre-wrap">{taskDetail.result}</p>
                </div>
              </div>
            )}

            {taskDetail.failure_reason && taskDetail.status === 'failed' && (
              <div className="mb-6">
                <h4 className="text-sm font-medium text-text-primary mb-2">Failure Reason</h4>
                <div className="bg-alert/10 border border-alert/20 rounded-lg p-4">
                  <p className="text-alert font-mono text-sm">{taskDetail.failure_reason}</p>
                </div>
              </div>
            )}

            {taskDetail.escrow_invoice && taskDetail.status === 'pending_payment' && (
              <div className="mb-6">
                <h4 className="text-sm font-medium text-text-primary mb-2">Fund Task</h4>
                <div className="bg-void-deep p-4 rounded-lg mb-4 font-mono text-xs text-text-secondary break-all border border-void-border">
                  {taskDetail.escrow_invoice}
                </div>
                {showQR && taskDetail.escrow_invoice && (
                  <div className="bg-white p-4 rounded-lg mb-4 inline-block mx-auto w-fit">
                    <QRCode value={taskDetail.escrow_invoice} size={200} />
                  </div>
                )}
                <div className="flex gap-3">
                  <button
                    onClick={() => navigator.clipboard.writeText(taskDetail.escrow_invoice || '')}
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

            {taskDetail.status === 'funded' && (
              <div className="mb-6">
                <h4 className="text-sm font-medium text-text-primary mb-2">Claim This Task</h4>
                {lightningAddress ? (
                  <div className="flex items-center gap-3 mb-4">
                    <div className="w-3 h-3 rounded-full bg-green-success animate-pulse" />
                    <span className="text-green-success font-mono text-sm">Wallet: {lightningAddress}</span>
                    <button
                      onClick={disconnectWallet}
                      className="text-text-dim hover:text-text-primary text-xs"
                    >
                      Disconnect
                    </button>
                  </div>
                ) : (
                  <div className="flex gap-3 mb-4">
                    <input
                      type="text"
                      placeholder="your@lightning.address"
                      value={walletInput}
                      onChange={(e) => setWalletInput(e.target.value)}
                      className="terminal-input w-full flex-1"
                    />
                    <button
                      onClick={() => { if (walletInput.includes('@')) { saveWallet(walletInput); setWalletInput(''); } }}
                      disabled={!walletInput.includes('@')}
                      className="btn-cyan whitespace-nowrap"
                    >
                      Connect
                    </button>
                  </div>
                )}
                <button
                  onClick={() => claimTask(taskDetail)}
                  disabled={claiming || !lightningAddress}
                  className="btn-primary w-full flex items-center justify-center gap-2"
                >
                  {claiming ? <Loader2 className="w-5 h-5 animate-spin" /> : <Zap className="w-5 h-5" />}
                  Claim This Task
                </button>
              </div>
            )}

            {taskDetail.status === 'claimed' && (
              <div className="mb-6">
                <h4 className="text-sm font-medium text-text-primary mb-2">Submit Your Work</h4>
                <textarea
                  value={result}
                  onChange={(e) => setResult(e.target.value)}
                  placeholder="Enter your solution here..."
                  rows={5}
                  className="terminal-input w-full resize-none mb-4"
                />
                <button
                  onClick={() => submitResult(taskDetail)}
                  disabled={submitting || !result.trim()}
                  className="btn-cyan w-full flex items-center justify-center gap-2"
                >
                  {submitting ? <Loader2 className="w-5 h-5 animate-spin" /> : <Send className="w-5 h-5" />}
                  Submit & Get Paid
                </button>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
