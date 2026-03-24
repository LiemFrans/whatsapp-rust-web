import { useState, useEffect, useCallback, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Plus,
  Loader2,
  Users,
  Clock,
  MessageSquare,
  BarChart3,
  ArrowUpRight,
} from 'lucide-react';
import Sidebar, { type SidebarTab } from '@/components/Sidebar';
import ChatList from '@/components/ChatList';
import ChatBubble from '@/components/ChatBubble';
import MessageInput from '@/components/MessageInput';
import QRCodeModal from '@/components/QRCodeModal';
import SettingsPanel from '@/pages/Settings';
import { useChatStore } from '@/store/chatStore';
import { useBusinessStore } from '@/store/businessStore';
import { getChatDisplayName } from '@/utils/chat';
import { useWebSocket } from '@/hooks/useWebSocket';
import wsService from '@/services/websocket';
import type { Message, Chat, Ticket } from '@/types';

// ─── Business Mode Layout ───────────────────────────────────────

export default function BusinessMode() {
  const navigate = useNavigate();
  const [activeTab, setActiveTab] = useState<SidebarTab>('queue');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedChatId, setSelectedChatId] = useState<string | null>(null);
  const [replyTo, setReplyTo] = useState<Message | null>(null);
  const [showQR, setShowQR] = useState(false);
  const [qrCode, setQrCode] = useState<string | null>(null);
  const [qrStatus, setQrStatus] = useState<'connecting' | 'qr_code' | 'connected' | 'error'>('connecting');
  const [sessionName, setSessionName] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const {
    sessions,
    chats,
    messages,
    isLoading,
    fetchSessions,
    fetchChats,
    fetchMessages,
    sendMessage,
    sendMedia,
    connectSession,
  } = useChatStore();

  const {
    queueChats,
    tickets,
    analytics,
    quickReplies,
    fetchQueue,
    fetchMyChats,
    fetchTickets,
    fetchAnalytics,
    fetchQuickReplies,
    takeChat,
  } = useBusinessStore();

  useWebSocket();

  // Listen for WebSocket QR code and session events
  useEffect(() => {
    const unsubQr = wsService.on('qr_code', (event) => {
      const qr = event.data?.qr_code as string;
      if (qr) {
        setQrCode(qr);
        setQrStatus('qr_code');
      }
    });
    const unsubConnected = wsService.on('session_connected', () => {
      setQrStatus('connected');
      fetchSessions();
    });
    return () => {
      unsubQr();
      unsubConnected();
    };
  }, [fetchSessions]);

  useEffect(() => {
    fetchSessions();
    fetchQueue();
    fetchMyChats();
    fetchTickets();
    fetchAnalytics();
    fetchQuickReplies();
  }, [fetchSessions, fetchQueue, fetchMyChats, fetchTickets, fetchAnalytics, fetchQuickReplies]);

  useEffect(() => {
    if (sessions.length > 0) {
      fetchChats(sessions[0].id);
    }
  }, [sessions, fetchChats]);

  useEffect(() => {
    if (selectedChatId) {
      fetchMessages(selectedChatId);
    }
  }, [selectedChatId, fetchMessages]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSelectChat = useCallback((chatId: string) => {
    setSelectedChatId(chatId);
    setReplyTo(null);
  }, []);

  const handleSendMessage = useCallback(
    (content: string) => {
      if (!selectedChatId || !sessions[0]) return;
      sendMessage(selectedChatId, sessions[0].id, content, replyTo?.message_id ?? undefined);
      setReplyTo(null);
    },
    [selectedChatId, sessions, sendMessage, replyTo]
  );

  const handleSendMedia = useCallback(
    (file: File, type: 'image' | 'document', caption?: string) => {
      if (!selectedChatId) return;
      sendMedia(selectedChatId, file, type, caption);
    },
    [selectedChatId, sendMedia]
  );

  const handleTakeChat = async (chatId: string, _sessionId: string) => {
    await takeChat(chatId);
    fetchQueue();
    fetchMyChats();
  };

  const handleConnectSession = async () => {
    const name = `Business-${Date.now()}`;
    setSessionName(name);
    setShowQR(true);
    setQrCode(null);
    setQrStatus('connecting');
    try {
      await connectSession(name);
      // QR code will arrive via WebSocket 'qr_code' event
    } catch {
      setQrStatus('error');
    }
  };

  const selectedChat = chats.find((c) => c.id === selectedChatId);
  const currentMessages = selectedChatId ? (messages[selectedChatId] || []) : [];

  const renderPanel = () => {
    switch (activeTab) {
      case 'queue':
        return (
          <QueuePanel
            chats={queueChats}
            selectedChatId={selectedChatId}
            onSelectChat={handleSelectChat}
            onTakeChat={handleTakeChat}
          />
        );
      case 'my-chats':
        return (
          <div className="flex h-full flex-col">
            <div className="flex h-14 items-center border-b border-gray-200 bg-wa-header px-4 dark:border-gray-700 dark:bg-wa-dark-header">
              <h2 className="text-lg font-semibold text-white">My Chats</h2>
            </div>
            <ChatList
              chats={chats}
              selectedChatId={selectedChatId}
              onSelectChat={handleSelectChat}
              searchQuery={searchQuery}
              onSearchChange={setSearchQuery}
            />
          </div>
        );
      case 'tickets':
        return <TicketPanel tickets={tickets} />;
      case 'analytics':
        return <AnalyticsPanel analytics={analytics} />;
      case 'quick-replies':
        return <QuickRepliesPanel quickReplies={quickReplies} />;
      case 'settings':
        return <SettingsPanel onConnectSession={handleConnectSession} />;
      default:
        return (
          <ChatList
            chats={chats}
            selectedChatId={selectedChatId}
            onSelectChat={handleSelectChat}
            searchQuery={searchQuery}
            onSearchChange={setSearchQuery}
          />
        );
    }
  };

  return (
    <div className="flex h-full">
      <Sidebar
        activeTab={activeTab}
        onTabChange={setActiveTab}
        mode="business"
        isConnected={sessions.some((s) => s.status === 'connected')}
        onBack={() => navigate('/mode-select')}
      />

      {/* Left panel */}
      <div className="w-80 shrink-0">{renderPanel()}</div>

      {/* Chat area */}
      <div className="flex flex-1 flex-col">
        {selectedChat ? (
          <>
            <div className="flex h-14 items-center gap-3 border-b border-gray-200 bg-gray-50 px-4 dark:border-gray-700 dark:bg-wa-dark-header">
              <div className="flex h-10 w-10 items-center justify-center rounded-full bg-gray-300 text-white dark:bg-gray-600">
                <span className="font-semibold">
                  {getChatDisplayName(selectedChat)[0]?.toUpperCase() || '?'}
                </span>
              </div>
              <div className="min-w-0 flex-1">
                <h3 className="truncate font-medium text-gray-900 dark:text-white">
                  {getChatDisplayName(selectedChat)}
                </h3>
                <p className="truncate text-xs text-gray-500">
                  {selectedChat.assigned_agent_name
                    ? `Assigned to: ${selectedChat.assigned_agent_name}`
                    : 'Unassigned'}
                </p>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={handleConnectSession}
                  className="rounded-lg bg-wa-green/10 px-3 py-1.5 text-xs font-medium text-wa-green hover:bg-wa-green/20"
                >
                  <Plus size={14} className="mr-1 inline" />
                  New Session
                </button>
              </div>
            </div>

            <div className="chat-bg flex-1 overflow-y-auto py-4 scrollbar-thin">
              {isLoading ? (
                <div className="flex h-full items-center justify-center">
                  <Loader2 size={32} className="animate-spin text-wa-green" />
                </div>
              ) : (
                <>
                  {currentMessages.map((msg) => (
                    <ChatBubble
                      key={msg.id}
                      message={msg}
                      chatId={selectedChatId!}
                      showSender={selectedChat.is_group}
                      onReply={setReplyTo}
                    />
                  ))}
                  <div ref={messagesEndRef} />
                </>
              )}
            </div>

            <MessageInput
              onSend={handleSendMessage}
              onSendMedia={handleSendMedia}
              replyTo={replyTo}
              onCancelReply={() => setReplyTo(null)}
              disabled={!sessions.some((s) => s.status === 'connected')}
              quickReplies={quickReplies
                .filter((qr) => qr.shortcut != null)
                .map((qr) => ({
                  shortcut: qr.shortcut!,
                  content: qr.content,
                }))}
            />
          </>
        ) : (
          <BusinessEmptyState
            analytics={analytics}
            onConnect={handleConnectSession}
            hasSession={sessions.length > 0}
          />
        )}
      </div>

      <QRCodeModal
        isOpen={showQR}
        onClose={() => setShowQR(false)}
        qrCode={qrCode}
        sessionName={sessionName}
        status={qrStatus}
      />
    </div>
  );
}

