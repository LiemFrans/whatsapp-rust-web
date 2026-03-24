import type { Chat } from '@/types';
import type { ContactsMap } from '@/store/chatStore';

/**
 * Get a clean display name for a chat.
 * Shows "Name (+phone)" when both are available, like WhatsApp Web.
 * Falls back gracefully from name → contacts → phone_number → cleaned JID.
 */
export function getChatDisplayName(chat: Chat, contacts?: ContactsMap): string {
  const hasName = chat.name && !isRawIdentifier(chat.name);
  const hasPhone = chat.phone_number && chat.phone_number.trim() !== '';

  // Show "Name (+phone)" when both are available
  if (hasName && hasPhone) {
    const phone = chat.phone_number!.startsWith('+') ? chat.phone_number! : `+${chat.phone_number!}`;
    return `${chat.name} (${phone})`;
  }

  // Use the name if it's not a raw JID/LID identifier
  if (hasName) {
    return chat.name!;
  }

  // Try contacts lookup by chat_jid
  if (contacts) {
    const resolved = resolveFromContacts(chat.chat_jid, contacts);
    if (resolved) return resolved;
  }

  // Use phone number if available
  if (hasPhone) {
    return chat.phone_number!.startsWith('+') ? chat.phone_number! : `+${chat.phone_number!}`;
  }

  // Clean up the JID for display
  return cleanJidForDisplay(chat.chat_jid, chat.is_group);
}

/**
 * Clean a sender JID/name for display in group chat bubbles.
 * Shows "~ Name (+phone)" like WhatsApp Web when a push name is available.
 * Uses contacts map for fallback resolution.
 */
export function cleanSenderDisplay(
  senderName?: string | null,
  sender?: string | null,
  contacts?: ContactsMap,
): string {
  // Use sender_name if it's a real name (not a raw identifier)
  if (senderName && !isRawIdentifier(senderName)) {
    // Try to find phone from contacts or sender JID
    const phone = findPhoneForSender(sender, contacts);
    if (phone) {
      return `~ ${senderName} (${phone})`;
    }
    return `~ ${senderName}`;
  }

  // Try contacts lookup by sender JID
  if (sender && contacts) {
    const resolved = resolveFromContacts(sender, contacts);
    if (resolved) return `~ ${resolved}`;
  }

  // Handle 'me' sender (outgoing messages in history sync)
  if (sender === 'me') {
    return '~ You';
  }

  // If sender is empty/missing, use sender_name if we have one, else try JID display
  if (!sender || sender === '') {
    if (senderName) return `~ ${cleanJidForDisplay(senderName, false)}`;
    return '~ Unknown';
  }

  // Strip @server part
  const userPart = sender.split('@')[0];
  // Strip device suffix (e.g., "4372444528669:71" → "4372444528669")
  const cleanUser = userPart.split(':')[0];

  // If it looks like a phone number (10-15 digits), format with +
  if (/^\d{10,15}$/.test(cleanUser)) {
    return `~ +${cleanUser}`;
  }

  // For shorter numbers or other formats, just show as-is with prefix
  if (/^\d+$/.test(cleanUser)) {
    return `~ ${cleanUser}`;
  }

  return cleanUser ? `~ ${cleanUser}` : '~ Unknown';
}

/**
 * Format @mentions in message text.
 * Replaces @<number> patterns with resolved contact names from the contacts map.
 * Falls back to @+<number> or @User for unresolved mentions.
 */
export function formatMentions(text: string, contacts?: ContactsMap): string {
  // Match @<digits> patterns — phone numbers, LID mentions
  // Also match @+<digits> patterns that the backend may already prefix
  return text.replace(/@\+?(\d{6,20})/g, (_match, digits: string) => {
    // Try contacts lookup
    if (contacts) {
      const contact = contacts[digits];
      if (contact?.push_name) {
        return `@${contact.push_name}`;
      }
      // Also try with common JID suffixes
      const sJid = `${digits}@s.whatsapp.net`;
      const lidJid = `${digits}@lid`;
      const fromS = contacts[sJid];
      const fromLid = contacts[lidJid];
      if (fromS?.push_name) return `@${fromS.push_name}`;
      if (fromLid?.push_name) return `@${fromLid.push_name}`;
    }

    // Fallback: if it looks like a phone number, show @+number
    if (digits.length >= 10 && digits.length <= 15) {
      return `@+${digits}`;
    }
    // Longer numbers are LID identifiers — show @User
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
 * Resolve a JID to a display string using the contacts map.
 * Returns "PushName (+phone)" or "PushName" or null if not found.
 */
function resolveFromContacts(jid: string, contacts: ContactsMap): string | null {
  // Try exact match
  let contact = contacts[jid];
  if (!contact) {
    // Try user part
    const userPart = jid.split('@')[0];
    contact = contacts[userPart];
    if (!contact) {
      // Try without device suffix
      const cleanUser = userPart.split(':')[0];
      contact = contacts[cleanUser];
    }
  }
  if (!contact?.push_name) return null;

  if (contact.phone_number) {
    const phone = contact.phone_number.startsWith('+') ? contact.phone_number : `+${contact.phone_number}`;
    return `${contact.push_name} (${phone})`;
  }
  return contact.push_name;
}

/**
 * Find a phone number for a sender from contacts or JID extraction
 */
function findPhoneForSender(sender?: string | null, contacts?: ContactsMap): string | null {
  if (!sender) return null;

  // Check contacts for phone_number
  if (contacts) {
    const contact = contacts[sender] ||
      contacts[sender.split('@')[0]] ||
      contacts[sender.split('@')[0].split(':')[0]];
    if (contact?.phone_number) {
      const p = contact.phone_number;
      return p.startsWith('+') ? p : `+${p}`;
    }
  }

  // Extract from JID if it's a phone-based JID
  const phone = extractPhoneFromJid(sender);
  if (phone) return `+${phone}`;

  return null;
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

/**
 * Extract phone number from a JID (e.g., "6281380888035@s.whatsapp.net" → "6281380888035")
 */
function extractPhoneFromJid(jid?: string | null): string | null {
  if (!jid) return null;
  const userPart = jid.split('@')[0];
  const cleanUser = userPart.split(':')[0];
  if (/^\d{10,15}$/.test(cleanUser)) {
    return cleanUser;
  }
  return null;
}
