import { useEffect, useCallback, useRef } from 'react';
import wsService from '@/services/websocket';
import { useChatStore } from '@/store/chatStore';
import type { WsEvent } from '@/types';

export function useWebSocket() {
  const { addIncomingMessage, updateMessageStatus, setSyncProgress, fetchChats } = useChatStore();
  const chatRefreshTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

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

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (chatRefreshTimer.current) {
        clearTimeout(chatRefreshTimer.current);
      }
    };
  }, []);

  const handleWsEvent = useCallback(
    (event: WsEvent) => {
      switch (event.type) {
        case 'new_message':
          addIncomingMessage(event.data as never);
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
          break;
        case 'session_connected':
        case 'session_disconnected':
          // Refresh sessions and chats
          fetchChats();
          break;
        case 'qr_code':
          // Handled by QRCodeModal component via its own listener
          break;
        default:
          console.log('[WS] Event:', event.type, event.data);
      }
    },
    [addIncomingMessage, updateMessageStatus, setSyncProgress, fetchChats, debouncedFetchChats]
  );

  useEffect(() => {
    const unsubscribe = wsService.on('*', handleWsEvent);
    return () => unsubscribe();
  }, [handleWsEvent]);

  return { isConnected: wsService.isConnected };
}
