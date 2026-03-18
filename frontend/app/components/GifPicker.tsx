"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

// Tenor API v2 (free tier) — Google's official GIF API
const TENOR_API_KEY = "AIzaSyAyimkuYQYF_FXVALexPuGQctUWRURdCYQ"; // Public web key
const TENOR_BASE = "https://tenor.googleapis.com/v2";
const TENOR_LIMIT = 30;

interface TenorGif {
  id: string;
  title: string;
  media_formats: {
    tinygif?: { url: string; dims: [number, number] };
    gif?: { url: string; dims: [number, number] };
    mp4?: { url: string; dims: [number, number] };
    tinymp4?: { url: string; dims: [number, number] };
    nanogif?: { url: string; dims: [number, number] };
  };
}

interface GifPickerProps {
  onSelect: (gifUrl: string, mp4Url: string | null, width: number, height: number) => void;
}

const CATEGORIES = [
  { id: "trending", label: "Trending", icon: "📈" },
  { id: "happy", label: "Happy", icon: "😊" },
  { id: "love", label: "Love", icon: "❤️" },
  { id: "thumbs up", label: "Thumbs Up", icon: "👍" },
  { id: "applause", label: "Applause", icon: "👏" },
  { id: "sad", label: "Sad", icon: "😢" },
  { id: "laugh", label: "LOL", icon: "😂" },
  { id: "wow", label: "Wow", icon: "😮" },
  { id: "hello", label: "Hello", icon: "👋" },
  { id: "bye", label: "Bye", icon: "🫡" },
  { id: "good morning", label: "Morning", icon: "☀️" },
  { id: "good night", label: "Night", icon: "🌙" },
];

async function fetchTenorGifs(query: string, isFeatured = false): Promise<TenorGif[]> {
  try {
    const endpoint = isFeatured ? "featured" : "search";
    const params = new URLSearchParams({
      key: TENOR_API_KEY,
      client_key: "whatsapp_web_clone",
      limit: String(TENOR_LIMIT),
      media_filter: "tinygif,gif,mp4,tinymp4",
    });
    if (!isFeatured) params.set("q", query);
    const res = await fetch(`${TENOR_BASE}/${endpoint}?${params}`);
    if (!res.ok) return [];
    const data = await res.json();
    return data.results ?? [];
  } catch {
    return [];
  }
}

export function GifPicker({ onSelect }: GifPickerProps) {
  const [search, setSearch] = useState("");
  const [activeCategory, setActiveCategory] = useState("trending");
  const [gifs, setGifs] = useState<TenorGif[]>([]);
  const [loading, setLoading] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const loadGifs = useCallback(async (query: string, isFeatured: boolean) => {
    setLoading(true);
    const results = await fetchTenorGifs(query, isFeatured);
    setGifs(results);
    setLoading(false);
  }, []);

  // Initial load — trending
  useEffect(() => {
    void loadGifs("", true);
  }, [loadGifs]);

  // Search with debounce
  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (!search.trim()) {
      if (activeCategory === "trending") {
        void loadGifs("", true);
      } else {
        void loadGifs(activeCategory, false);
      }
      return;
    }
    debounceRef.current = setTimeout(() => {
      void loadGifs(search, false);
    }, 400);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [search, activeCategory, loadGifs]);

  const handleCategoryClick = useCallback(
    (categoryId: string) => {
      setActiveCategory(categoryId);
      setSearch("");
      if (categoryId === "trending") {
        void loadGifs("", true);
      } else {
        void loadGifs(categoryId, false);
      }
    },
    [loadGifs],
  );

  const handleGifClick = useCallback(
    (gif: TenorGif) => {
      const gifMedia = gif.media_formats.gif ?? gif.media_formats.tinygif;
      const mp4Media = gif.media_formats.mp4 ?? gif.media_formats.tinymp4;
      if (!gifMedia) return;
      onSelect(
        gifMedia.url,
        mp4Media?.url ?? null,
        gifMedia.dims[0],
        gifMedia.dims[1],
      );
    },
    [onSelect],
  );

  return (
    <div className="flex h-[340px] flex-col">
      {/* Category tabs */}
      <div className="flex items-center gap-0.5 overflow-x-auto border-b border-wa-border px-2 py-1 scrollbar-none">
        {CATEGORIES.map((cat) => (
          <button
            key={cat.id}
            type="button"
            onClick={() => handleCategoryClick(cat.id)}
            className={`flex h-9 shrink-0 items-center justify-center rounded-lg px-1.5 text-base transition-colors ${
              activeCategory === cat.id && !search
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
            placeholder="Search GIFs via Tenor"
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

      {/* GIF grid */}
      <div className="flex-1 overflow-y-auto px-2 pb-2">
        {loading ? (
          <div className="flex h-full items-center justify-center">
            <div className="h-6 w-6 animate-spin rounded-full border-2 border-gray-200 border-t-wa-teal" />
          </div>
        ) : gifs.length === 0 ? (
          <div className="flex h-full items-center justify-center text-sm text-wa-text-secondary">
            {search ? "No GIFs found" : "Loading GIFs…"}
          </div>
        ) : (
          <div className="columns-2 gap-1.5">
            {gifs.map((gif) => {
              const preview = gif.media_formats.tinygif ?? gif.media_formats.nanogif;
              if (!preview) return null;
              const aspect = preview.dims[1] / preview.dims[0];
              return (
                <button
                  key={gif.id}
                  type="button"
                  onClick={() => handleGifClick(gif)}
                  className="mb-1.5 block w-full overflow-hidden rounded-lg transition-opacity hover:opacity-80"
                  title={gif.title}
                >
                  <img
                    src={preview.url}
                    alt={gif.title}
                    className="w-full rounded-lg object-cover"
                    style={{ aspectRatio: `1 / ${aspect}` }}
                    loading="lazy"
                  />
                </button>
              );
            })}
          </div>
        )}
      </div>

      {/* Tenor attribution */}
      <div className="border-t border-wa-border px-3 py-1.5 text-center text-[10px] text-wa-text-secondary">
        Powered by Tenor
      </div>
    </div>
  );
}
