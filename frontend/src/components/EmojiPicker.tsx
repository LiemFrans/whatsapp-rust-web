import { useState, useMemo } from 'react';

interface EmojiPickerProps {
  onSelect: (emoji: string) => void;
  onClose: () => void;
}

const EMOJI_CATEGORIES: Record<string, string[]> = {
  'Smileys': [
    '😀', '😃', '😄', '😁', '😆', '😅', '🤣', '😂', '🙂', '😊',
    '😇', '🥰', '😍', '🤩', '😘', '😗', '😚', '😙', '🥲', '😋',
    '😛', '😜', '🤪', '😝', '🤑', '🤗', '🤭', '🤫', '🤔', '🫡',
    '🤐', '🤨', '😐', '😑', '😶', '🫥', '😏', '😒', '🙄', '😬',
    '😮‍💨', '🤥', '😌', '😔', '😪', '🤤', '😴', '😷', '🤒', '🤕',
    '🤢', '🤮', '🥵', '🥶', '🥴', '😵', '🤯', '🤠', '🥳', '🥸',
    '😎', '🤓', '🧐', '😕', '🫤', '😟', '🙁', '😮', '😯', '😲',
    '😳', '🥺', '🥹', '😦', '😧', '😨', '😰', '😥', '😢', '😭',
    '😱', '😖', '😣', '😞', '😓', '😩', '😫', '🥱', '😤', '😡',
    '😠', '🤬', '😈', '👿', '💀', '☠️', '💩', '🤡', '👹', '👺',
  ],
  'Gestures': [
    '👋', '🤚', '🖐️', '✋', '🖖', '🫱', '🫲', '🫳', '🫴', '👌',
    '🤌', '🤏', '✌️', '🤞', '🫰', '🤟', '🤘', '🤙', '👈', '👉',
    '👆', '🖕', '👇', '☝️', '🫵', '👍', '👎', '✊', '👊', '🤛',
    '🤜', '👏', '🙌', '🫶', '👐', '🤲', '🤝', '🙏', '💪', '🦾',
  ],
  'Hearts': [
    '❤️', '🧡', '💛', '💚', '💙', '💜', '🖤', '🤍', '🤎', '💔',
    '❤️‍🔥', '❤️‍🩹', '❣️', '💕', '💞', '💓', '💗', '💖', '💘', '💝',
    '💟', '♥️', '💋', '💯', '🔥', '✨', '⭐', '🌟', '💫', '🎉',
  ],
  'Objects': [
    '📱', '💻', '⌨️', '🖥️', '🖨️', '📷', '📸', '📹', '🎥', '📼',
    '🔍', '🔎', '📝', '✏️', '📎', '📌', '📁', '📂', '📊', '📈',
    '🗓️', '📅', '🔔', '🔕', '📣', '📢', '💡', '🔧', '🔨', '⚙️',
  ],
};

export default function EmojiPicker({ onSelect, onClose }: EmojiPickerProps) {
  const [activeCategory, setActiveCategory] = useState('Smileys');
  const [search, setSearch] = useState('');

  const categories = useMemo(() => Object.keys(EMOJI_CATEGORIES), []);
  const emojis = EMOJI_CATEGORIES[activeCategory] || [];

  const filteredEmojis = search
    ? Object.values(EMOJI_CATEGORIES).flat()
    : emojis;

  return (
    <div className="absolute bottom-full left-0 z-20 mb-2 w-80 rounded-xl border border-gray-200 bg-white shadow-xl dark:border-gray-600 dark:bg-gray-800">
      {/* Header */}
      <div className="flex items-center gap-2 border-b border-gray-200 px-3 py-2 dark:border-gray-600">
        <input
          type="text"
          placeholder="Search emoji..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="flex-1 rounded-lg bg-gray-100 px-3 py-1.5 text-sm outline-none dark:bg-gray-700 dark:text-gray-200"
          autoFocus
        />
        <button
          onClick={onClose}
          className="text-xs text-gray-400 hover:text-gray-600"
        >
          ✕
        </button>
      </div>

      {/* Category tabs */}
      {!search && (
        <div className="flex gap-1 overflow-x-auto border-b border-gray-200 px-2 py-1 dark:border-gray-600">
          {categories.map((cat) => (
            <button
              key={cat}
              onClick={() => setActiveCategory(cat)}
              className={`shrink-0 rounded-md px-2 py-1 text-xs transition-colors ${
                activeCategory === cat
                  ? 'bg-wa-green/10 font-medium text-wa-green'
                  : 'text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-700'
              }`}
            >
              {cat}
            </button>
          ))}
        </div>
      )}

      {/* Emoji grid */}
      <div className="grid max-h-48 grid-cols-8 gap-0.5 overflow-y-auto p-2 scrollbar-thin">
        {filteredEmojis.map((emoji, i) => (
          <button
            key={`${emoji}-${i}`}
            onClick={() => {
              onSelect(emoji);
              onClose();
            }}
            className="flex h-9 w-9 items-center justify-center rounded-md text-xl hover:bg-gray-100 dark:hover:bg-gray-700"
          >
            {emoji}
          </button>
        ))}
      </div>
    </div>
  );
}
