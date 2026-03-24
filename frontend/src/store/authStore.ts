import { create } from 'zustand';
import type { User, AppMode } from '@/types';
import { authApi } from '@/services/api';
import wsService from '@/services/websocket';

interface AuthState {
  user: User | null;
  mode: AppMode | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;

  login: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  setMode: (mode: AppMode) => void;
  checkAuth: () => Promise<void>;
  clearError: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  mode: (localStorage.getItem('app_mode') as AppMode) || null,
  isAuthenticated: !!localStorage.getItem('access_token'),
  isLoading: false,
  error: null,

  login: async (username: string, password: string) => {
    set({ isLoading: true, error: null });
    try {
      const res = await authApi.login(username, password);
      const { access_token, refresh_token, user } = res.data;
      localStorage.setItem('access_token', access_token);
      localStorage.setItem('refresh_token', refresh_token);

      // Connect WebSocket
      wsService.connect(access_token);

      set({ user, isAuthenticated: true, isLoading: false });
    } catch (err: unknown) {
      const message = (err as { response?: { data?: { error?: string } } })?.response?.data?.error || 'Login failed';
      set({ error: message, isLoading: false });
      throw err;
    }
  },

  logout: async () => {
    try {
      await authApi.logout();
    } catch {
      // Ignore errors on logout
    }
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    localStorage.removeItem('app_mode');
    wsService.disconnect();
    set({ user: null, mode: null, isAuthenticated: false });
  },

  setMode: (mode: AppMode) => {
    localStorage.setItem('app_mode', mode);
    set({ mode });
  },

  checkAuth: async () => {
    const token = localStorage.getItem('access_token');
    if (!token) {
      set({ isAuthenticated: false, user: null });
      return;
    }

    set({ isLoading: true });
    try {
      const res = await authApi.me();
      const { user } = res.data;

      // Connect WebSocket if not already connected
      if (!wsService.isConnected) {
        wsService.connect(token);
      }

      set({ user, isAuthenticated: true, isLoading: false });
    } catch {
      localStorage.removeItem('access_token');
      localStorage.removeItem('refresh_token');
      set({ user: null, isAuthenticated: false, isLoading: false });
    }
  },

  clearError: () => set({ error: null }),
}));
