import { create } from 'zustand';
import type { Chat, Ticket, Analytics, QuickReply, User } from '@/types';
import { businessApi } from '@/services/api';

interface BusinessState {
  queueChats: Chat[];
  myChats: Chat[];
  tickets: Ticket[];
  analytics: Analytics | null;
  quickReplies: QuickReply[];
  agents: User[];
  activeTab: 'queue' | 'my-chats' | 'resolved';
  isLoading: boolean;

  // Actions
  fetchQueue: () => Promise<void>;
  fetchMyChats: () => Promise<void>;
  takeChat: (chatId: string) => Promise<void>;
  transferChat: (chatId: string, toAgentId: string, reason?: string) => Promise<void>;
  fetchTickets: (params?: Record<string, string>) => Promise<void>;
  createTicket: (data: Record<string, unknown>) => Promise<void>;
  fetchAnalytics: () => Promise<void>;
  fetchQuickReplies: () => Promise<void>;
  createQuickReply: (data: Record<string, unknown>) => Promise<void>;
  fetchAgents: () => Promise<void>;
  setActiveTab: (tab: 'queue' | 'my-chats' | 'resolved') => void;
}

export const useBusinessStore = create<BusinessState>((set) => ({
  queueChats: [],
  myChats: [],
  tickets: [],
  analytics: null,
  quickReplies: [],
  agents: [],
  activeTab: 'queue',
  isLoading: false,

  fetchQueue: async () => {
    try {
      const res = await businessApi.queue();
      set({ queueChats: res.data.queue });
    } catch (err) {
      console.error('Failed to fetch queue:', err);
    }
  },

  fetchMyChats: async () => {
    try {
      const res = await businessApi.myChats();
      set({ myChats: res.data.chats });
    } catch (err) {
      console.error('Failed to fetch my chats:', err);
    }
  },

  takeChat: async (chatId: string) => {
    try {
      await businessApi.take(chatId);
      // Refresh queue and my chats
      const [queueRes, myChatsRes] = await Promise.all([
        businessApi.queue(),
        businessApi.myChats(),
      ]);
      set({
        queueChats: queueRes.data.queue,
        myChats: myChatsRes.data.chats,
      });
    } catch (err) {
      console.error('Failed to take chat:', err);
      throw err;
    }
  },

  transferChat: async (chatId: string, toAgentId: string, reason?: string) => {
    try {
      await businessApi.transfer(chatId, toAgentId, reason);
      // Refresh my chats
      const res = await businessApi.myChats();
      set({ myChats: res.data.chats });
    } catch (err) {
      console.error('Failed to transfer chat:', err);
      throw err;
    }
  },

  fetchTickets: async (params?: Record<string, string>) => {
    set({ isLoading: true });
    try {
      const res = await businessApi.tickets.list(params);
      set({ tickets: res.data.tickets, isLoading: false });
    } catch (err) {
      console.error('Failed to fetch tickets:', err);
      set({ isLoading: false });
    }
  },

  createTicket: async (data: Record<string, unknown>) => {
    try {
      await businessApi.tickets.create(data);
    } catch (err) {
      console.error('Failed to create ticket:', err);
      throw err;
    }
  },

  fetchAnalytics: async () => {
    try {
      const res = await businessApi.analytics();
      set({ analytics: res.data as Analytics });
    } catch (err) {
      console.error('Failed to fetch analytics:', err);
    }
  },

  fetchQuickReplies: async () => {
    try {
      const res = await businessApi.quickReplies.list();
      set({ quickReplies: res.data.quick_replies });
    } catch (err) {
      console.error('Failed to fetch quick replies:', err);
    }
  },

  createQuickReply: async (data: Record<string, unknown>) => {
    try {
      await businessApi.quickReplies.create(data);
      // Refresh quick replies list
      const res = await businessApi.quickReplies.list();
      set({ quickReplies: res.data.quick_replies });
    } catch (err) {
      console.error('Failed to create quick reply:', err);
      throw err;
    }
  },

  fetchAgents: async () => {
    try {
      const res = await businessApi.agents();
      set({ agents: res.data.agents });
    } catch (err) {
      console.error('Failed to fetch agents:', err);
    }
  },

  setActiveTab: (tab) => set({ activeTab: tab }),
}));
