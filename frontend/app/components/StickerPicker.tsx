"use client";

import { useCallback, useEffect, useRef, useState } from "react";

export interface StickerInfo {
  chatJid: string;
  messageId: string;
  downloadPath: string;
  isAnimated: boolean;
}

interface StickerPickerProps {
  stickers: StickerInfo[];
  onSelect: (sticker: StickerInfo) => void;
}

export function StickerPicker({ stickers, onSelect }: StickerPickerProps) {
  const [search, setSearch] = useState("");
  const [view, setView] = useState<"recent" | "all">("recent");
  const searchRef = useRef<HTMLInputElement>(null);

  // Popular static sticker shortcuts (Unicode emoji stickers rendered as text)
  const STICKER_SHORTCUTS: { label: string; emoji: string }[] = [
    { label: "Thumbs Up", emoji: "👍" },
    { label: "Laugh", emoji: "😂" },
    { label: "Heart", emoji: "❤️" },
    { label: "Fire", emoji: "🔥" },
    { label: "Pray", emoji: "🙏" },
    { label: "Clap", emoji: "👏" },
    { label: "Party", emoji: "🎉" },
    { label: "Star", emoji: "⭐" },
    { label: "OK", emoji: "👌" },
    { label: "Wave", emoji: "👋" },
    { label: "Muscle", emoji: "💪" },
    { label: "100", emoji: "💯" },
  ];

  return (
    <div className="flex h-[340px] flex-col">
      {/* View tabs */}
      <div className="flex items-center gap-0.5 border-b border-wa-border px-2 py-1">
        <button
          type="button"
          onClick={() => setView("recent")}
          className={`flex h-9 items-center gap-1.5 rounded-lg px-3 text-sm transition-colors ${
            view === "recent"
              ? "bg-wa-teal/10 font-medium text-wa-teal"
              : "text-wa-icon hover:bg-gray-100"
          }`}
        >
          🕐 Recent
        </button>
        <button
          type="button"
          onClick={() => setView("all")}
          className={`flex h-9 items-center gap-1.5 rounded-lg px-3 text-sm transition-colors ${
            view === "all"
              ? "bg-wa-teal/10 font-medium text-wa-teal"
              : "text-wa-icon hover:bg-gray-100"
          }`}
        >
          ⭐ Favourites
        </button>
        <div className="flex-1" />
        <span className="text-[10px] text-wa-text-secondary">
          {stickers.length} sticker{stickers.length !== 1 ? "s" : ""}
        </span>
      </div>

      {/* Search */}
      <div className="px-3 py-2">
        <div className="flex items-center gap-2 rounded-lg bg-wa-input-bg px-3 py-1.5">
          <svg viewBox="0 0 24 24" width="16" height="16" className="shrink-0 fill-wa-icon">
            <path d="M15.009 13.805h-.636l-.22-.219a5.184 5.184 0 0 0 1.256-3.386 5.207 5.207 0 1 0-5.207 5.208 5.183 5.183 0 0 0 3.385-1.255l.221.22v.635l4.004 3.999 1.194-1.195-3.997-4.007zm-4.808 0a3.6 3.6 0 1 1 0-7.2 3.6 3.6 0 0 1 0 7.2z" />
          </svg>
          <input
            ref={searchRef}
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search stickers"
            className="min-w-0 flex-1 bg-transparent text-sm text-wa-text outline-none placeholder:text-wa-text-secondary"
          />
          {search && (
            <button
              type="button"
              onClick={() => setSearch("")}
              className="text-xs text-wa-text-secondary hover:text-wa-text"
            >
              ✕
            </button>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-2 pb-2">
        {view === "all" || stickers.length === 0 ? (
          <>
            {/* Sticker shortcuts as large emoji */}
            <div className="mb-3">
              <div className="px-1 py-1.5 text-xs font-medium text-wa-text-secondary">
                Quick Stickers
              </div>
              <div className="grid grid-cols-6 gap-1">
                {STICKER_SHORTCUTS.filter((s) =>
                  !search || s.label.toLowerCase().includes(search.toLowerCase()),
                ).map((shortcut) => (
                  <button
                    key={shortcut.label}
                    type="button"
                    onClick={() =>
                      onSelect({
                        chatJid: "",
                        messageId: "",
                        downloadPath: "",
                        isAnimated: false,
                      })
                    }
                    className="flex h-16 w-full items-center justify-center rounded-xl text-4xl transition-all hover:scale-110 hover:bg-gray-100"
                    title={shortcut.label}
                  >
                    {shortcut.emoji}
                  </button>
                ))}
              </div>
            </div>

            {stickers.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-8 text-center text-sm text-wa-text-secondary">
                <div className="mb-2 text-4xl">🪄</div>
                <p className="font-medium text-wa-text">No stickers yet</p>
                <p className="mt-1 text-xs">
                  Stickers from received messages will appear here
                </p>
              </div>
            ) : null}
          </>
        ) : null}

        {/* Received stickers grid */}
        {stickers.length > 0 && view === "recent" ? (
          <div className="grid grid-cols-4 gap-2">
            {stickers.map((sticker, idx) => (
              <button
                key={`${sticker.chatJid}-${sticker.messageId}-${idx}`}
                type="button"
                onClick={() => onSelect(sticker)}
                className="group flex aspect-square items-center justify-center overflow-hidden rounded-xl bg-gray-50 p-1.5 transition-all hover:scale-105 hover:bg-gray-100"
                title="Send sticker"
              >
                <img
                  src={sticker.downloadPath}
                  alt="Sticker"
                  className="h-full w-full object-contain"
                  loading="lazy"
                />
              </button>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}