// ─── Queue Panel ────────────────────────────────────────────────

function QueuePanel({
  chats,
  selectedChatId,
  onSelectChat,
  onTakeChat,
}: {
  chats: Chat[];
  selectedChatId: string | null;
  onSelectChat: (id: string) => void;
  onTakeChat: (chatId: string, sessionId: string) => void;
}) {
  return (
    <div className="flex h-full flex-col">
      <div className="flex h-14 items-center justify-between border-b border-gray-200 bg-wa-header px-4 dark:border-gray-700 dark:bg-wa-dark-header">
        <h2 className="text-lg font-semibold text-white">Queue</h2>
        <span className="rounded-full bg-white/20 px-2 py-0.5 text-xs text-white">
          {chats.length}
        </span>
      </div>
      <div className="flex-1 overflow-y-auto">
        {chats.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-16 text-gray-400">
            <Users size={48} className="mb-2 opacity-50" />
            <p className="text-sm">Queue is empty</p>
            <p className="text-xs">All chats have been assigned</p>
          </div>
        ) : (
          chats.map((chat) => (
            <div
              key={chat.id}
              className={`border-b border-gray-100 p-3 dark:border-gray-700 ${
                selectedChatId === chat.id ? 'bg-wa-green/5' : ''
              }`}
            >
              <button
                onClick={() => onSelectChat(chat.id)}
                className="mb-2 flex w-full items-center gap-3 text-left"
              >
                <div className="flex h-10 w-10 items-center justify-center rounded-full bg-gray-300 text-sm font-bold text-white dark:bg-gray-600">
                  {getChatDisplayName(chat)[0]?.toUpperCase() || '?'}
                </div>
                <div className="min-w-0 flex-1">
                  <h4 className="truncate text-sm font-medium text-gray-900 dark:text-white">
                    {getChatDisplayName(chat)}
                  </h4>
                  <p className="truncate text-xs text-gray-500">{chat.last_message}</p>
                </div>
                {chat.unread_count > 0 && (
                  <span className="flex h-5 min-w-5 items-center justify-center rounded-full bg-red-500 px-1 text-xs font-bold text-white">
                    {chat.unread_count}
                  </span>
                )}
              </button>
              <button
                onClick={() => onTakeChat(chat.id, chat.session_id)}
                className="w-full rounded-lg bg-wa-green/10 py-1.5 text-xs font-medium text-wa-green hover:bg-wa-green/20"
              >
                Take Chat
              </button>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// ─── Ticket Panel ───────────────────────────────────────────────

function TicketPanel({ tickets }: { tickets: Ticket[] }) {
  const statusColors: Record<string, string> = {
    open: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
    in_progress: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
    pending: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400',
    resolved: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
    closed: 'bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400',
  };

  const priorityColors: Record<string, string> = {
    low: 'text-gray-500',
    medium: 'text-blue-500',
    high: 'text-orange-500',
    critical: 'text-red-500',
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-14 items-center justify-between border-b border-gray-200 bg-wa-header px-4 dark:border-gray-700 dark:bg-wa-dark-header">
        <h2 className="text-lg font-semibold text-white">Tickets</h2>
        <span className="rounded-full bg-white/20 px-2 py-0.5 text-xs text-white">
          {tickets.length}
        </span>
      </div>
      <div className="flex-1 overflow-y-auto">
        {tickets.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-16 text-gray-400">
            <MessageSquare size={48} className="mb-2 opacity-50" />
            <p className="text-sm">No tickets</p>
          </div>
        ) : (
          tickets.map((ticket) => (
            <div key={ticket.id} className="border-b border-gray-100 p-3 dark:border-gray-700">
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0 flex-1">
                  <h4 className="truncate text-sm font-medium text-gray-900 dark:text-white">
                    {ticket.title}
                  </h4>
                  {ticket.description && (
                    <p className="mt-0.5 truncate text-xs text-gray-500">{ticket.description}</p>
                  )}
                </div>
                <span className={`shrink-0 text-xs font-bold ${priorityColors[ticket.priority] || ''}`}>
                  {ticket.priority.toUpperCase()}
                </span>
              </div>
              <div className="mt-2 flex items-center gap-2">
                <span className={`rounded-full px-2 py-0.5 text-xs font-medium ${statusColors[ticket.status] || ''}`}>
                  {ticket.status.replace('_', ' ')}
                </span>
                <span className="text-xs text-gray-400">
                  {new Date(ticket.created_at).toLocaleDateString()}
                </span>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// ─── Analytics Panel ────────────────────────────────────────────

function AnalyticsPanel({ analytics }: { analytics: any }) {
  if (!analytics) {
    return (
      <div className="flex h-full flex-col">
        <div className="flex h-14 items-center border-b border-gray-200 bg-wa-header px-4 dark:border-gray-700 dark:bg-wa-dark-header">
          <h2 className="text-lg font-semibold text-white">Analytics</h2>
        </div>
        <div className="flex flex-1 items-center justify-center text-gray-400">
          <Loader2 size={24} className="animate-spin" />
        </div>
      </div>
    );
  }

  const stats = [
    { label: 'Messages Today', value: analytics.messages_today || 0, icon: <MessageSquare size={20} /> },
    { label: 'Unassigned', value: analytics.unassigned_count || 0, icon: <Users size={20} /> },
    { label: 'Open Tickets', value: analytics.ticket_breakdown?.open || 0, icon: <Clock size={20} /> },
    { label: 'Active Agents', value: analytics.chats_per_agent?.length || 0, icon: <BarChart3 size={20} /> },
  ];

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-14 items-center border-b border-gray-200 bg-wa-header px-4 dark:border-gray-700 dark:bg-wa-dark-header">
        <h2 className="text-lg font-semibold text-white">Analytics</h2>
      </div>
      <div className="flex-1 overflow-y-auto p-3">
        <div className="grid grid-cols-2 gap-3">
          {stats.map((stat, i) => (
            <div
              key={i}
              className="rounded-xl border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800"
            >
              <div className="mb-2 text-wa-green">{stat.icon}</div>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">{stat.value}</p>
              <p className="text-xs text-gray-500">{stat.label}</p>
            </div>
          ))}
        </div>

        {analytics.chats_per_agent && analytics.chats_per_agent.length > 0 && (
          <div className="mt-4">
            <h3 className="mb-2 text-sm font-semibold text-gray-700 dark:text-gray-300">Chats per Agent</h3>
            <div className="space-y-2">
              {analytics.chats_per_agent.map((agent: any, i: number) => (
                <div key={i} className="flex items-center justify-between rounded-lg bg-gray-50 p-2 dark:bg-gray-800">
                  <span className="text-sm text-gray-700 dark:text-gray-300">{agent.agent_name}</span>
                  <span className="rounded-full bg-wa-green/10 px-2 py-0.5 text-xs font-bold text-wa-green">
                    {agent.count}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Quick Replies Panel ────────────────────────────────────────

function QuickRepliesPanel({ quickReplies }: { quickReplies: any[] }) {
  return (
    <div className="flex h-full flex-col">
      <div className="flex h-14 items-center justify-between border-b border-gray-200 bg-wa-header px-4 dark:border-gray-700 dark:bg-wa-dark-header">
        <h2 className="text-lg font-semibold text-white">Quick Replies</h2>
        <button className="rounded-full bg-white/20 p-1.5 text-white hover:bg-white/30">
          <Plus size={16} />
        </button>
      </div>
      <div className="flex-1 overflow-y-auto">
        {quickReplies.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-16 text-gray-400">
            <ArrowUpRight size={48} className="mb-2 opacity-50" />
            <p className="text-sm">No quick replies</p>
            <p className="text-xs">Create shortcuts for common messages</p>
          </div>
        ) : (
          quickReplies.map((qr: any) => (
            <div key={qr.id} className="border-b border-gray-100 p-3 dark:border-gray-700">
              <div className="flex items-center gap-2">
                <span className="rounded bg-wa-green/10 px-2 py-0.5 font-mono text-xs text-wa-green">
                  /{qr.shortcut}
                </span>
                {qr.category && (
                  <span className="rounded bg-gray-100 px-1.5 py-0.5 text-xs text-gray-500 dark:bg-gray-700">
                    {qr.category}
                  </span>
                )}
              </div>
              <p className="mt-1 text-sm text-gray-600 dark:text-gray-300">{qr.content}</p>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// ─── Business Empty State ───────────────────────────────────────

function BusinessEmptyState({
  analytics,
  onConnect,
  hasSession,
}: {
  analytics: any;
  onConnect: () => void;
  hasSession: boolean;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center bg-gray-50 dark:bg-wa-dark-bg">
      <div className="text-center">
        <div className="mb-6 inline-flex h-24 w-24 items-center justify-center rounded-full bg-wa-teal/10">
          <MessageSquare size={48} className="text-wa-teal" />
        </div>
        <h2 className="text-2xl font-light text-gray-600 dark:text-gray-300">Business Mode</h2>
        <p className="mt-2 max-w-md text-sm text-gray-400">
          Manage customer conversations, assign chats to agents, create tickets, and track analytics.
          Select a chat from the panel to get started.
        </p>

        {!hasSession && (
          <button
            onClick={onConnect}
            className="mt-6 flex items-center gap-2 rounded-lg bg-wa-teal px-6 py-2.5 text-sm font-medium text-white shadow-md hover:bg-wa-teal/90"
          >
            <Plus size={18} />
            Connect WhatsApp Session
          </button>
        )}

        {analytics && (
          <div className="mt-8 grid grid-cols-3 gap-4">
            <div className="rounded-xl bg-white p-4 shadow dark:bg-gray-800">
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {analytics.messages_today || 0}
              </p>
              <p className="text-xs text-gray-500">Messages Today</p>
            </div>
            <div className="rounded-xl bg-white p-4 shadow dark:bg-gray-800">
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {analytics.unassigned_count || 0}
              </p>
              <p className="text-xs text-gray-500">In Queue</p>
            </div>
            <div className="rounded-xl bg-white p-4 shadow dark:bg-gray-800">
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {analytics.ticket_breakdown?.open || 0}
              </p>
              <p className="text-xs text-gray-500">Open Tickets</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
