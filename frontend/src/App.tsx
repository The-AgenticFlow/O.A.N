import { Outlet, NavLink } from 'react-router-dom'
import { Zap, User, Bot, Terminal } from 'lucide-react'

export default function App() {
  return (
    <div className="min-h-screen text-void-surface font-body relative">
      <div className="fixed inset-0 pointer-events-none z-0">
        <div className="absolute inset-0 bg-void-deep" />
        <div 
          className="absolute inset-0 opacity-[0.03]"
          style={{
            backgroundImage: `repeating-linear-gradient(
              0deg,
              transparent,
              transparent 2px,
              rgba(0, 229, 255, 0.5) 2px,
              rgba(0, 229, 255, 0.5) 4px
            )`
          }}
        />
        <div 
          className="absolute inset-0"
          style={{
            backgroundImage: `
              radial-gradient(ellipse at 20% 50%, rgba(255, 184, 0, 0.03) 0%, transparent 50%),
              radial-gradient(ellipse at 80% 20%, rgba(0, 229, 255, 0.02) 0%, transparent 50%),
              radial-gradient(ellipse at 50% 80%, rgba(255, 61, 61, 0.015) 0%, transparent 50%)
            `
          }}
        />
      </div>
      
      <header className="border-b border-void-border bg-void-surface/80 backdrop-blur-md sticky top-0 z-50">
        <div className="container mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="relative">
                <div className="absolute inset-0 bg-amber-400/20 blur-lg rounded-full" />
                <Terminal className="w-8 h-8 text-amber-400 relative" />
              </div>
              <div>
                <h1 className="text-2xl font-bold amber-text tracking-tight">OAN</h1>
                <span className="text-[10px] font-mono text-cyan-400/70 tracking-widest uppercase">Agent Marketplace</span>
              </div>
            </div>
            
            <nav className="flex gap-1 bg-void-deep/50 p-1 rounded-lg border border-void-border">
              <NavLink 
                to="/" 
                className={({ isActive }) => 
                  `flex items-center gap-2 px-4 py-2 rounded-md transition-all duration-200 font-medium text-sm ${
                    isActive 
                      ? 'bg-amber-400/10 text-amber-400 border border-amber-400/20' 
                      : 'text-void-border hover:text-amber-300 border border-transparent'
                  }`
                }
              >
                <Zap className="w-4 h-4" />
                Tasks
              </NavLink>
              <NavLink 
                to="/human" 
                className={({ isActive }) => 
                  `flex items-center gap-2 px-4 py-2 rounded-md transition-all duration-200 font-medium text-sm ${
                    isActive 
                      ? 'bg-cyan-400/10 text-cyan-400 border border-cyan-400/20' 
                      : 'text-void-border hover:text-cyan-300 border border-transparent'
                  }`
                }
              >
                <User className="w-4 h-4" />
                Human
              </NavLink>
              <NavLink 
                to="/agent" 
                className={({ isActive }) => 
                  `flex items-center gap-2 px-4 py-2 rounded-md transition-all duration-200 font-medium text-sm ${
                    isActive 
                      ? 'bg-amber-400/10 text-amber-400 border border-amber-400/20' 
                      : 'text-void-border hover:text-amber-300 border border-transparent'
                  }`
                }
              >
                <Bot className="w-4 h-4" />
                Agent
              </NavLink>
            </nav>
          </div>
        </div>
      </header>
      
      <main className="container mx-auto px-6 py-10 relative z-10">
        <Outlet />
      </main>
    </div>
  )
}
