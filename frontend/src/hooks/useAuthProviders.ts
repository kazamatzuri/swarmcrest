import { useEffect, useState } from 'react';

export interface AuthProviders {
  github: boolean;
  google: boolean;
  password: boolean;
}

export function useAuthProviders() {
  const [providers, setProviders] = useState<AuthProviders | null>(null);

  useEffect(() => {
    fetch('/api/auth/providers')
      .then(r => r.ok ? r.json() : null)
      .then(data => { if (data) setProviders(data); })
      .catch(() => {});
  }, []);

  return providers;
}
