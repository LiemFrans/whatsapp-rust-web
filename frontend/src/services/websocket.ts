import type { WsEvent } from '@/types';

type WsEventHandler = (event: WsEvent) => void;

const WS_URL = import.meta.env.VITE_WS_URL || `ws://${window.location.host}`;

class WebSocketService {
  private ws: WebSocket | null = null;
  private handlers: Map<string, Set<WsEventHandler>> = new Map();
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private token: string | null = null;
  private _isConnected = false;

  get isConnected(): boolean {
    return this._isConnected;
  }

  connect(token: string): void {
    this.token = token;
    this.reconnectAttempts = 0;
    this.doConnect();
  }

  private doConnect(): void {
    if (!this.token) return;

    try {
      this.ws = new WebSocket(`${WS_URL}/ws?token=${this.token}`);

      this.ws.onopen = () => {
        this._isConnected = true;
        this.reconnectAttempts = 0;
        console.log('[WS] Connected');
        this.emit({ type: 'ws_connected', data: {} });
      };

      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data) as WsEvent;
          this.emit(data);
        } catch (e) {
          console.error('[WS] Failed to parse message:', e);
        }
      };

      this.ws.onclose = (event) => {
        this._isConnected = false;
        console.log('[WS] Disconnected:', event.code, event.reason);
        this.emit({ type: 'ws_disconnected', data: {} });
        this.scheduleReconnect();
      };

      this.ws.onerror = (error) => {
        console.error('[WS] Error:', error);
      };
    } catch (e) {
      console.error('[WS] Connection error:', e);
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.warn('[WS] Max reconnect attempts reached');
      return;
    }

    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
    this.reconnectAttempts++;
    console.log(`[WS] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);

    this.reconnectTimer = setTimeout(() => {
      this.doConnect();
    }, delay);
  }

  disconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.token = null;
    this.reconnectAttempts = this.maxReconnectAttempts; // Prevent reconnect
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this._isConnected = false;
  }

  on(type: string, handler: WsEventHandler): () => void {
    if (!this.handlers.has(type)) {
      this.handlers.set(type, new Set());
    }
    this.handlers.get(type)!.add(handler);

    // Return unsubscribe function
    return () => {
      this.handlers.get(type)?.delete(handler);
    };
  }

  off(type: string, handler: WsEventHandler): void {
    this.handlers.get(type)?.delete(handler);
  }

  private emit(event: WsEvent): void {
    // Notify specific type handlers
    const typeHandlers = this.handlers.get(event.type);
    if (typeHandlers) {
      typeHandlers.forEach((handler) => handler(event));
    }

    // Notify wildcard handlers
    const wildcardHandlers = this.handlers.get('*');
    if (wildcardHandlers) {
      wildcardHandlers.forEach((handler) => handler(event));
    }
  }

  send(data: Record<string, unknown>): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(data));
    }
  }
}

// Singleton instance
export const wsService = new WebSocketService();
export default wsService;
