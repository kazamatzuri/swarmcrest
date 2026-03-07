import { BrowserRouter, Routes, Route, NavLink, Link } from 'react-router-dom';
import { AuthProvider, useAuth } from './context/AuthContext';
import { Home } from './pages/Home';
import { BotLibrary } from './pages/BotLibrary';
import { BotEditor } from './pages/BotEditor';
import { TournamentList } from './pages/TournamentList';
import { TournamentDetail } from './pages/TournamentDetail';
import { GameViewer } from './pages/GameViewer';
import { GameList } from './pages/GameList';
import { Login } from './pages/Login';
import { Register } from './pages/Register';
import { Leaderboard } from './pages/Leaderboard';
import { ApiKeys } from './pages/ApiKeys';
import { Challenge } from './pages/Challenge';
import { MyMatches } from './pages/MyMatches';
import { Teams } from './pages/Teams';
import { MatchDetail } from './pages/MatchDetail';
import { Documentation } from './pages/Documentation';
import { About } from './pages/About';
import { ProtectedRoute } from './components/ProtectedRoute';
import { NotificationBell } from './components/NotificationBell';
import { FeedbackButton } from './components/FeedbackButton';
import './App.css';

function NavBar() {
  const { user, logout } = useAuth();

  return (
    <nav className="app-nav">
      <Link to="/" style={{ textDecoration: 'none', color: 'inherit' }}><h1 className="app-title">SwarmCrest</h1></Link>
      {user ? (
        <>
          <NavLink to="/bots" className={navLinkClass}>Bot Library</NavLink>
          <NavLink to="/leaderboard" className={navLinkClass}>Leaderboard</NavLink>
          <NavLink to="/tournaments" className={navLinkClass}>Tournaments</NavLink>
          <NavLink to="/games" className={navLinkClass}>Games</NavLink>
          <NavLink to="/docs" className={navLinkClass}>Docs</NavLink>
          <NavLink to="/about" className={navLinkClass}>About</NavLink>
        </>
      ) : (
        <>
          <NavLink to="/leaderboard" className={navLinkClass}>Leaderboard</NavLink>
          <NavLink to="/tournaments" className={navLinkClass}>Tournaments</NavLink>
          <NavLink to="/games" className={navLinkClass}>Games</NavLink>
          <NavLink to="/docs" className={navLinkClass}>Docs</NavLink>
          <NavLink to="/about" className={navLinkClass}>About</NavLink>
        </>
      )}
      <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 8 }}>
        {user ? (
          <>
            <NavLink to="/challenge" className={navLinkClass}>Challenge</NavLink>
            <NavLink to="/my-matches" className={navLinkClass}>My Matches</NavLink>
            <NavLink to="/teams" className={navLinkClass}>Teams</NavLink>
            <NavLink to="/api-keys" className={navLinkClass}>API Keys</NavLink>
            <NotificationBell />
            <span style={{ color: '#aaa', fontSize: 14 }}>{user.username}</span>
            <button onClick={logout} style={{ padding: '4px 12px', fontSize: 13, cursor: 'pointer' }}>
              Logout
            </button>
          </>
        ) : (
          <>
            <NavLink to="/login" className={navLinkClass}>Login</NavLink>
            <NavLink to="/register" className={navLinkClass}>Register</NavLink>
          </>
        )}
      </div>
    </nav>
  );
}

function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <div style={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
          <NavBar />
          <main style={{ flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column' }}>
            <Routes>
              <Route path="/" element={<Home />} />
              <Route path="/bots" element={<ProtectedRoute><BotLibrary /></ProtectedRoute>} />
              <Route path="/editor" element={<ProtectedRoute><BotEditor /></ProtectedRoute>} />
              <Route path="/editor/:botId" element={<ProtectedRoute><BotEditor /></ProtectedRoute>} />
              <Route path="/leaderboard" element={<Leaderboard />} />
              <Route path="/tournaments" element={<TournamentList />} />
              <Route path="/tournaments/:id" element={<TournamentDetail />} />
              <Route path="/games" element={<GameList />} />
              <Route path="/game" element={<ProtectedRoute><GameViewer /></ProtectedRoute>} />
              <Route path="/teams" element={<ProtectedRoute><Teams /></ProtectedRoute>} />
              <Route path="/matches/:id" element={<MatchDetail />} />
              <Route path="/challenge" element={<ProtectedRoute><Challenge /></ProtectedRoute>} />
              <Route path="/my-matches" element={<ProtectedRoute><MyMatches /></ProtectedRoute>} />
              <Route path="/api-keys" element={<ProtectedRoute><ApiKeys /></ProtectedRoute>} />
              <Route path="/docs" element={<Documentation />} />
              <Route path="/about" element={<About />} />
              <Route path="/login" element={<Login />} />
              <Route path="/register" element={<Register />} />
            </Routes>
          </main>
          <FeedbackButton />
        </div>
      </AuthProvider>
    </BrowserRouter>
  );
}

function navLinkClass({ isActive }: { isActive: boolean }): string {
  return isActive ? 'nav-link nav-link-active' : 'nav-link';
}

export default App;
