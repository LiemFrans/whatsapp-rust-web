import { useState, useRef, useEffect, type FormEvent } from 'react';
import {
  Send,
  Paperclip,
  Smile,
  Mic,
  X,
  Image,
  FileText,
  Camera,
  Reply,
} from 'lucide-react';
import type { Message } from '@/types';
import EmojiPicker from './EmojiPicker';

interface MessageInputProps {
  onSend: (content: string, type?: string) => void;
  onSendMedia?: (file: File, type: 'image' | 'document', caption?: string) => void;
  replyTo?: Message | null;
  onCancelReply?: () => void;
  disabled?: boolean;
  quickReplies?: { shortcut: string; content: string }[];
}

export default function MessageInput({
  onSend,
  onSendMedia,
  replyTo,
  onCancelReply,
  disabled,
  quickReplies,
}: MessageInputProps) {
  const [text, setText] = useState('');
  const [showAttach, setShowAttach] = useState(false);
  const [showEmoji, setShowEmoji] = useState(false);
  const [showQuickReplies, setShowQuickReplies] = useState(false);
  const [filteredQR, setFilteredQR] = useState<typeof quickReplies>([]);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const imageInputRef = useRef<HTMLInputElement>(null);
  const docInputRef = useRef<HTMLInputElement>(null);
  const cameraInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (replyTo && inputRef.current) {
      inputRef.current.focus();
    }
  }, [replyTo]);

  useEffect(() => {
    if (text.startsWith('/') && quickReplies?.length) {
      const search = text.slice(1).toLowerCase();
      const filtered = quickReplies.filter((qr) =>
        qr.shortcut.toLowerCase().includes(search)
      );
      setFilteredQR(filtered);
      setShowQuickReplies(filtered.length > 0);
    } else {
      setShowQuickReplies(false);
    }
  }, [text, quickReplies]);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!text.trim() || disabled) return;
    onSend(text.trim());
    setText('');
    setShowAttach(false);
    setShowEmoji(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e as unknown as FormEvent);
    }
  };

  const handleQuickReply = (content: string) => {
    onSend(content);
    setText('');
    setShowQuickReplies(false);
  };

  const adjustHeight = () => {
    const textarea = inputRef.current;
    if (textarea) {
      textarea.style.height = 'auto';
      textarea.style.height = Math.min(textarea.scrollHeight, 120) + 'px';
    }
  };

  const handleEmojiSelect = (emoji: string) => {
    const textarea = inputRef.current;
    if (textarea) {
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const newText = text.substring(0, start) + emoji + text.substring(end);
      setText(newText);
      setTimeout(() => {
        textarea.selectionStart = textarea.selectionEnd = start + emoji.length;
        textarea.focus();
      }, 0);
    } else {
      setText(text + emoji);
    }
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>, type: 'image' | 'document') => {
    const file = e.target.files?.[0];
    if (file && onSendMedia) {
      onSendMedia(file, type);
      setShowAttach(false);
    }
    e.target.value = '';
  };

  return (
    <div className="relative border-t border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-wa-dark-bg">
      {/* Hidden file inputs */}
      <input
        ref={imageInputRef}
        type="file"
        accept="image/*"
        className="hidden"
        onChange={(e) => handleFileSelect(e, 'image')}
      />
      <input
        ref={docInputRef}
        type="file"
        accept="*/*"
        className="hidden"
        onChange={(e) => handleFileSelect(e, 'document')}
      />
      <input
        ref={cameraInputRef}
        type="file"
        accept="image/*"
        capture="environment"
        className="hidden"
        onChange={(e) => handleFileSelect(e, 'image')}
      />
      {/* Quick replies dropdown */}
      {showQuickReplies && filteredQR && filteredQR.length > 0 && (
        <div className="absolute bottom-full left-0 right-0 z-10 max-h-48 overflow-y-auto rounded-t-lg border border-gray-200 bg-white shadow-lg dark:border-gray-600 dark:bg-gray-800">
          {filteredQR.map((qr, i) => (
            <button
              key={i}
              onClick={() => handleQuickReply(qr.content)}
              className="flex w-full items-center gap-3 px-4 py-2 text-left text-sm hover:bg-gray-100 dark:hover:bg-gray-700"
            >
              <span className="font-mono text-xs text-wa-green">/{qr.shortcut}</span>
              <span className="truncate text-gray-600 dark:text-gray-300">{qr.content}</span>
            </button>
          ))}
        </div>
      )}

      {/* Emoji picker */}
      {showEmoji && (
        <EmojiPicker
          onSelect={handleEmojiSelect}
          onClose={() => setShowEmoji(false)}
        />
      )}

      {/* Reply preview */}
      {replyTo && (
        <div className="flex items-center gap-2 border-b border-gray-200 bg-white px-4 py-2 dark:border-gray-700 dark:bg-gray-800">
          <Reply size={16} className="shrink-0 text-wa-green" />
          <div className="min-w-0 flex-1 rounded border-l-2 border-wa-green bg-gray-50 px-3 py-1 dark:bg-gray-700">
            <p className="text-xs font-medium text-wa-green">
              {replyTo.is_from_me ? 'You' : replyTo.sender_name || replyTo.sender}
            </p>
            <p className="truncate text-xs text-gray-500 dark:text-gray-400">{replyTo.content}</p>
          </div>
          <button onClick={onCancelReply} className="shrink-0 text-gray-400 hover:text-gray-600">
            <X size={18} />
          </button>
        </div>
      )}

      {/* Attachment menu */}
      {showAttach && (
        <div className="absolute bottom-full left-4 mb-2 flex gap-3 rounded-xl bg-white p-3 shadow-xl dark:bg-gray-800">
          <button
            onClick={() => { imageInputRef.current?.click(); }}
            className="flex h-12 w-12 items-center justify-center rounded-full bg-purple-500 text-white hover:bg-purple-600"
            title="Send image"
          >
            <Image size={22} />
          </button>
          <button
            onClick={() => { docInputRef.current?.click(); }}
            className="flex h-12 w-12 items-center justify-center rounded-full bg-blue-500 text-white hover:bg-blue-600"
            title="Send document"
          >
            <FileText size={22} />
          </button>
          <button
            onClick={() => { cameraInputRef.current?.click(); }}
            className="flex h-12 w-12 items-center justify-center rounded-full bg-pink-500 text-white hover:bg-pink-600"
            title="Take photo"
          >
            <Camera size={22} />
          </button>
        </div>
      )}

      {/* Input area */}
      <form onSubmit={handleSubmit} className="flex items-end gap-2 px-3 py-2">
        <button
          type="button"
          onClick={() => { setShowEmoji(!showEmoji); setShowAttach(false); }}
          className={`mb-1 shrink-0 rounded-full p-2 transition-colors ${
            showEmoji
              ? 'bg-wa-green text-white'
              : 'text-gray-500 hover:bg-gray-200 dark:hover:bg-gray-700'
          }`}
        >
          <Smile size={22} />
        </button>

        <button
          type="button"
          onClick={() => { setShowAttach(!showAttach); setShowEmoji(false); }}
          className={`mb-1 shrink-0 rounded-full p-2 transition-colors ${
            showAttach
              ? 'bg-wa-green text-white'
              : 'text-gray-500 hover:bg-gray-200 dark:hover:bg-gray-700'
          }`}
        >
          <Paperclip size={22} />
        </button>

        <textarea
          ref={inputRef}
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            adjustHeight();
          }}
          onKeyDown={handleKeyDown}
          placeholder="Type a message"
          disabled={disabled}
          rows={1}
          className="max-h-[120px] min-h-[40px] flex-1 resize-none rounded-lg bg-white px-4 py-2.5 text-sm text-gray-800 placeholder-gray-500 outline-none focus:ring-2 focus:ring-wa-green/20 dark:bg-gray-700 dark:text-gray-200 dark:placeholder-gray-400"
        />

        {text.trim() ? (
          <button
            type="submit"
            disabled={disabled}
            className="mb-1 flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-wa-green text-white transition-transform hover:scale-105 hover:bg-wa-green/90 disabled:opacity-50"
          >
            <Send size={18} />
          </button>
        ) : (
          <button
            type="button"
            className="mb-1 flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-wa-green text-white hover:bg-wa-green/90"
          >
            <Mic size={18} />
          </button>
        )}
      </form>
    </div>
  );
}
