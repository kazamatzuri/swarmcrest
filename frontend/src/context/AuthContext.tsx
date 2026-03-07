import { createContext, useContext, useState, useEffect } from 'react';
import type { ReactNode } from 'react';

export interface AuthUser {
  id: number;
  username: string;
  email: string;
  display_name: string | null;
  role: string;
  created_at: string;
}

interface AuthContextType {
  user: AuthUser | null;
  token: string | null;
  login: (token: string, user: AuthUser) => void;
  logout: () => void;
  isLoading: boolean;
}

const AuthContext = createContext<AuthContextType>({
  user: null,
  token: null,
  login: () => {},
  logout: () => {},
  isLoading: true,
});

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const savedToken = localStorage.getItem('swarmcrest_token');
  const [isLoading, setIsLoading] = useState(!!savedToken);

  useEffect(() => {
    if (savedToken) {
      fetch('/api/auth/me', {
        headers: { Authorization: `Bearer ${savedToken}` },
      })
        .then(r => {
          if (r.ok) return r.json();
          throw new Error('Invalid token');
        })
        .then((u: AuthUser) => {
          setToken(savedToken);
          setUser(u);
        })
        .catch(() => {
          localStorage.removeItem('swarmcrest_token');
        })
        .finally(() => setIsLoading(false));
    }
  }, [savedToken]);

  const login = (newToken: string, newUser: AuthUser) => {
    localStorage.setItem('swarmcrest_token', newToken);
    setToken(newToken);
    setUser(newUser);
  };

  const logout = () => {
    localStorage.removeItem('swarmcrest_token');
    setToken(null);
    setUser(null);
  };

  return (
    <AuthContext.Provider value={{ user, token, login, logout, isLoading }}>
      {children}
    </AuthContext.Provider>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export function useAuth() {
  return useContext(AuthContext);
}
