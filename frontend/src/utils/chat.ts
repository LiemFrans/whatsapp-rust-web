import type { Chat } from '@/types';
import type { ContactsMap } from '@/store/chatStore';

/**
 * Get a clean display name for a chat.
 * Shows "Name (+phone)" when both are available, like WhatsApp Web.
 * Falls back gracefully from name → contacts → phone_number → cleaned JID.
 */
export function getChatDisplayName(chat: Chat, contacts?: ContactsMap): string {
  const hasName = chat.name && !isRawIdentifier(chat.name);
  const rawPhone = chat.phone_number?.trim().replace(/^\+/, '') || '';
  const hasPhone = rawPhone !== '' && isLikelyRealPhone(rawPhone, chat.chat_jid);

  // Show "Name" only — phone is shown separately in the UI header
  if (hasName && hasPhone) {
    return chat.name!;
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

  // If it's a LID JID, try contacts for a phone number, don't show the raw LID number
  if (sender.includes('@lid')) {
    const phone = findPhoneForSender(sender, contacts);
    if (phone) {
      return `~ ${formatPhoneDisplay(phone.replace(/^\+/, ''))}`;
    }
    return '~ Unknown';
  }

  // If it looks like a real phone number, format with +
  if (/^\d{10,15}$/.test(cleanUser) && isLikelyRealPhone(cleanUser, sender)) {
    return `~ +${cleanUser}`;
  }

  // For shorter numbers or other formats, just show as-is with prefix
  if (/^\d+$/.test(cleanUser) && cleanUser.length < 10) {
    return `~ ${cleanUser}`;
  }

  // Don't display long numeric strings that are likely internal IDs
  if (/^\d+$/.test(cleanUser)) {
    return '~ Unknown';
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

    // Fallback: if it looks like a phone number (10-13 digits), show formatted
    if (digits.length >= 10 && digits.length <= 13) {
      return `@${formatPhoneDisplay(digits)}`;
    }
    // For longer numbers (LID identifiers), try contacts phone_number lookup
    if (contacts) {
      // Check if any contact with this LID has a phone_number
      const lidJid2 = `${digits}@lid`;
      const fromLid2 = contacts[lidJid2] || contacts[digits];
      if (fromLid2?.phone_number) {
        const raw = fromLid2.phone_number.replace(/^\+/, '');
        if (/^\d{7,15}$/.test(raw)) {
          return `@${formatPhoneDisplay(raw)}`;
        }
      }
    }
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
  // Dash-separated numeric strings are group JID user parts (e.g., "6285732931330-1606750208")
  if (/^\d+-\d+$/.test(name)) {
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
      const raw = contact.phone_number.replace(/^\+/, '');
      // Trust phone_number from contacts DB — it's always a real phone
      if (/^\d{7,15}$/.test(raw)) {
        const p = contact.phone_number;
        return p.startsWith('+') ? p : `+${p}`;
      }
    }
  }

  // Extract from JID if it's a phone-based JID (not LID)
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
 * Format a phone number for display with proper spacing, like WhatsApp Web.
 * E.g. "6285722786222" → "+62 857-2278-6222"
 */
export function formatPhoneDisplay(rawPhone: string): string {
  const digits = rawPhone.replace(/[^\d]/g, '');
  if (digits.length < 7) return `+${digits}`;

  // Detect country code length
  let ccLen = 2; // default for most countries
  const first = digits[0];
  if (first === '1' || first === '7') ccLen = 1; // US/Canada, Russia/Kazakhstan

  const cc = digits.slice(0, ccLen);
  const national = digits.slice(ccLen);

  // Split national number into groups of 4 from the right
  const groups: string[] = [];
  let i = national.length;
  while (i > 0) {
    const start = Math.max(0, i - 4);
    groups.unshift(national.slice(start, i));
    i = start;
  }

  return `+${cc} ${groups.join('-')}`;
}

/**
 * Get structured sender information for display in group chat bubbles.
 * Returns separate name and phone so they can be styled independently.
 * Matches WhatsApp Web format: "~ Name    +62 857-2278-6222"
 */
export function getSenderDisplayInfo(
  senderName?: string | null,
  sender?: string | null,
  contacts?: ContactsMap,
  senderPhoneNumber?: string | null,
): { displayName: string; formattedPhone: string | null } {
  let name: string | null = null;
  let phone: string | null = null;

  // 1. Use sender_name if it's a real name
  if (senderName && !isRawIdentifier(senderName)) {
    name = senderName;
  }

  // 2. Use sender_phone_number from backend if available
  if (senderPhoneNumber) {
    const raw = senderPhoneNumber.replace(/^\+/, '');
    if (/^\d{7,15}$/.test(raw)) {
      phone = formatPhoneDisplay(raw);
    }
  }

  // 3. Try contacts lookup for both name and phone
  if (sender && contacts) {
    const userPart = sender.split('@')[0];
    const cleanUser = userPart.split(':')[0];
    const contact = contacts[sender] || contacts[userPart] || contacts[cleanUser];

    if (contact) {
      if (!name && contact.push_name) {
        name = contact.push_name;
      }
      if (!phone && contact.phone_number) {
        const raw = contact.phone_number.replace(/^\+/, '');
        // Trust phone_number from contacts DB — it's always a real phone,
        // regardless of whether the JID is @lid or @s.whatsapp.net
        if (/^\d{7,15}$/.test(raw)) {
          phone = formatPhoneDisplay(raw);
        }
      }
    }
  }

  // 3. Try to extract phone from sender JID (only works for @s.whatsapp.net)
  if (!phone && sender) {
    const extracted = extractPhoneFromJid(sender);
    if (extracted) {
      phone = formatPhoneDisplay(extracted);
    }
  }

  // 4. Determine display name fallbacks
  if (!name) {
    if (sender === 'me') {
      name = 'You';
    } else if (phone) {
      // No name but have phone — phone becomes the display name (like WhatsApp Web)
      name = phone;
      phone = null;
    } else if (sender) {
      // Last resort: extract whatever phone-like number we can from the JID
      const userPart = sender.split('@')[0].split(':')[0];
      if (/^\d{10,15}$/.test(userPart)) {
        name = formatPhoneDisplay(userPart);
      } else {
        name = 'Unknown';
      }
    } else {
      name = 'Unknown';
    }
  }

  return { displayName: name, formattedPhone: phone };
}

/**
 * Check if a number string is likely a real phone number vs a WhatsApp LID.
 * Real phone numbers are typically 10-15 digits.
 * LID numbers are internal WhatsApp identifiers that can overlap in length.
 * We use context (JID domain) when available, otherwise reject very long numbers.
 */
export function isLikelyRealPhone(number: string, jid?: string | null): boolean {
  // If we have the JID context, only phone-based JIDs have real phone numbers
  if (jid) {
    // @lid JIDs are never real phone numbers
    if (jid.includes('@lid')) return false;
    // @s.whatsapp.net are phone-based
    if (jid.includes('@s.whatsapp.net')) return true;
    // @g.us are group JIDs, not phone numbers
    if (jid.includes('@g.us')) return false;
  }
  // Without JID context, use a stricter phone number check (max 13 digits)
  // Most real international phone numbers are 10-13 digits
  return /^\d{10,13}$/.test(number);
}

/**
 * Extract phone number from a JID (e.g., "6281380888035@s.whatsapp.net" → "6281380888035")
 * Only extracts from phone-based JIDs (@s.whatsapp.net), NOT from LID JIDs (@lid).
 */
function extractPhoneFromJid(jid?: string | null): string | null {
  if (!jid) return null;
  // Only extract from phone-based JIDs
  if (jid.includes('@lid') || jid.includes('@g.us')) return null;
  const userPart = jid.split('@')[0];
  const cleanUser = userPart.split(':')[0];
  if (/^\d{10,15}$/.test(cleanUser) && isLikelyRealPhone(cleanUser, jid)) {
    return cleanUser;
  }
  return null;
}
