import { Navigate, Link } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';

export function Home() {
  const { user, isLoading } = useAuth();

  if (isLoading) {
    return <div style={{ padding: 24, textAlign: 'center', color: '#888' }}>Loading...</div>;
  }

  if (user) {
    return <Navigate to="/bots" replace />;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', flex: 1, padding: 48, textAlign: 'center' }}>
      <h1 style={{ fontSize: 48, color: '#16c79a', marginBottom: 8 }}>SwarmCrest</h1>
      <p style={{ fontSize: 20, color: '#aaa', marginBottom: 32 }}>
        Program your bots. Compete for dominance.
      </p>
      <p style={{ maxWidth: 600, color: '#888', lineHeight: 1.6, marginBottom: 40 }}>
        SwarmCrest is a programming game where you write Lua scripts to control swarms of creatures competing for food and territory. Train your bots, challenge opponents, and climb the leaderboard.
      </p>
      <div style={{ display: 'flex', gap: 16, marginBottom: 40 }}>
        <Link to="/register" style={{ background: '#16c79a', color: '#fff', padding: '12px 32px', borderRadius: 4, textDecoration: 'none', fontWeight: 700, fontSize: 16 }}>
          Start Competing
        </Link>
        <Link to="/login" style={{ background: 'transparent', color: '#16c79a', padding: '12px 32px', borderRadius: 4, textDecoration: 'none', fontWeight: 600, fontSize: 16, border: '1px solid #16c79a' }}>
          Login
        </Link>
      </div>
      <div style={{ display: 'flex', gap: 24 }}>
        <Link to="/leaderboard" style={{ color: '#6a6aff', textDecoration: 'none' }}>Leaderboard</Link>
        <Link to="/tournaments" style={{ color: '#6a6aff', textDecoration: 'none' }}>Tournaments</Link>
        <Link to="/docs" style={{ color: '#6a6aff', textDecoration: 'none' }}>Documentation</Link>
      </div>
    </div>
  );
}
