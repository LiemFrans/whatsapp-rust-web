import { create } from 'zustand';
import type { Chat, Message, WhatsAppSession } from '@/types';
import { chatApi, sessionApi } from '@/services/api';

export interface ChatState {
  sessions: WhatsAppSession[];
  activeSessionId: string | null;
  chats: Chat[];
  activeChat: Chat | null;
  messages: Record<string, Message[]>;
  isLoading: boolean;
  isLoadingChats: boolean;
  isLoadingMessages: boolean;
  hasMoreMessages: boolean;
  searchQuery: string;
  syncProgress: string | null;

  // Actions
  fetchSessions: () => Promise<void>;
  setActiveSession: (sessionId: string | null) => void;
  fetchChats: (sessionId?: string, search?: string) => Promise<void>;
  setActiveChat: (chat: Chat | null) => void;
  fetchMessages: (chatId: string, cursor?: string) => Promise<void>;
  sendMessage: (chatId: string, sessionId: string, content: string, replyTo?: string) => Promise<void>;
  sendMedia: (chatId: string, file: File, type: 'image' | 'document', caption?: string) => Promise<void>;
  addIncomingMessage: (message: Message & { chat_id?: string }) => void;
  updateMessageStatus: (messageId: string, status: string) => void;
  setSearchQuery: (query: string) => void;
  markChatRead: (chatId: string) => Promise<void>;
  connectSession: (name: string) => Promise<{ qr_code?: string } | undefined>;
  disconnectSession: (sessionId: string) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
  triggerSync: (sessionId: string) => Promise<void>;
  setSyncProgress: (progress: string | null) => void;
  updateChatUnread: (chatId: string, increment: boolean) => void;
}

