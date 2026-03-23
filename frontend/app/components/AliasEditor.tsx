"use client";

import { useCallback, useState, type FormEvent } from "react";
import type { ContactAlias, ApiContact, ApiChat } from "../lib/types";

interface AliasEditorProps {
  aliases: ContactAlias[];
  contacts: ApiContact[];
  chats: ApiChat[];
  onClose: () => void;
  onAliasesChanged: () => void;
}

export function AliasEditor({ aliases, contacts, chats, onClose, onAliasesChanged }: AliasEditorProps) {
  const [newPhone, setNewPhone] = useState("");
  const [newName, setNewName] = useState("");
  const [saving, setSaving] = useState(false);
  const [editingPhone, setEditingPhone] = useState<string | null>(null);
  const [editingName, setEditingName] = useState("");
  const [error, setError] = useState<string | null>(null);

  // Build a set of all known phones for quick suggestion
  const knownPhones = new Map<string, string>();
  for (const c of contacts) {
    if (c.phone) knownPhones.set(c.phone, c.name);
  }
  for (const c of chats) {
    if (c.phone && !c.is_group) knownPhones.set(c.phone, c.name);
  }

  // Filter contacts/chats that don't have an alias yet (for suggestions)
  const aliasPhones = new Set(aliases.map((a) => a.phone));
  const suggestions = Array.from(knownPhones.entries())
    .filter(([phone]) => !aliasPhones.has(phone))
    .sort(([, a], [, b]) => a.localeCompare(b));

  const saveAlias = useCallback(
    async (phone: string, name: string) => {
      setSaving(true);
      setError(null);
      try {
        const res = await fetch(`/api/contact-aliases/${encodeURIComponent(phone)}`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name }),
        });
        if (!res.ok) {
          const data = await res.json().catch(() => ({ error: "Unknown error" }));
          setError(data.error ?? "Failed to save alias");
          return;
        }
        onAliasesChanged();
        setNewPhone("");
        setNewName("");
        setEditingPhone(null);
      } catch {
        setError("Network error");
      } finally {
        setSaving(false);
      }
    },
    [onAliasesChanged],
  );

  const deleteAlias = useCallback(
    async (phone: string) => {
      try {
        await fetch(`/api/contact-aliases/${encodeURIComponent(phone)}`, {
          method: "DELETE",
        });
        onAliasesChanged();
      } catch {
        // ignore
      }
    },
    [onAliasesChanged],
  );

  const handleAdd = (e: FormEvent) => {
    e.preventDefault();
    const phone = newPhone.replace(/[^0-9]/g, "");
    const name = newName.trim();
    if (!phone || !name) return;
    void saveAlias(phone, name);
  };

  const handleEditSave = (phone: string) => {
    const name = editingName.trim();
    if (!name) return;
    void saveAlias(phone, name);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="flex max-h-[80vh] w-full max-w-lg flex-col rounded-2xl bg-white shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-wa-border px-5 py-4">
          <h2 className="text-lg font-semibold text-wa-text">Contact Aliases</h2>
          <button
            className="rounded-full p-1 text-wa-icon transition-colors hover:bg-gray-200"
            onClick={onClose}
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Error */}
        {error && (
          <div className="bg-red-50 px-5 py-2 text-sm text-red-600">{error}</div>
        )}

        {/* Add new alias form */}
        <form onSubmit={handleAdd} className="flex items-end gap-2 border-b border-wa-border px-5 py-3">
          <div className="flex-1">
            <label className="mb-1 block text-[11px] font-medium uppercase tracking-wider text-wa-text-secondary">Phone</label>
            <input
              type="text"
              placeholder="e.g. 6285111240397"
              className="w-full rounded-lg border border-gray-300 bg-gray-50 px-3 py-2 text-sm text-wa-text outline-none focus:border-wa-teal-dark focus:ring-1 focus:ring-wa-teal-dark"
              value={newPhone}
              onChange={(e) => setNewPhone(e.target.value)}
            />
          </div>
          <div className="flex-1">
            <label className="mb-1 block text-[11px] font-medium uppercase tracking-wider text-wa-text-secondary">Display Name</label>
            <input
              type="text"
              placeholder="e.g. Cici"
              className="w-full rounded-lg border border-gray-300 bg-gray-50 px-3 py-2 text-sm text-wa-text outline-none focus:border-wa-teal-dark focus:ring-1 focus:ring-wa-teal-dark"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
            />
          </div>
          <button
            type="submit"
            disabled={saving || !newPhone.trim() || !newName.trim()}
            className="rounded-lg bg-wa-teal-dark px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-[#005c4b] disabled:opacity-50"
          >
            Add
          </button>
        </form>

        {/* Alias list */}
        <div className="flex-1 overflow-y-auto">
          {aliases.length === 0 && suggestions.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-8 text-sm text-wa-text-secondary">
              <p>No aliases yet.</p>
              <p className="mt-1 text-xs">Add an alias above to display friendly names for mentions.</p>
            </div>
          ) : null}

          {aliases.length > 0 && (
            <div className="px-5 py-3">
              <h3 className="mb-2 text-[11px] font-medium uppercase tracking-wider text-wa-text-secondary">Current Aliases</h3>
              <ul className="space-y-1">
                {aliases.map((alias) => (
                  <li key={alias.phone} className="flex items-center gap-2 rounded-lg px-3 py-2 hover:bg-gray-50">
                    {editingPhone === alias.phone ? (
                      <>
                        <span className="w-32 shrink-0 truncate text-xs text-wa-text-secondary">+{alias.phone}</span>
                        <input
                          type="text"
                          className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm outline-none focus:border-wa-teal-dark"
                          value={editingName}
                          onChange={(e) => setEditingName(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") handleEditSave(alias.phone);
                            if (e.key === "Escape") setEditingPhone(null);
                          }}
                          autoFocus
                        />
                        <button
                          className="text-xs text-wa-teal-dark hover:underline"
                          onClick={() => handleEditSave(alias.phone)}
                        >
                          Save
                        </button>
                        <button
                          className="text-xs text-wa-text-secondary hover:underline"
                          onClick={() => setEditingPhone(null)}
                        >
                          Cancel
                        </button>
                      </>
                    ) : (
                      <>
                        <span className="w-32 shrink-0 truncate text-xs text-wa-text-secondary">+{alias.phone}</span>
                        <span className="flex-1 truncate text-sm font-medium text-wa-text">{alias.name}</span>
                        <button
                          className="text-xs text-wa-teal-dark hover:underline"
                          onClick={() => {
                            setEditingPhone(alias.phone);
                            setEditingName(alias.name);
                          }}
                        >
                          Edit
                        </button>
                        <button
                          className="text-xs text-red-500 hover:underline"
                          onClick={() => void deleteAlias(alias.phone)}
                        >
                          Delete
                        </button>
                      </>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {suggestions.length > 0 && (
            <div className="border-t border-wa-border px-5 py-3">
              <h3 className="mb-2 text-[11px] font-medium uppercase tracking-wider text-wa-text-secondary">
                Known Contacts (click to add alias)
              </h3>
              <ul className="space-y-1">
                {suggestions.slice(0, 30).map(([phone, currentName]) => (
                  <li key={phone}>
                    <button
                      className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors hover:bg-gray-50"
                      onClick={() => {
                        setNewPhone(phone);
                        setNewName("");
                      }}
                    >
                      <span className="w-32 shrink-0 truncate text-xs text-wa-text-secondary">+{phone}</span>
                      <span className="flex-1 truncate text-sm text-wa-text">{currentName}</span>
                      <span className="text-xs text-wa-teal-dark">+ Alias</span>
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
