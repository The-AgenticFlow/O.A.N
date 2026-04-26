import { useState, useEffect, useCallback } from 'react'
import { Bot, Plus, Play, Square, Loader2, Star, TrendingUp, X, RefreshCw, Zap } from 'lucide-react'

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
  created_at: string
}

export default function AgentManager() {
  const [agents, setAgents] = useState<Agent[]>([])
  const [loading, setLoading] = useState(true)
  const [showCreate, setShowCreate] = useState(false)
  const [creating, setCreating] = useState(false)
  const [newAgent, setNewAgent] = useState({ name: '', agent_type: 'worker' })
  const [spawning, setSpawning] = useState<string | null>(null)

  const fetchAgents = useCallback(async () => {
    try {
      const res = await fetch('/api/agents')
      if (res.ok) {
        const data = await res.json()
        setAgents(data)
      }
    } catch (error) {
      console.error('Failed to fetch agents:', error)
    }
  }, [])

  useEffect(() => {
    setLoading(true)
    fetchAgents().then(() => setLoading(false))
  }, [fetchAgents])

  useEffect(() => {
    const interval = setInterval(fetchAgents, 5000)
    return () => clearInterval(interval)
  }, [fetchAgents])

  const createAgent = async () => {
    if (!newAgent.name.trim()) return

    setCreating(true)
    try {
      const res = await fetch('/api/agents', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(newAgent),
      })
      
      if (res.ok) {
        setShowCreate(false)
        setNewAgent({ name: '', agent_type: 'worker' })
        fetchAgents()
      }
    } catch (error) {
      console.error('Failed to create agent:', error)
    } finally {
      setCreating(false)
    }
  }

  const spawnAgent = async (agentId: string) => {
    setSpawning(agentId)
    try {
      const res = await fetch('/api/agents/spawn', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ agent_id: agentId }),
      })
      
      if (res.ok) {
        fetchAgents()
      }
    } catch (error) {
      console.error('Failed to spawn agent:', error)
    } finally {
      setSpawning(null)
    }
  }

  const stopAgent = async (agentId: string) => {
    try {
      const res = await fetch(`/api/agents/${agentId}/stop`, {
        method: 'POST',
      })
      
      if (res.ok) {
        fetchAgents()
      }
    } catch (error) {
      console.error('Failed to stop agent:', error)
    }
  }

  const getReputationStars = (score: number) => {
    const stars = Math.round(score * 5)
    return Array(5).fill(0).map((_, i) => (
      <Star
        key={i}
        className={`w-4 h-4 ${i < stars ? 'text-amber-400 fill-amber-400' : 'text-text-dim'}`}
      />
    ))
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
            <Bot className="w-6 h-6 text-amber-400" />
            <h2 className="text-3xl font-bold tracking-tight">Agent Manager</h2>
          </div>
          <p className="text-text-primary font-mono text-sm">Spawn and manage autonomous agents</p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={fetchAgents}
            className="btn-secondary flex items-center gap-2"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
          <button
            onClick={() => setShowCreate(true)}
            className="btn-primary flex items-center gap-2"
          >
            <Plus className="w-5 h-5" />
            New Agent
          </button>
        </div>
      </div>

      {showCreate && (
        <div className="fixed inset-0 bg-void/80 backdrop-blur-sm flex items-center justify-center z-50 animate-fade-in-up">
          <div className="terminal-panel p-6 w-full max-w-lg mx-4 glow-amber">
            <div className="flex items-center justify-between mb-6">
              <h3 className="text-xl font-bold amber-text">Create New Agent</h3>
              <button
                onClick={() => setShowCreate(false)}
                className="text-text-secondary hover:text-text-primary transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2 font-mono text-text-primary">Agent Name</label>
                <input
                  type="text"
                  value={newAgent.name}
                  onChange={(e) => setNewAgent({ ...newAgent, name: e.target.value })}
                  placeholder="e.g., CodeBot, DataMiner"
                  className="terminal-input w-full"
                />
              </div>
              
              <div>
                <label className="block text-sm font-medium mb-2 font-mono text-text-primary">Agent Type</label>
                <select
                  value={newAgent.agent_type}
                  onChange={(e) => setNewAgent({ ...newAgent, agent_type: e.target.value })}
                  className="terminal-input w-full"
                >
                  <option value="worker">Worker (completes tasks)</option>
                  <option value="buyer">Buyer (posts tasks)</option>
                </select>
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
                onClick={createAgent}
                disabled={creating || !newAgent.name.trim()}
                className="btn-primary flex-1 flex items-center justify-center gap-2"
              >
                {creating ? <Loader2 className="w-5 h-5 animate-spin" /> : <Plus className="w-5 h-5" />}
                Create Agent
              </button>
            </div>
          </div>
        </div>
      )}

      <div>
        <h3 className="text-xl font-bold mb-6 flex items-center gap-2">
          <Bot className="w-5 h-5 text-cyan-400" />
          Your Agents
        </h3>
        
        {agents.length === 0 ? (
          <div className="terminal-panel p-12 text-center">
            <Bot className="w-12 h-12 mx-auto mb-4 text-text-dim opacity-50" />
            <p className="text-text-primary mb-2">No agents yet</p>
            <p className="text-sm text-text-secondary font-mono">Create an agent to get started</p>
          </div>
        ) : (
          <div className="grid gap-4">
            {agents.map((agent) => (
              <div
                key={agent.pubkey}
                className={`terminal-panel p-5 transition-all duration-300 ${
                  agent.is_active ? 'border-green-500/30 glow-green' : ''
                }`}
              >
                <div className="flex items-start gap-4">
                  {agent.avatar_url && (
                    <div className="w-16 h-16 rounded-lg overflow-hidden bg-void-deep border border-void-border flex-shrink-0">
                      <img
                        src={agent.avatar_url}
                        alt={agent.name || 'Agent'}
                        className="w-full h-full object-cover"
                        onError={(e) => {
                          (e.target as HTMLImageElement).style.display = 'none'
                        }}
                      />
                    </div>
                  )}
                  
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-3 mb-2">
                      <h4 className="text-lg font-bold text-text-primary">
                        {agent.name || `Agent ${agent.pubkey.slice(0, 8)}`}
                      </h4>
                      <span className={`status-badge ${
                        agent.is_active ? 'status-completed' : 'status-default'
                      }`}>
                        {agent.is_active ? 'Running' : 'Stopped'}
                      </span>
                      <span className="text-xs text-text-dim font-mono">
                        {agent.agent_type || 'worker'}
                      </span>
                    </div>
                    
                    <div className="flex items-center gap-1 mb-3">
                      {getReputationStars(agent.reputation_score)}
                      <span className="text-xs text-text-secondary ml-2 font-mono">
                        {agent.reputation_score.toFixed(2)}
                      </span>
                    </div>
                    
                    <div className="grid grid-cols-3 gap-4 text-sm">
                      <div>
                        <span className="text-text-dim">Tasks: </span>
                        <span className="font-mono">{agent.total_tasks}</span>
                        <span className="text-green-success ml-1">({agent.successful_tasks})</span>
                      </div>
                      <div className="flex items-center gap-1">
                        <Zap className="w-3 h-3 text-amber-400" />
                        <span className="font-mono">{agent.total_earned_sats.toLocaleString()}</span>
                        <span className="text-text-dim">sats</span>
                      </div>
                      <div className="flex items-center gap-1">
                        <TrendingUp className="w-3 h-3 text-cyan-400" />
                        <span className="font-mono text-xs truncate">{agent.pubkey.slice(0, 16)}...</span>
                      </div>
                    </div>
                  </div>
                  
                  <div className="flex gap-2">
                    {agent.is_active ? (
                      <button
                        onClick={() => stopAgent(agent.pubkey)}
                        className="btn-secondary flex items-center gap-2 text-alert border-alert/30 hover:bg-alert/10"
                      >
                        <Square className="w-4 h-4" />
                        Stop
                      </button>
                    ) : (
                      <button
                        onClick={() => spawnAgent(agent.pubkey)}
                        disabled={spawning === agent.pubkey}
                        className="btn-primary flex items-center gap-2"
                      >
                        {spawning === agent.pubkey ? (
                          <Loader2 className="w-4 h-4 animate-spin" />
                        ) : (
                          <Play className="w-4 h-4" />
                        )}
                        Spawn
                      </button>
                    )}
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
