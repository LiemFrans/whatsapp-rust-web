import { useMemo } from 'react';
import { formatDistanceToNow } from 'date-fns';
import {
  Search,
  Archive,
  Pin,
  BellOff,
  Check,
  CheckCheck,
  Image,
  Video,
  Mic,
  FileText,
  MapPin,
  Contact,
} from 'lucide-react';
import type { Chat } from '@/types';
import type { ContactsMap } from '@/store/chatStore';
import { getChatDisplayName } from '@/utils/chat';

interface ChatListProps {
  chats: Chat[];
  selectedChatId: string | null;
  onSelectChat: (chatId: string) => void;
  searchQuery: string;
  onSearchChange: (query: string) => void;
  contacts?: ContactsMap;
}

export default function ChatList({
  chats,
  selectedChatId,
  onSelectChat,
  searchQuery,
  onSearchChange,
  contacts,
}: ChatListProps) {
  const filteredChats = useMemo(() => {
    if (!searchQuery.trim()) return chats;
    const q = searchQuery.toLowerCase();
    return chats.filter(
      (c) =>
        c.name?.toLowerCase().includes(q) ||
        c.phone_number?.toLowerCase().includes(q) ||
        c.last_message?.toLowerCase().includes(q)
    );
  }, [chats, searchQuery]);

  return (
    <div className="flex h-full flex-col border-r border-gray-200 dark:border-gray-700">
      {/* Search */}
      <div className="p-2">
        <div className="relative">
          <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
          <input
            type="text"
            placeholder="Search or start new chat"
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            className="w-full rounded-lg bg-gray-100 py-2 pl-10 pr-4 text-sm text-gray-800 placeholder-gray-500 outline-none focus:ring-2 focus:ring-wa-green/30 dark:bg-gray-700 dark:text-gray-200 dark:placeholder-gray-400"
          />
        </div>
      </div>

      {/* Chat list */}
      <div className="flex-1 overflow-y-auto scrollbar-thin">
        {filteredChats.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 text-gray-400">
            <Search size={48} className="mb-2 opacity-50" />
            <p className="text-sm">No chats found</p>
          </div>
        ) : (
          filteredChats.map((chat) => (
            <ChatListItem
              key={chat.id}
              chat={chat}
              isSelected={chat.id === selectedChatId}
              onClick={() => onSelectChat(chat.id)}
              contacts={contacts}
            />
          ))
        )}
      </div>
    </div>
  );
}

function ChatListItem({
  chat,
  isSelected,
  onClick,
  contacts,
}: {
  chat: Chat;
  isSelected: boolean;
  onClick: () => void;
  contacts?: ContactsMap;
}) {
  const lastMsgTime = (() => {
    if (!chat.last_message_at) return '';
    const d = new Date(chat.last_message_at);
    if (isNaN(d.getTime()) || d.getTime() < 86400000) return '';
    return formatDistanceToNow(d, { addSuffix: false });
  })();

  const statusIcon = useMemo(() => {
    if (!chat.last_message_status || !chat.is_last_message_from_me) return null;
    switch (chat.last_message_status) {
      case 'sent':
        return <Check size={14} className="text-gray-400" />;
      case 'delivered':
        return <CheckCheck size={14} className="text-gray-400" />;
      case 'read':
        return <CheckCheck size={14} className="text-blue-500" />;
      default:
        return null;
    }
  }, [chat.last_message_status, chat.is_last_message_from_me]);

  const mediaPrefix = useMemo(() => {
    if (!chat.last_message_type || chat.last_message_type === 'text') return null;
    const icons: Record<string, React.ReactNode> = {
      image: <Image size={14} className="shrink-0" />,
      video: <Video size={14} className="shrink-0" />,
      audio: <Mic size={14} className="shrink-0" />,
      document: <FileText size={14} className="shrink-0" />,
      location: <MapPin size={14} className="shrink-0" />,
      contact: <Contact size={14} className="shrink-0" />,
    };
    return icons[chat.last_message_type] || null;
  }, [chat.last_message_type]);

  return (
    <button
      onClick={onClick}
      className={`flex w-full items-center gap-3 px-3 py-3 text-left transition-colors hover:bg-gray-50 dark:hover:bg-gray-800 ${
        isSelected ? 'bg-wa-green/5 dark:bg-wa-green/10' : ''
      }`}
    >
      {/* Avatar */}
      <div className="relative shrink-0">
        <div className="flex h-12 w-12 items-center justify-center overflow-hidden rounded-full bg-gray-300 text-white dark:bg-gray-600">
          {chat.profile_pic_url ? (
            <img src={chat.profile_pic_url} alt="" className="h-full w-full object-cover" />
          ) : (
            <span className="text-lg font-semibold">
              {getChatDisplayName(chat, contacts)[0]?.toUpperCase() || '?'}
            </span>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="min-w-0 flex-1">
        <div className="flex items-center justify-between gap-2">
          <h3 className="text-sm font-medium text-gray-900 dark:text-white break-words leading-tight">
            {getChatDisplayName(chat, contacts)}
          </h3>
          <span className="shrink-0 text-xs text-gray-500">{lastMsgTime}</span>
        </div>
        <div className="mt-0.5 flex items-center justify-between gap-2">
          <div className="flex min-w-0 items-center gap-1 text-sm text-gray-500 dark:text-gray-400">
            {statusIcon}
            {mediaPrefix}
            <span className="truncate">
              {chat.last_message || (chat.last_message_type ? chat.last_message_type : '')}
            </span>
          </div>
          <div className="flex shrink-0 items-center gap-1">
            {chat.is_pinned && <Pin size={12} className="text-gray-400" />}
            {chat.is_muted && <BellOff size={12} className="text-gray-400" />}
            {chat.is_archived && <Archive size={12} className="text-gray-400" />}
            {chat.unread_count > 0 && (
              <span className="flex h-5 min-w-5 items-center justify-center rounded-full bg-wa-green px-1 text-xs font-bold text-white">
                {chat.unread_count > 99 ? '99+' : chat.unread_count}
              </span>
            )}
          </div>
        </div>
      </div>
    </button>
  );
}
