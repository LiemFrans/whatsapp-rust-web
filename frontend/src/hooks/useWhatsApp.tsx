import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  useCallback,
  type ReactNode,
} from "react";

// ── Types matching backend WsOutgoing ──────────────────────────────────────

export type ConnectionState = "disconnected" | "scanning" | "connected";

export interface WaMessage {
  id: string;
  from: string;
  fromName: string;
  chat: string;
  text: string;
  timestamp: number;
  isFromMe: boolean;
}

interface WsEvent {
  type: string;
  [key: string]: unknown;
}

// ── Context value ──────────────────────────────────────────────────────────

interface WhatsAppContextValue {
  /** WhatsApp connection state (disconnected → scanning → connected) */
  connectionState: ConnectionState;
  /** Current QR code string (only set while scanning) */
  qrCode: string | null;
  /** Whether the WebSocket to our backend is alive */
  wsConnected: boolean;
  /** All incoming real WA messages (newest last) */
  messages: WaMessage[];
  /** Send a text message via WhatsApp */
  sendMessage: (to: string, body: string) => void;
}

const WhatsAppContext = createContext<WhatsAppContextValue>({
  connectionState: "disconnected",
  qrCode: null,
  wsConnected: false,
  messages: [],
  sendMessage: () => {},
});

export function useWhatsApp() {
  return useContext(WhatsAppContext);
}

// ── Provider ───────────────────────────────────────────────────────────────

const RECONNECT_DELAY = 3000;

export function WhatsAppProvider({ children }: { children: ReactNode }) {
  const [connectionState, setConnectionState] =
    useState<ConnectionState>("disconnected");
  const [qrCode, setQrCode] = useState<string | null>(null);
  const [wsConnected, setWsConnected] = useState(false);
  const [messages, setMessages] = useState<WaMessage[]>([]);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  // Fetch initial connection state from REST endpoint on mount
  useEffect(() => {
    fetch("/api/auth/status")
      .then((r) => r.json())
      .then((data: { connected: boolean; state: string }) => {
        if (data.connected || data.state === "connected") {
          setConnectionState("connected");
        } else if (data.state === "scanning") {
          setConnectionState("scanning");
        }
      })
      .catch(() => {
        // Backend not reachable — stay disconnected
      });
  }, []);

  const connect = useCallback(() => {
    // Build ws:// or wss:// URL based on current page location
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${proto}//${window.location.host}/ws`;

    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => {
      setWsConnected(true);
    };

    ws.onclose = () => {
      setWsConnected(false);
      wsRef.current = null;
      // Auto-reconnect
      reconnectTimer.current = setTimeout(connect, RECONNECT_DELAY);
    };

    ws.onerror = () => {
      ws.close();
    };

    ws.onmessage = (ev) => {
      try {
        const data = JSON.parse(ev.data) as WsEvent;

        switch (data.type) {
          case "qr":
            setQrCode(data.code as string);
            setConnectionState("scanning");
            break;
          case "connected":
            setConnectionState("connected");
            setQrCode(null);
            break;
          case "disconnected":
            setConnectionState("disconnected");
            setQrCode(null);
            break;
          case "pair_success":
            setConnectionState("connected");
            setQrCode(null);
            break;
          case "logged_out":
            setConnectionState("disconnected");
            setQrCode(null);
            break;
          case "message":
            setMessages((prev) => [
              ...prev,
              {
                id: data.id as string,
                from: data.from as string,
                fromName: (data.from_name as string) || (data.from as string),
                chat: data.chat as string,
                text: data.text as string,
                timestamp: data.timestamp as number,
                isFromMe: data.is_from_me as boolean,
              },
            ]);
            break;
        }
      } catch {
        // ignore malformed messages
      }
    };
  }, []);

  useEffect(() => {
    connect();
    return () => {
      clearTimeout(reconnectTimer.current);
      wsRef.current?.close();
    };
  }, [connect]);

  const sendMessage = useCallback((to: string, body: string) => {
    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "send_msg", to, body }));
    }
  }, []);

  return (
    <WhatsAppContext.Provider
      value={{ connectionState, qrCode, wsConnected, messages, sendMessage }}
    >
      {children}
    </WhatsAppContext.Provider>
  );
}
