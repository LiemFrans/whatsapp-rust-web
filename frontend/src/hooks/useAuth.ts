import { useEffect } from 'react';
import { useAuthStore } from '@/store/authStore';

export function useAuth() {
  const store = useAuthStore();

  useEffect(() => {
    if (store.isAuthenticated && !store.user) {
      store.checkAuth();
    }
  }, [store.isAuthenticated]);

  return store;
}
