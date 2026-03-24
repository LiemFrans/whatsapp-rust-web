import { describe, it, expect } from 'vitest';
import {
  getChatDisplayName,
  cleanSenderDisplay,
  formatMentions,
  isRawIdentifier,
} from '@/utils/chat';
import type { Chat } from '@/types';

// Helper to build a minimal Chat object
function makeChat(overrides: Partial<Chat> = {}): Chat {
  return {
    id: 'c1',
    session_id: 's1',
    chat_jid: '6281380888035@s.whatsapp.net',
    is_group: false,
    name: null,
    phone_number: null,
    profile_pic_url: null,
    last_message: null,
    last_message_at: null,
    unread_count: 0,
    is_pinned: false,
    is_muted: false,
    is_archived: false,
    assigned_agent_id: null,
    assigned_agent_name: null,
    created_at: '2024-01-01T00:00:00Z',
    ...overrides,
  };
}

// ─── getChatDisplayName ─────────────────────────────────────────

describe('getChatDisplayName', () => {
  it('shows "Name (+phone)" when both name and phone are available', () => {
    const chat = makeChat({ name: 'John Doe', phone_number: '6281380888035' });
    expect(getChatDisplayName(chat)).toBe('John Doe (+6281380888035)');
  });

  it('shows name with phone already prefixed with +', () => {
    const chat = makeChat({ name: 'Alice', phone_number: '+6281380888035' });
    expect(getChatDisplayName(chat)).toBe('Alice (+6281380888035)');
  });

  it('shows name only when no phone number', () => {
    const chat = makeChat({ name: 'Bob Smith' });
    expect(getChatDisplayName(chat)).toBe('Bob Smith');
  });

  it('shows phone number when no name', () => {
    const chat = makeChat({ phone_number: '6281380888035' });
    expect(getChatDisplayName(chat)).toBe('+6281380888035');
  });

  it('falls back to cleaned JID when no name or phone', () => {
    const chat = makeChat({ chat_jid: '6281380888035@s.whatsapp.net' });
    expect(getChatDisplayName(chat)).toBe('+6281380888035');
  });

  it('skips raw JID names and uses phone number instead', () => {
    const chat = makeChat({
      name: '6281380888035@s.whatsapp.net',
      phone_number: '6281380888035',
    });
    expect(getChatDisplayName(chat)).toBe('+6281380888035');
  });

  it('skips LID names and uses JID fallback', () => {
    const chat = makeChat({
      name: '149615370338545@lid',
      chat_jid: '6281380888035@s.whatsapp.net',
    });
    expect(getChatDisplayName(chat)).toBe('+6281380888035');
  });

  it('shows group name for groups', () => {
    const chat = makeChat({
      is_group: true,
      name: 'Family Group',
      chat_jid: '6281380888035-1572323526@g.us',
    });
    expect(getChatDisplayName(chat)).toBe('Family Group');
  });

  it('shows group JID suffix for unnamed groups', () => {
    const chat = makeChat({
      is_group: true,
      chat_jid: '6281380888035-1572323526@g.us',
    });
    expect(getChatDisplayName(chat)).toContain('Group');
  });
});

// ─── cleanSenderDisplay ─────────────────────────────────────────

describe('cleanSenderDisplay', () => {
  it('shows "~ Name (+phone)" when sender name and JID phone available', () => {
    const result = cleanSenderDisplay('Alice', '6281380888035@s.whatsapp.net');
    expect(result).toBe('~ Alice (+6281380888035)');
  });

  it('shows "~ Name" when sender name but no phone in JID (short LID)', () => {
    // LID with >15 digits won't extract as phone number
    const result = cleanSenderDisplay('Bob', '1496153703385451234@lid');
    expect(result).toBe('~ Bob');
  });

  it('shows ~ +phone from JID when no sender name', () => {
    const result = cleanSenderDisplay(null, '6281380888035@s.whatsapp.net');
    expect(result).toBe('~ +6281380888035');
  });

  it('strips device suffix from sender JID', () => {
    const result = cleanSenderDisplay(null, '6281380888035:71@s.whatsapp.net');
    expect(result).toBe('~ +6281380888035');
  });

  it('returns "~ Unknown" when no info available', () => {
    const result = cleanSenderDisplay(null, null);
    expect(result).toBe('~ Unknown');
  });

  it('skips raw identifier sender names', () => {
    const result = cleanSenderDisplay('6281380888035@s.whatsapp.net', '6281380888035@s.whatsapp.net');
    expect(result).toBe('~ +6281380888035');
  });
});

// ─── formatMentions ─────────────────────────────────────────────

describe('formatMentions', () => {
  it('formats phone-length mentions with +', () => {
    expect(formatMentions('Hello @6281380888035')).toBe('Hello @+6281380888035');
  });

  it('formats long LID mentions as @User', () => {
    expect(formatMentions('Hello @149615370338545123')).toBe('Hello @User');
  });

  it('leaves non-mention text unchanged', () => {
    expect(formatMentions('Hello world!')).toBe('Hello world!');
  });
});

// ─── isRawIdentifier ────────────────────────────────────────────

describe('isRawIdentifier', () => {
  it('detects @s.whatsapp.net JID', () => {
    expect(isRawIdentifier('6281380888035@s.whatsapp.net')).toBe(true);
  });

  it('detects @lid JID', () => {
    expect(isRawIdentifier('149615370338545@lid')).toBe(true);
  });

  it('detects @g.us group JID', () => {
    expect(isRawIdentifier('6281380888035-1234@g.us')).toBe(true);
  });

  it('detects pure long numeric strings', () => {
    expect(isRawIdentifier('149615370338545')).toBe(true);
  });

  it('does not flag real names', () => {
    expect(isRawIdentifier('Alice Smith')).toBe(false);
    expect(isRawIdentifier('John')).toBe(false);
  });
});