export const useChatStore = create<ChatState>((set, get) => ({
  sessions: [],
  activeSessionId: null,
  chats: [],
  activeChat: null,
  messages: {},
  isLoading: false,
  isLoadingChats: false,
  isLoadingMessages: false,
  hasMoreMessages: true,
  searchQuery: '',
  syncProgress: null,

  fetchSessions: async () => {
    try {
      const res = await sessionApi.list();
      set({ sessions: res.data.sessions });
    } catch (err) {
      console.error('Failed to fetch sessions:', err);
    }
  },

  setActiveSession: (sessionId) => {
    set({ activeSessionId: sessionId });
    if (sessionId) {
      get().fetchChats(sessionId);
    }
  },

  fetchChats: async (sessionId?: string, search?: string) => {
    set({ isLoadingChats: true });
    try {
      const res = await chatApi.list({
        session_id: sessionId || get().activeSessionId || undefined,
        search: search || get().searchQuery || undefined,
      });
      set({ chats: res.data.chats, isLoadingChats: false });
    } catch (err) {
      console.error('Failed to fetch chats:', err);
      set({ isLoadingChats: false });
    }
  },

  setActiveChat: (chat) => {
    set({ activeChat: chat, hasMoreMessages: true });
    if (chat) {
      get().fetchMessages(chat.id);
      if (chat.unread_count > 0) {
        get().markChatRead(chat.id);
      }
    }
  },

  fetchMessages: async (chatId: string, cursor?: string) => {
    set({ isLoadingMessages: true });
    try {
      const res = await chatApi.messages(chatId, { cursor, limit: 50 });
      const dbMessages = res.data.messages as Message[];

      if (cursor) {
        // Prepend older messages (pagination)
        set((state) => {
          const existing = state.messages[chatId] || [];
          const merged = [...dbMessages, ...existing];
          // Deduplicate by id, keeping the first occurrence
          const seen = new Set<string>();
          const unique = merged.filter((m) => {
            if (seen.has(m.id)) return false;
            seen.add(m.id);
            return true;
          });
          return {
            messages: { ...state.messages, [chatId]: unique },
            isLoadingMessages: false,
            hasMoreMessages: res.data.has_more,
          };
        });
      } else {
        // Initial load: merge DB results with any WS messages already in memory
        set((state) => {
          const existing = state.messages[chatId] || [];
          const dbIds = new Set(dbMessages.map((m) => m.id));
          // Keep WS messages that aren't already in DB results
          const wsOnly = existing.filter((m) => !dbIds.has(m.id));
          const merged = [...dbMessages, ...wsOnly];
          // Sort by timestamp ascending
          merged.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
          return {
            messages: { ...state.messages, [chatId]: merged },
            isLoadingMessages: false,
            hasMoreMessages: res.data.has_more,
          };
        });
      }
    } catch (err) {
      console.error('Failed to fetch messages:', err);
      set({ isLoadingMessages: false });
    }
  },

  sendMessage: async (chatId: string, _sessionId: string, content: string, replyTo?: string) => {
    try {
      const res = await chatApi.send(chatId, content, replyTo);
      const message = res.data.message as Message;
      set((state) => ({
        messages: {
          ...state.messages,
          [chatId]: [...(state.messages[chatId] || []), message],
        },
      }));
    } catch (err) {
      console.error('Failed to send message:', err);
      throw err;
    }
  },

  sendMedia: async (chatId: string, file: File, type: 'image' | 'document', caption?: string) => {
    try {
      const res = await chatApi.sendMedia(chatId, file, type, caption);
      const message = res.data.message as Message;
      set((state) => ({
        messages: {
          ...state.messages,
          [chatId]: [...(state.messages[chatId] || []), message],
        },
      }));
    } catch (err) {
      console.error('Failed to send media:', err);
      throw err;
    }
  },

  addIncomingMessage: (message) => {
    const chatId = message.chat_id;
    if (chatId) {
      set((state) => ({
        messages: {
          ...state.messages,
          [chatId]: [...(state.messages[chatId] || []), message],
        },
      }));
    }

    // Update chat list
    set((state) => ({
      chats: state.chats.map((chat) =>
        chat.id === message.chat_id
          ? {
              ...chat,
              last_message: message.content,
              last_message_at: message.timestamp,
              unread_count: state.activeChat?.id === message.chat_id
                ? chat.unread_count
                : chat.unread_count + 1,
            }
          : chat
      ),
    }));
  },

  updateMessageStatus: (messageId: string, status: string) => {
    set((state) => {
      const newMessages = { ...state.messages };
      for (const chatId of Object.keys(newMessages)) {
        newMessages[chatId] = newMessages[chatId].map((msg) =>
          msg.message_id === messageId ? { ...msg, status: status as Message['status'] } : msg
        );
      }
      return { messages: newMessages };
    });
  },

  setSearchQuery: (query) => {
    set({ searchQuery: query });
    get().fetchChats(undefined, query);
  },

  markChatRead: async (chatId: string) => {
    try {
      await chatApi.markRead(chatId);
      set((state) => ({
        chats: state.chats.map((chat) =>
          chat.id === chatId ? { ...chat, unread_count: 0 } : chat
        ),
      }));
    } catch (err) {
      console.error('Failed to mark chat as read:', err);
    }
  },

  connectSession: async (name: string) => {
    try {
      const res = await sessionApi.connect(name);
      set((state) => ({
        sessions: [...state.sessions, res.data.session],
      }));
      return res.data as { qr_code?: string };
    } catch (err) {
      console.error('Failed to connect session:', err);
      throw err;
    }
  },

  triggerSync: async (sessionId: string) => {
    try {
      await chatApi.sync(sessionId);
    } catch (err) {
      console.error('Failed to trigger sync:', err);
    }
  },

  disconnectSession: async (sessionId: string) => {
    try {
      await sessionApi.disconnect(sessionId);
      set((state) => ({
        sessions: state.sessions.map((s) =>
          s.id === sessionId ? { ...s, status: 'disconnected' as const } : s
        ),
      }));
    } catch (err) {
      console.error('Failed to disconnect session:', err);
      throw err;
    }
  },

  deleteSession: async (sessionId: string) => {
    try {
      await sessionApi.delete(sessionId);
      set((state) => ({
        sessions: state.sessions.filter((s) => s.id !== sessionId),
      }));
    } catch (err) {
      console.error('Failed to delete session:', err);
      throw err;
    }
  },

  setSyncProgress: (progress) => set({ syncProgress: progress }),

  updateChatUnread: (chatId: string, increment: boolean) => {
    set((state) => ({
      chats: state.chats.map((chat) =>
        chat.id === chatId
          ? { ...chat, unread_count: increment ? chat.unread_count + 1 : 0 }
          : chat
      ),
    }));
  },
}));
