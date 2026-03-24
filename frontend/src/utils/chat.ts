import type { Chat } from '@/types';

/**
 * Get a clean display name for a chat.
 * Falls back gracefully from name → phone_number → cleaned JID.
 */
export function getChatDisplayName(chat: Chat): string {
  // Use the name if it's not a raw JID/LID identifier
  if (chat.name && !isRawIdentifier(chat.name)) {
    return chat.name;
  }

  // Use phone number if available
  if (chat.phone_number) {
    return chat.phone_number;
  }

  // Clean up the JID for display
  return cleanJidForDisplay(chat.chat_jid, chat.is_group);
}

/**
 * Clean a sender JID/name for display in group chat bubbles.
 * Prefers sender_name, falls back to cleaned sender JID, then phone-like format.
 */
export function cleanSenderDisplay(senderName?: string | null, sender?: string | null): string {
  // Use sender_name if it's a real name (not a raw identifier)
  if (senderName && !isRawIdentifier(senderName)) {
    return senderName;
  }

  // If sender is empty/missing, use sender_name if we have one, else "Participant"
  if (!sender || sender === '' || sender === 'me') {
    if (senderName) return cleanJidForDisplay(senderName, false);
    return 'Participant';
  }

  // Strip @server part
  const userPart = sender.split('@')[0];
  // Strip device suffix (e.g., "4372444528669:71" → "4372444528669")
  const cleanUser = userPart.split(':')[0];

  // If it looks like a phone number (10-15 digits), format with +
  if (/^\d{10,15}$/.test(cleanUser)) {
    return `+${cleanUser}`;
  }

  // For shorter numbers or other formats, just show as-is
  if (/^\d+$/.test(cleanUser)) {
    return `~${cleanUser}`;
  }

  return cleanUser || 'Participant';
}

/**
 * Format @mentions in message text.
 * Replaces @<LID_number> patterns with styled @+<number> or @User format.
 */
export function formatMentions(text: string): string {
  // Match @<digits> patterns (LID mentions like @149615370338545)
  return text.replace(/@(\d{10,20})/g, (_match, digits: string) => {
    // If it's 10-15 digits, likely a phone number
    if (digits.length >= 10 && digits.length <= 15) {
      return `@+${digits}`;
    }
    // Longer numbers are LID identifiers — show shortened
    return `@User`;
  });
}

/**
 * Check if a string looks like a raw JID/LID identifier
 */
export function isRawIdentifier(name: string): boolean {
  if (name.includes('@lid') || name.includes('@g.us') || name.includes('@s.whatsapp.net')) {
    return true;
  }
  // Pure long numeric strings are likely LID numbers
  if (/^\d{10,}$/.test(name)) {
    return true;
  }
  return false;
}

/**
 * Extract a human-friendly label from a raw JID
 */
function cleanJidForDisplay(jid: string, isGroup: boolean): string {
  // Strip the @server part
  const userPart = jid.split('@')[0];

  if (isGroup) {
    // Group JIDs look like "6281380888035-1572323526"
    return `Group ${userPart.slice(-6)}`;
  }

  // For personal chats, the user part might be a phone number or LID
  // If it looks like a phone number (starts with country code), format it
  if (/^\d{10,15}$/.test(userPart)) {
    return `+${userPart}`;
  }

  return userPart;
}
