import { useState, useEffect, useCallback, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { Plus, Loader2, RefreshCw } from 'lucide-react';
import Sidebar, { type SidebarTab } from '@/components/Sidebar';
import ChatList from '@/components/ChatList';
import ChatBubble from '@/components/ChatBubble';
import MessageInput from '@/components/MessageInput';
import QRCodeModal from '@/components/QRCodeModal';
import SettingsPanel from '@/pages/Settings';
import { useChatStore } from '@/store/chatStore';
import { getChatDisplayName } from '@/utils/chat';
import { useWebSocket } from '@/hooks/useWebSocket';
import wsService from '@/services/websocket';
import type { Message } from '@/types';

export default function PersonalMode() {
  const navigate = useNavigate();
  const [activeTab, setActiveTab] = useState<SidebarTab>('chats');
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
    contacts,
    isLoadingMessages,
    fetchSessions,
    fetchChats,
    fetchContacts,
    fetchMessages,
    sendMessage,
    sendMedia,
    connectSession,
    setActiveChat,
  } = useChatStore();

  // Initialize WebSocket event handling
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

  // Load sessions on mount
  useEffect(() => {
    fetchSessions();
  }, [fetchSessions]);

  // Load chats when sessions change
  useEffect(() => {
    if (sessions.length > 0) {
      fetchChats(sessions[0].id);
      fetchContacts(sessions[0].id);
    }
  }, [sessions, fetchChats, fetchContacts]);

  // Load messages when chat selected
  useEffect(() => {
    if (selectedChatId) {
      fetchMessages(selectedChatId);
    }
  }, [selectedChatId, fetchMessages]);

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSelectChat = useCallback((chatId: string) => {
    setSelectedChatId(chatId);
    setReplyTo(null);
  }, []);

  useEffect(() => {
    const chat = chats.find((c) => c.id === selectedChatId) || null;
    setActiveChat(chat);
  }, [selectedChatId, chats, setActiveChat]);

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

  const handleConnectSession = async () => {
    const name = `Personal-${Date.now()}`;
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

  return (
    <div className="flex h-full overflow-hidden">
      {/* Sidebar */}
      <Sidebar
        activeTab={activeTab}
        onTabChange={setActiveTab}
        mode="personal"
        isConnected={sessions.some((s) => s.status === 'connected')}
        onBack={() => navigate('/mode-select')}
      />

      {/* Chat list panel */}
      <div className="w-80 shrink-0">
        {activeTab === 'settings' ? (
          <SettingsPanel onConnectSession={handleConnectSession} />
        ) : (
          <>
            <div className="flex h-14 items-center justify-between border-b border-gray-200 bg-wa-header px-4 dark:border-gray-700 dark:bg-wa-dark-header">
              <h2 className="text-lg font-semibold text-white">Chats</h2>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => sessions[0] && useChatStore.getState().triggerSync(sessions[0].id)}
                  className="rounded-full p-2 text-white/80 hover:bg-white/10"
                  title="Sync history"
                >
                  <RefreshCw size={18} />
                </button>
                <button
                  onClick={handleConnectSession}
                  className="rounded-full p-2 text-white/80 hover:bg-white/10"
                  title="New session"
                >
                  <Plus size={18} />
                </button>
              </div>
            </div>
            <ChatList
              chats={chats}
              selectedChatId={selectedChatId}
              onSelectChat={handleSelectChat}
              searchQuery={searchQuery}
              onSearchChange={setSearchQuery}
              contacts={contacts}
            />
          </>
        )}
      </div>

      {/* Chat area */}
      <div className="flex min-w-0 flex-1 flex-col">
        {selectedChat ? (
          <>
            {/* Chat header */}
            <div className="flex h-14 items-center gap-3 border-b border-gray-200 bg-gray-50 px-4 dark:border-gray-700 dark:bg-wa-dark-header">
              <div className="flex h-10 w-10 items-center justify-center rounded-full bg-gray-300 text-white dark:bg-gray-600">
                {selectedChat.profile_pic_url ? (
                  <img src={selectedChat.profile_pic_url} alt="" className="h-full w-full rounded-full object-cover" />
                ) : (
                  <span className="font-semibold">
                    {getChatDisplayName(selectedChat, contacts)[0]?.toUpperCase() || '?'}
                  </span>
                )}
              </div>
              <div className="min-w-0 flex-1">
                <h3 className="truncate font-medium text-gray-900 dark:text-white">
                  {getChatDisplayName(selectedChat, contacts)}
                </h3>
                <p className="truncate text-xs text-gray-500">
                  {selectedChat.is_group ? 'Group' : selectedChat.phone_number || 'Online'}
                </p>
              </div>
            </div>

            {/* Messages */}
            <div className="chat-bg flex-1 overflow-y-auto py-4 scrollbar-thin">
              {isLoadingMessages ? (
                <div className="flex h-full items-center justify-center">
                  <Loader2 size={32} className="animate-spin text-wa-green" />
                </div>
              ) : currentMessages.length === 0 ? (
                <div className="flex h-full flex-col items-center justify-center text-gray-400">
                  <p>No messages yet</p>
                  <p className="text-sm">Send a message to start the conversation</p>
                </div>
              ) : (
                <>
                  {currentMessages.map((msg) => (
                    <ChatBubble
                      key={msg.id}
                      message={msg}
                      chatId={selectedChatId!}
                      showSender={selectedChat.is_group}
                      contacts={contacts}
                      onReply={setReplyTo}
                    />
                  ))}
                  <div ref={messagesEndRef} />
                </>
              )}
            </div>

            {/* Input */}
            <MessageInput
              onSend={handleSendMessage}
              onSendMedia={handleSendMedia}
              replyTo={replyTo}
              onCancelReply={() => setReplyTo(null)}
              disabled={!sessions.some((s) => s.status === 'connected')}
            />
          </>
        ) : (
          /* Empty state */
          <div className="flex h-full flex-col items-center justify-center bg-gray-50 dark:bg-wa-dark-bg">
            <div className="mb-4 flex h-24 w-24 items-center justify-center rounded-full bg-wa-green/10">
              <img
                src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%2325D366' stroke-width='1.5'%3E%3Cpath d='M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z'/%3E%3C/svg%3E"
                alt=""
                className="h-16 w-16"
              />
            </div>
            <h3 className="text-xl font-light text-gray-600 dark:text-gray-300">WhatsApp Web</h3>
            <p className="mt-2 max-w-sm text-center text-sm text-gray-400">
              Send and receive messages without keeping your phone online.
              Select a chat to start messaging.
            </p>
            {sessions.length === 0 && (
              <button
                onClick={handleConnectSession}
                className="mt-6 flex items-center gap-2 rounded-lg bg-wa-green px-6 py-2.5 text-sm font-medium text-white shadow-md hover:bg-wa-green/90"
              >
                <Plus size={18} />
                Connect WhatsApp
              </button>
            )}
          </div>
        )}
      </div>

      {/* QR Code Modal */}
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
