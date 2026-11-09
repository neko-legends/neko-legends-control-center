import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './styles.css'

type CrashState = { error: Error | null }

// A render error unmounts the whole tree and leaves an empty (black) window.
// Show what broke and offer a reload instead.
class CrashScreen extends React.Component<{ children: React.ReactNode }, CrashState> {
  state: CrashState = { error: null }

  static getDerivedStateFromError(error: Error): CrashState {
    return { error }
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error('Control Center crashed', error, info.componentStack)
  }

  render() {
    const { error } = this.state
    if (!error) return this.props.children
    return (
      <div className="crash-screen" role="alert">
        <h1>Something went wrong</h1>
        <p>The Control Center hit an error while drawing this screen. Your apps and settings are untouched.</p>
        <pre>{`${error.name}: ${error.message}\n${error.stack ?? ''}`}</pre>
        <button type="button" onClick={() => window.location.reload()}>
          Reload
        </button>
      </div>
    )
  }
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <CrashScreen>
      <App />
    </CrashScreen>
  </React.StrictMode>,
)
