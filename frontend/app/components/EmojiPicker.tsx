"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { EMOJI_CATEGORIES, EMOJI_SEARCH_INDEX, type EmojiCategory } from "../lib/emoji-data";

const RECENT_KEY = "wa-recent-emoji";
const MAX_RECENT = 32;

function loadRecent(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function saveRecent(list: string[]) {
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(list.slice(0, MAX_RECENT)));
  } catch { /* quota exceeded — ignore */ }
}

interface EmojiPickerProps {
  onSelect: (emoji: string) => void;
}

export function EmojiPicker({ onSelect }: EmojiPickerProps) {
  const [search, setSearch] = useState("");
  const [activeCategory, setActiveCategory] = useState("recent");
  const [recent, setRecent] = useState<string[]>(loadRecent);
  const scrollRef = useRef<HTMLDivElement>(null);
  const categoryRefs = useRef<Record<string, HTMLDivElement | null>>({});
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    searchRef.current?.focus();
  }, []);

  const categories = useMemo<EmojiCategory[]>(() => {
    const cats = [...EMOJI_CATEGORIES];
    const recentCat = cats.find((c) => c.id === "recent");
    if (recentCat) recentCat.emojis = recent;
    return cats;
  }, [recent]);

  const searchResults = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return null;
    const matches = new Set<string>();
    // Search index
    for (const [keyword, emojis] of Object.entries(EMOJI_SEARCH_INDEX)) {
      if (keyword.includes(q)) {
        for (const e of emojis) matches.add(e);
      }
    }
    // Also search all emojis character-by-character (for emoji search like pasting an emoji)
    for (const cat of EMOJI_CATEGORIES) {
      for (const emoji of cat.emojis) {
        if (emoji.includes(q)) matches.add(emoji);
      }
    }
    return Array.from(matches);
  }, [search]);

  const handleSelect = useCallback(
    (emoji: string) => {
      onSelect(emoji);
      setRecent((prev) => {
        const next = [emoji, ...prev.filter((e) => e !== emoji)].slice(0, MAX_RECENT);
        saveRecent(next);
        return next;
      });
    },
    [onSelect],
  );

  const scrollToCategory = useCallback((id: string) => {
    setActiveCategory(id);
    setSearch("");
    const el = categoryRefs.current[id];
    if (el && scrollRef.current) {
      scrollRef.current.scrollTo({
        top: el.offsetTop - scrollRef.current.offsetTop - 8,
        behavior: "smooth",
      });
    }
  }, []);

  const handleScroll = useCallback(() => {
    const container = scrollRef.current;
    if (!container || search) return;
    const scrollTop = container.scrollTop + container.offsetTop + 16;
    let current = "recent";
    for (const cat of categories) {
      const el = categoryRefs.current[cat.id];
      if (el && el.offsetTop <= scrollTop) {
        current = cat.id;
      }
    }
    setActiveCategory(current);
  }, [categories, search]);

  return (
    <div className="flex h-[340px] flex-col">
      {/* Category tabs */}
      <div className="flex items-center gap-0.5 border-b border-wa-border px-2 py-1">
        {categories.map((cat) => (
          <button
            key={cat.id}
            type="button"
            onClick={() => scrollToCategory(cat.id)}
            className={`flex h-9 w-9 items-center justify-center rounded-lg text-base transition-colors ${
              activeCategory === cat.id
                ? "bg-wa-teal/10 text-wa-teal"
                : "text-wa-icon hover:bg-gray-100"
            }`}
            title={cat.label}
          >
            {cat.icon}
          </button>
        ))}
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
            placeholder="Search emoji"
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

      {/* Emoji grid */}
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto px-2 pb-2"
      >
        {searchResults !== null ? (
          searchResults.length === 0 ? (
            <div className="flex h-full items-center justify-center text-sm text-wa-text-secondary">
              No emoji found
            </div>
          ) : (
            <div className="grid grid-cols-8 gap-0.5">
              {searchResults.map((emoji) => (
                <button
                  key={emoji}
                  type="button"
                  onClick={() => handleSelect(emoji)}
                  className="flex h-10 w-10 items-center justify-center rounded-lg text-2xl transition-colors hover:bg-gray-100"
                >
                  {emoji}
                </button>
              ))}
            </div>
          )
        ) : (
          categories.map((cat) =>
            cat.emojis.length === 0 && cat.id === "recent" ? (
              <div
                key={cat.id}
                ref={(el) => { categoryRefs.current[cat.id] = el; }}
              >
                <div className="sticky top-0 z-10 bg-white px-1 py-1.5 text-xs font-medium text-wa-text-secondary">
                  {cat.label}
                </div>
                <div className="py-4 text-center text-xs text-wa-text-secondary">
                  Your recently used emoji will appear here
                </div>
              </div>
            ) : (
              <div
                key={cat.id}
                ref={(el) => { categoryRefs.current[cat.id] = el; }}
              >
                <div className="sticky top-0 z-10 bg-white px-1 py-1.5 text-xs font-medium text-wa-text-secondary">
                  {cat.label}
                </div>
                <div className="grid grid-cols-8 gap-0.5">
                  {cat.emojis.map((emoji, idx) => (
                    <button
                      key={`${cat.id}-${idx}`}
                      type="button"
                      onClick={() => handleSelect(emoji)}
                      className="flex h-10 w-10 items-center justify-center rounded-lg text-2xl transition-colors hover:bg-gray-100"
                    >
                      {emoji}
                    </button>
                  ))}
                </div>
              </div>
            ),
          )
        )}
      </div>
    </div>
  );
}
