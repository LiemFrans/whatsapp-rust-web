import { useMemo } from 'react';
import {
  Check,
  CheckCheck,
  Clock,
  Download,
  Play,
  FileText,
  MapPin,
  Reply,
  Star,
  MoreVertical,
} from 'lucide-react';
import type { Message } from '@/types';
import { cleanSenderDisplay, formatMentions } from '@/utils/chat';

interface ChatBubbleProps {
  message: Message;
  showSender?: boolean;
  onReply?: (message: Message) => void;
  onStar?: (messageId: string) => void;
}

export default function ChatBubble({ message, showSender, onReply, onStar }: ChatBubbleProps) {
  const isMe = message.is_from_me;
  const parsedDate = new Date(message.timestamp);
  const time = isNaN(parsedDate.getTime())
    ? ''
    : parsedDate.getTime() < 86400000 // epoch 0 + 1 day
      ? ''
      : parsedDate.toLocaleTimeString([], {
          hour: '2-digit',
          minute: '2-digit',
        });

  const statusIcon = useMemo(() => {
    if (!isMe) return null;
    switch (message.status) {
      case 'pending':
        return <Clock size={12} className="text-gray-400" />;
      case 'sent':
        return <Check size={12} className="text-gray-400" />;
      case 'delivered':
        return <CheckCheck size={12} className="text-gray-400" />;
      case 'read':
        return <CheckCheck size={12} className="text-blue-400" />;
      default:
        return null;
    }
  }, [isMe, message.status]);

  const renderContent = () => {
    switch (message.message_type) {
      case 'image':
        return (
          <div className="max-w-xs overflow-hidden rounded-lg">
            {message.media_url ? (
              <img
                src={message.media_url}
                alt="Image"
                className="w-full cursor-pointer object-cover"
                loading="lazy"
              />
            ) : (
              <div className="flex h-48 w-64 items-center justify-center rounded-lg bg-gray-200 dark:bg-gray-600">
                <div className="text-center">
                  <Download size={24} className="mx-auto text-gray-400" />
                  <span className="mt-1 block text-xs text-gray-500">📷 Photo</span>
                </div>
              </div>
            )}
            {message.content && (
              <p className="mt-1 whitespace-pre-wrap text-sm">{formatMentions(message.content)}</p>
            )}
          </div>
        );

      case 'video':
        return (
          <div className="max-w-xs overflow-hidden rounded-lg">
            <div className="relative">
              {message.media_url ? (
                <video src={message.media_url} className="w-full" controls preload="metadata" />
              ) : (
                <div className="flex h-48 w-64 items-center justify-center rounded-lg bg-gray-200 dark:bg-gray-600">
                  <div className="text-center">
                    <Play size={36} className="mx-auto text-gray-400" />
                    <span className="mt-1 block text-xs text-gray-500">🎥 Video</span>
                  </div>
                </div>
              )}
            </div>
            {message.content && (
              <p className="mt-1 whitespace-pre-wrap text-sm">{formatMentions(message.content)}</p>
            )}
          </div>
        );

      case 'audio':
      case 'voice':
        return (
          <div className="flex w-64 items-center gap-3 py-1">
            <button className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-wa-green text-white">
              <Play size={18} />
            </button>
            <div className="flex-1">
              <div className="h-1 w-full rounded-full bg-gray-300 dark:bg-gray-500">
                <div className="h-1 w-0 rounded-full bg-wa-green" />
              </div>
              <span className="mt-1 text-xs text-gray-500">0:00</span>
            </div>
          </div>
        );

      case 'document':
        return (
          <a
            href={message.media_url || '#'}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-3 rounded-lg bg-white/30 p-3 dark:bg-black/10"
          >
            <FileText size={28} className="shrink-0 text-wa-green" />
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium">{message.content || 'Document'}</p>
              <p className="text-xs text-gray-500">{message.media_mime_type || 'file'}</p>
            </div>
            <Download size={18} className="shrink-0 text-gray-400" />
          </a>
        );

      case 'location':
        return (
          <div className="max-w-xs overflow-hidden rounded-lg">
            <div className="flex h-32 w-64 items-center justify-center bg-gray-200 dark:bg-gray-600">
              <MapPin size={36} className="text-wa-green" />
            </div>
            {message.content && (
              <p className="mt-1 whitespace-pre-wrap text-sm">{formatMentions(message.content)}</p>
            )}
          </div>
        );

      case 'unknown':
        return (
          <div className="rounded-lg bg-gray-100 px-3 py-2 dark:bg-gray-700">
            <p className="text-xs italic text-gray-500">
              ⚠️ Unsupported message type
            </p>
            {message.content && (
              <p className="mt-1 whitespace-pre-wrap text-sm">{formatMentions(message.content)}</p>
            )}
          </div>
        );

      case 'sticker':
        return (
          <div className="h-32 w-32">
            {message.media_url ? (
              <img src={message.media_url} alt="Sticker" className="h-full w-full object-contain" />
            ) : (
              <div className="flex h-full w-full items-center justify-center rounded-lg bg-gray-100 dark:bg-gray-700">
                <span className="text-5xl">🏷️</span>
              </div>
            )}
          </div>
        );

      case 'text':
      default:
        return (
          <p className="whitespace-pre-wrap text-sm leading-relaxed">
            {message.content ? formatMentions(message.content) : ''}
          </p>
        );
    }
  };

  if (message.is_deleted) {
    return (
      <div className={`flex ${isMe ? 'justify-end' : 'justify-start'} px-4 py-0.5`}>
        <div className="rounded-lg bg-white/50 px-3 py-2 italic text-gray-400 dark:bg-gray-800/50">
          <span className="text-sm">🚫 This message was deleted</span>
        </div>
      </div>
    );
  }

  return (
    <div className={`group flex ${isMe ? 'justify-end' : 'justify-start'} px-4 py-0.5`}>
      <div
        className={`relative max-w-[65%] rounded-lg px-3 py-1.5 shadow-sm ${
          isMe
            ? 'bg-wa-bubble-out text-gray-900 dark:bg-wa-dark-bubble-out dark:text-gray-100'
            : 'bg-white text-gray-900 dark:bg-wa-dark-bubble-in dark:text-gray-100'
        } ${isMe ? 'bubble-tail-right' : 'bubble-tail-left'}`}
      >
        {/* Reply reference */}
        {message.reply_to && (
          <div className="mb-1 rounded border-l-4 border-wa-green bg-black/5 px-2 py-1 dark:bg-white/5">
            <p className="truncate text-xs text-wa-green">Replied message</p>
          </div>
        )}

        {/* Sender name (groups) */}
        {showSender && !isMe && (
          <p className="mb-0.5 text-xs font-semibold text-wa-green">{cleanSenderDisplay(message.sender_name, message.sender)}</p>
        )}

        {/* Starred indicator */}
        {message.is_starred && (
          <Star size={10} className="absolute -left-1 -top-1 fill-yellow-400 text-yellow-400" />
        )}

        {/* Message content */}
        {renderContent()}

        {/* Time + status */}
        <div
          className={`mt-0.5 flex items-center justify-end gap-1 ${
            message.message_type === 'sticker' ? '' : ''
          }`}
        >
          <span className="text-[10px] text-gray-500 dark:text-gray-400">{time}</span>
          {statusIcon}
        </div>

        {/* Hover actions */}
        <div className="invisible absolute -top-2 right-0 flex items-center gap-0.5 rounded bg-white px-1 py-0.5 shadow group-hover:visible dark:bg-gray-700">
          {onReply && (
            <button onClick={() => onReply(message)} className="rounded p-0.5 hover:bg-gray-100 dark:hover:bg-gray-600">
              <Reply size={14} className="text-gray-500" />
            </button>
          )}
          {onStar && (
            <button onClick={() => onStar(message.id)} className="rounded p-0.5 hover:bg-gray-100 dark:hover:bg-gray-600">
              <Star size={14} className="text-gray-500" />
            </button>
          )}
          <button className="rounded p-0.5 hover:bg-gray-100 dark:hover:bg-gray-600">
            <MoreVertical size={14} className="text-gray-500" />
          </button>
        </div>
      </div>
    </div>
  );
}
