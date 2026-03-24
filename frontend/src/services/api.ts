import axios, { type AxiosInstance, type InternalAxiosRequestConfig } from 'axios';

const API_URL = import.meta.env.VITE_API_URL || '';

const api: AxiosInstance = axios.create({
  baseURL: `${API_URL}/api`,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Add auth token to every request
api.interceptors.request.use((config: InternalAxiosRequestConfig) => {
  const token = localStorage.getItem('access_token');
  if (token && config.headers) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// Handle 401 - try refresh token
api.interceptors.response.use(
  (response) => response,
  async (error) => {
    const originalRequest = error.config;
    if (error.response?.status === 401 && !originalRequest._retry) {
      originalRequest._retry = true;
      const refreshToken = localStorage.getItem('refresh_token');
      if (refreshToken) {
        try {
          const res = await axios.post(`${API_URL}/api/auth/refresh`, {
            refresh_token: refreshToken,
          });
          const { access_token, refresh_token } = res.data;
          localStorage.setItem('access_token', access_token);
          localStorage.setItem('refresh_token', refresh_token);
          originalRequest.headers.Authorization = `Bearer ${access_token}`;
          return api(originalRequest);
        } catch {
          localStorage.removeItem('access_token');
          localStorage.removeItem('refresh_token');
          window.location.href = '/login';
        }
      }
    }
    return Promise.reject(error);
  }
);

// ── Auth API ──────────────────────────────────────────────────

export const authApi = {
  login: (username: string, password: string) =>
    api.post('/auth/login', { username, password }),
  register: (data: { username: string; email: string; password: string; display_name?: string; role?: string }) =>
    api.post('/auth/register', data),
  refresh: (refresh_token: string) =>
    api.post('/auth/refresh', { refresh_token }),
  logout: () => api.post('/auth/logout'),
  me: () => api.get('/auth/me'),
};

// ── WhatsApp Session API ──────────────────────────────────────

export const sessionApi = {
  list: () => api.get('/whatsapp/sessions'),
  connect: (session_name: string) =>
    api.post('/whatsapp/connect', { session_name }),
  disconnect: (session_id: string) =>
    api.post(`/whatsapp/disconnect/${session_id}`),
  delete: (session_id: string) =>
    api.delete(`/whatsapp/sessions/${session_id}`),
  status: (session_id: string) =>
    api.get(`/whatsapp/status/${session_id}`),
};

// ── Chat API ──────────────────────────────────────────────────

export const chatApi = {
  list: (params?: { session_id?: string; search?: string; filter?: string; page?: number }) =>
    api.get('/chats', { params }),
  get: (chatId: string) => api.get(`/chats/${chatId}`),
  messages: (chatId: string, params?: { cursor?: string; limit?: number }) =>
    api.get(`/chats/${chatId}/messages`, { params }),
  send: (chatId: string, content: string, reply_to?: string) =>
    api.post(`/chats/${chatId}/send`, { content, reply_to }),
  sendMedia: (chatId: string, file: File, type: 'image' | 'document', caption?: string) => {
    const formData = new FormData();
    formData.append('file', file);
    formData.append('type', type);
    if (caption) formData.append('caption', caption);
    return api.post(`/chats/${chatId}/send-media`, formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    });
  },
  mediaUrl: (chatId: string, messageId: string) =>
    `${API_URL}/api/chats/${chatId}/messages/${messageId}/media`,
  markRead: (chatId: string) => api.post(`/chats/${chatId}/read`),
  toggleArchive: (chatId: string) => api.post(`/chats/${chatId}/archive`),
  togglePin: (chatId: string) => api.post(`/chats/${chatId}/pin`),
  toggleMute: (chatId: string) => api.post(`/chats/${chatId}/mute`),
  toggleStar: (messageId: string) => api.post(`/chats/messages/${messageId}/star`),
  deleteMessage: (messageId: string) => api.delete(`/chats/messages/${messageId}`),
  sync: (sessionId?: string) => api.post('/chats/sync', { session_id: sessionId }),
};

// ── Business API ──────────────────────────────────────────────

export const businessApi = {
  queue: () => api.get('/business/queue'),
  myChats: () => api.get('/business/my-chats'),
  assign: (chat_id: string, agent_id?: string) =>
    api.post('/business/assign', { chat_id, agent_id }),
  take: (chat_id: string) => api.post('/business/take', { chat_id }),
  transfer: (chat_id: string, to_agent_id: string, reason?: string) =>
    api.post('/business/transfer', { chat_id, to_agent_id, reason }),
  tickets: {
    list: (params?: Record<string, string>) => api.get('/business/tickets', { params }),
    get: (id: string) => api.get(`/business/tickets/${id}`),
    create: (data: Record<string, unknown>) => api.post('/business/tickets', data),
    update: (id: string, data: Record<string, unknown>) => api.put(`/business/tickets/${id}`, data),
    notes: (id: string) => api.get(`/business/tickets/${id}/notes`),
    addNote: (id: string, note: string) => api.post(`/business/tickets/${id}/notes`, { note }),
  },
  escalate: (data: { ticket_id: string; to_user_id: string; reason: string }) =>
    api.post('/business/escalations', data),
  resolveEscalation: (id: string, data: { action: string; resolution_note?: string }) =>
    api.post(`/business/escalations/${id}/resolve`, data),
  quickReplies: {
    list: () => api.get('/business/quick-replies'),
    create: (data: Record<string, unknown>) => api.post('/business/quick-replies', data),
    update: (id: string, data: Record<string, unknown>) => api.put(`/business/quick-replies/${id}`, data),
  },
  analytics: () => api.get('/business/analytics'),
  agents: () => api.get('/business/agents'),
};

// ── Users API ──────────────────────────────────────────────────

export const usersApi = {
  list: () => api.get('/users'),
  get: (id: string) => api.get(`/users/${id}`),
  create: (data: { username: string; email: string; password: string; display_name?: string; role?: string }) =>
    authApi.register(data),
  update: (id: string, data: { display_name?: string; email?: string; role?: string }) =>
    api.put(`/users/${id}`, data),
  toggleActive: (id: string) => api.put(`/users/${id}/toggle-active`),
};

export default api;
