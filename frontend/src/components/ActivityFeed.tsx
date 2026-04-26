import { useState, useEffect, useCallback } from 'react'
import { Activity, Clock, Bot, Zap, CheckCircle, XCircle, AlertCircle, Loader2, RefreshCw } from 'lucide-react'

interface ActivityEntry {
  id: string
  agent_pubkey: string
  agent_name: string | null
  event_type: string
  event_data: string | null
  task_id: string | null
  created_at: string
}

const EVENT_ICONS: Record<string, typeof Activity> = {
  agent_spawned: Bot,
  agent_stopped: XCircle,
  task_created: Zap,
  task_funded: Zap,
  task_found: AlertCircle,
  task_claimed: CheckCircle,
  task_assigned: CheckCircle,
  task_completed: CheckCircle,
  task_failed: XCircle,
  task_verified: CheckCircle,
  default: Activity,
}

const EVENT_COLORS: Record<string, string> = {
  agent_spawned: 'text-purple-400',
  agent_stopped: 'text-alert',
  task_created: 'text-amber-400',
  task_funded: 'text-green-success',
  task_found: 'text-cyan-400',
  task_claimed: 'text-cyan-400',
  task_assigned: 'text-cyan-400',
  task_completed: 'text-green-success',
  task_failed: 'text-alert',
  task_verified: 'text-green-success',
  default: 'text-text-secondary',
}

const EVENT_LABELS: Record<string, string> = {
  agent_spawned: 'Agent Spawned',
  agent_stopped: 'Agent Stopped',
  task_created: 'Task Created',
  task_funded: 'Task Funded',
  task_found: 'Task Found',
  task_claimed: 'Task Claimed',
  task_assigned: 'Task Assigned',
  task_completed: 'Task Completed',
  task_failed: 'Task Failed',
  task_verified: 'Task Verified',
  default: 'Activity',
}

function formatTimestamp(ts: string): string {
  const date = new Date(ts)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffSec = Math.floor(diffMs / 1000)
  const diffMin = Math.floor(diffSec / 60)
  const diffHr = Math.floor(diffMin / 60)

  if (diffSec < 10) return 'Just now'
  if (diffSec < 60) return `${diffSec}s ago`
  if (diffMin < 60) return `${diffMin}m ago`
  if (diffHr < 24) return `${diffHr}h ago`
  return date.toLocaleDateString()
}

export default function ActivityFeed() {
  const [activities, setActivities] = useState<ActivityEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [filter, setFilter] = useState<string>('all')

  const fetchActivities = useCallback(async () => {
    try {
      const res = await fetch('/api/activity?limit=100')
      if (res.ok) {
        const data = await res.json()
        setActivities(data)
      }
    } catch (error) {
      console.error('Failed to fetch activities:', error)
    }
  }, [])

  useEffect(() => {
    const init = async () => {
      setLoading(true)
      await fetchActivities()
      setLoading(false)
    }
    init()
  }, [fetchActivities])

  useEffect(() => {
    const interval = setInterval(fetchActivities, 5000)
    return () => clearInterval(interval)
  }, [fetchActivities])

  const filteredActivities = filter === 'all'
    ? activities
    : activities.filter(a => a.event_type.includes(filter))

  const uniqueEventTypes = Array.from(new Set(activities.map(a => {
    const base = a.event_type.replace('agent_', '').replace('task_', '')
    return base
  })))

  if (loading) {
    return (
      <div className="flex items-center justify-center py-24">
        <div className="relative">
          <div className="absolute inset-0 bg-cyan-400/20 blur-lg rounded-full" />
          <Loader2 className="w-8 h-8 text-cyan-400 animate-spin relative" />
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-4xl mx-auto animate-fade-in-up">
      <div className="flex items-center justify-between mb-10">
        <div>
          <div className="flex items-center gap-3 mb-1">
            <Activity className="w-6 h-6 text-cyan-400" />
            <h2 className="text-3xl font-bold tracking-tight">Activity Feed</h2>
          </div>
          <p className="text-text-primary font-mono text-sm">Real-time agent and task events</p>
        </div>
        <button
          onClick={fetchActivities}
          className="btn-secondary flex items-center gap-2"
        >
          <RefreshCw className="w-4 h-4" />
        </button>
      </div>

      <div className="flex gap-2 mb-6 overflow-x-auto pb-2">
        <button
          onClick={() => setFilter('all')}
          className={`px-3 py-1.5 rounded-md text-sm font-mono transition-all whitespace-nowrap ${
            filter === 'all'
              ? 'bg-cyan-400/10 text-cyan-400 border border-cyan-400/20'
              : 'text-text-secondary hover:text-cyan-300 border border-transparent'
          }`}
        >
          All
        </button>
        {uniqueEventTypes.map(type => (
          <button
            key={type}
            onClick={() => setFilter(type)}
            className={`px-3 py-1.5 rounded-md text-sm font-mono transition-all whitespace-nowrap ${
              filter === type
                ? 'bg-cyan-400/10 text-cyan-400 border border-cyan-400/20'
                : 'text-text-secondary hover:text-cyan-300 border border-transparent'
            }`}
          >
            {type}
          </button>
        ))}
      </div>

      {filteredActivities.length === 0 ? (
        <div className="terminal-panel p-12 text-center">
          <Activity className="w-12 h-12 mx-auto mb-4 text-text-dim opacity-50" />
          <p className="text-text-primary mb-2">No activity yet</p>
          <p className="text-sm text-text-secondary font-mono">Spawn agents to see activity here</p>
        </div>
      ) : (
        <div className="space-y-3">
          {filteredActivities.map((entry, idx) => {
            const Icon = EVENT_ICONS[entry.event_type] || EVENT_ICONS.default
            const colorClass = EVENT_COLORS[entry.event_type] || EVENT_COLORS.default
            const label = EVENT_LABELS[entry.event_type] || EVENT_LABELS.default

            return (
              <div
                key={entry.id}
                className="terminal-panel p-4 hover:border-cyan-400/30 transition-all duration-300 animate-fade-in-up"
                style={{ animationDelay: `${idx * 0.05}s` }}
              >
                <div className="flex items-start gap-4">
                  <div className={`mt-1 ${colorClass}`}>
                    <Icon className="w-5 h-5" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <span className={`font-mono text-sm font-medium ${colorClass}`}>
                        {label}
                      </span>
                      {entry.agent_name && (
                        <span className="text-text-dim font-mono text-xs">
                          by {entry.agent_name}
                        </span>
                      )}
                    </div>
                    {entry.event_data && (
                      <p className="text-text-secondary text-sm font-mono break-words">
                        {entry.event_data}
                      </p>
                    )}
                    {entry.task_id && (
                      <p className="text-text-dim text-xs font-mono mt-1">
                        Task: {entry.task_id.slice(0, 8)}...
                      </p>
                    )}
                  </div>
                  <div className="text-right ml-4 shrink-0">
                    <Clock className="w-4 h-4 text-text-dim mx-auto mb-1" />
                    <span className="text-xs text-text-dim font-mono">
                      {formatTimestamp(entry.created_at)}
                    </span>
                  </div>
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
