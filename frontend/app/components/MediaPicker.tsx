"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { EmojiPicker } from "./EmojiPicker";
import { GifPicker } from "./GifPicker";
import { StickerPicker, type StickerInfo } from "./StickerPicker";

export type PickerTab = "emoji" | "gif" | "sticker";

interface MediaPickerProps {
  activeTab: PickerTab;
  onTabChange: (tab: PickerTab) => void;
  onClose: () => void;
  onEmojiSelect: (emoji: string) => void;
  onGifSelect: (gifUrl: string, mp4Url: string | null, width: number, height: number) => void;
  onStickerSelect: (sticker: StickerInfo) => void;
  stickers: StickerInfo[];
}

export function MediaPicker({
  activeTab,
  onTabChange,
  onClose,
  onEmojiSelect,
  onGifSelect,
  onStickerSelect,
  stickers,
}: MediaPickerProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);

  return (
    <div
      ref={containerRef}
      className="mb-2 overflow-hidden rounded-2xl bg-white shadow-lg ring-1 ring-black/5"
    >
      {/* Panel content */}
      <div className="relative">
        {activeTab === "emoji" && <EmojiPicker onSelect={onEmojiSelect} />}
        {activeTab === "gif" && <GifPicker onSelect={onGifSelect} />}
        {activeTab === "sticker" && (
          <StickerPicker stickers={stickers} onSelect={onStickerSelect} />
        )}
      </div>

      {/* Bottom tab bar — like WhatsApp Web */}
      <div className="flex items-center justify-center gap-1 border-t border-wa-border bg-[#f0f2f5] px-4 py-1.5">
        <button
          type="button"
          onClick={() => onTabChange("emoji")}
          className={`flex h-10 w-10 items-center justify-center rounded-xl text-lg transition-colors ${
            activeTab === "emoji"
              ? "bg-white text-wa-teal shadow-sm"
              : "text-wa-icon hover:bg-white/60"
          }`}
          title="Emoji"
        >
          😊
        </button>
        <button
          type="button"
          onClick={() => onTabChange("gif")}
          className={`flex h-10 items-center justify-center rounded-xl px-3 text-xs font-bold transition-colors ${
            activeTab === "gif"
              ? "bg-white text-wa-teal shadow-sm"
              : "text-wa-icon hover:bg-white/60"
          }`}
          title="GIF"
        >
          GIF
        </button>
        <button
          type="button"
          onClick={() => onTabChange("sticker")}
          className={`flex h-10 w-10 items-center justify-center rounded-xl text-lg transition-colors ${
            activeTab === "sticker"
              ? "bg-white text-wa-teal shadow-sm"
              : "text-wa-icon hover:bg-white/60"
          }`}
          title="Stickers"
        >
          {/* Sticker icon (like WhatsApp's folded paper icon) */}
          <svg viewBox="0 0 24 24" width="22" height="22" className="fill-current">
            <path d="M21.8 10.3c-.1-.3-.2-.5-.4-.7l-.1-.1c-.1-.1-.1-.2-.2-.3l-7.3-7.3c-.1-.1-.2-.1-.3-.2l-.1-.1c-.2-.2-.5-.3-.7-.4-.3-.1-.6-.2-.9-.2H5C3.3 1 2 2.3 2 4v16c0 1.7 1.3 3 3 3h14c1.7 0 3-1.3 3-3v-8.8c0-.3-.1-.6-.2-.9zM14 3.4 20.6 10H15c-.6 0-1-.4-1-1V3.4zM20 20c0 .6-.4 1-1 1H5c-.6 0-1-.4-1-1V4c0-.6.4-1 1-1h7v6c0 1.7 1.3 3 3 3h6v8z" />
          </svg>
        </button>
      </div>
    </div>
  );
}
