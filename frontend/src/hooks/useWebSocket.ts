import { useEffect, useCallback, useRef } from 'react';
import wsService from '@/services/websocket';
import { useChatStore } from '@/store/chatStore';
import type { WsEvent } from '@/types';

export function useWebSocket() {
  const { addIncomingMessage, updateMessageStatus, setSyncProgress, fetchChats, fetchContacts, updateContact } = useChatStore();
  const chatRefreshTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const contactsRefreshTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Debounced chat refresh — waits 500ms after the last trigger
  const debouncedFetchChats = useCallback(() => {
    if (chatRefreshTimer.current) {
      clearTimeout(chatRefreshTimer.current);
    }
    chatRefreshTimer.current = setTimeout(() => {
      fetchChats();
      chatRefreshTimer.current = null;
    }, 500);
  }, [fetchChats]);

  // Debounced contacts refresh — waits 1s after the last trigger
  const debouncedFetchContacts = useCallback(() => {
    if (contactsRefreshTimer.current) {
      clearTimeout(contactsRefreshTimer.current);
    }
    contactsRefreshTimer.current = setTimeout(() => {
      fetchContacts();
      contactsRefreshTimer.current = null;
    }, 1000);
  }, [fetchContacts]);

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (chatRefreshTimer.current) {
        clearTimeout(chatRefreshTimer.current);
      }
      if (contactsRefreshTimer.current) {
        clearTimeout(contactsRefreshTimer.current);
      }
    };
  }, []);

  const handleWsEvent = useCallback(
    (event: WsEvent) => {
      switch (event.type) {
        case 'new_message':
          if (event.data && typeof event.data === 'object') {
            const payload = event.data as Record<string, unknown>;
            const nestedMessage = payload.message;

            if (nestedMessage && typeof nestedMessage === 'object') {
              addIncomingMessage({
                ...(nestedMessage as Record<string, unknown>),
                chat_id: (payload.chat_id as string | undefined) || (nestedMessage as Record<string, unknown>).chat_id as string | undefined,
              } as never);
            } else {
              addIncomingMessage(payload as never);
            }
          }
          break;
        case 'message_status':
          updateMessageStatus(
            event.data.message_id as string,
            event.data.status as string
          );
          break;
        case 'sync_progress':
          setSyncProgress(event.data.status as string);
          // Refresh chats when sync is completed
          if (event.data.status === 'completed') {
            fetchChats();
          }
          break;
        case 'chats_updated':
          // History sync added new chats — debounced refresh
          debouncedFetchChats();
          debouncedFetchContacts();
          break;
        case 'contacts_updated':
          // Single contact push_name update — apply instantly + debounce full refresh
          if (event.data.jid) {
            updateContact(
              event.data.jid as string,
              (event.data.push_name as string) || null,
              (event.data.phone_number as string) || null,
            );
          }
          break;
        case 'session_connected':
        case 'session_disconnected':
          // Refresh sessions and chats
          fetchChats();
          fetchContacts();
          break;
        case 'qr_code':
          // Handled by QRCodeModal component via its own listener
          break;
        default:
          console.log('[WS] Event:', event.type, event.data);
      }
    },
    [addIncomingMessage, updateMessageStatus, setSyncProgress, fetchChats, fetchContacts, updateContact, debouncedFetchChats, debouncedFetchContacts]
  );

  useEffect(() => {
    const unsubscribe = wsService.on('*', handleWsEvent);
    return () => unsubscribe();
  }, [handleWsEvent]);

  return { isConnected: wsService.isConnected };
}
