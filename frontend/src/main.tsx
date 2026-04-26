import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter, Routes, Route } from 'react-router-dom'
import App from './App'
import TaskBoard from './components/TaskBoard'
import HumanDashboard from './components/HumanDashboard'
import AgentDashboard from './components/AgentDashboard'
import './index.css'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<App />}>
          <Route index element={<TaskBoard />} />
          <Route path="human" element={<HumanDashboard />} />
          <Route path="agent" element={<AgentDashboard />} />
        </Route>
      </Routes>
    </BrowserRouter>
  </StrictMode>,
)
