import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';

export function OAuthCallback() {
  const [searchParams] = useSearchParams();
  const [fetchError, setFetchError] = useState('');
  const { login } = useAuth();
  const navigate = useNavigate();

  const token = searchParams.get('token');
  const paramError = useMemo(() => {
    const err = searchParams.get('error');
    if (err) return err === 'internal' ? 'Authentication failed. Please try again.' : err;
    if (!token) return 'No authentication token received.';
    return null;
  }, [searchParams, token]);

  useEffect(() => {
    if (paramError || !token) return;

    fetch('/api/auth/me', {
      headers: { Authorization: `Bearer ${token}` },
    })
      .then(r => {
        if (!r.ok) throw new Error('Invalid token');
        return r.json();
      })
      .then(user => {
        login(token, user);
        navigate('/', { replace: true });
      })
      .catch(() => {
        setFetchError('Authentication failed. Please try again.');
      });
  }, [paramError, token, login, navigate]);

  const error = paramError || fetchError;

  if (error) {
    return (
      <div style={{ maxWidth: 400, margin: '80px auto', padding: 24, textAlign: 'center' }}>
        <h2>Authentication Error</h2>
        <p style={{ color: '#f44' }}>{error}</p>
        <a href="/login" style={{ color: '#16c79a' }}>Back to Login</a>
      </div>
    );
  }

  return (
    <div style={{ maxWidth: 400, margin: '80px auto', padding: 24, textAlign: 'center' }}>
      <p>Signing you in...</p>
    </div>
  );
}
