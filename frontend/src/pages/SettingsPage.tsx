import { useState, useEffect, useRef } from "react";
// @ts-expect-error: CJS/ESM interop — named export exists at runtime
import { QRCode } from "react-qr-code";
import { Icon } from "../components/Icons";
import { useToast } from "../components/Toast";
import { useWhatsApp } from "../hooks/useWhatsApp";

type LocalConnState = "disconnected" | "scanning" | "connected";

export default function SettingsPage() {
  const { showToast } = useToast();
  const {
    connectionState: waState,
    qrCode: waQrCode,
    wsConnected,
  } = useWhatsApp();

  // Local UI state — always the single source of truth for the displayed
  // connection flow. Defaults to "disconnected" so the page is deterministic
  // regardless of whether a real backend is running.
  const [connState, setConnState] = useState<LocalConnState>("disconnected");
  const [dummyQr, setDummyQr] = useState("whatsapp://connect?dummy=" + Date.now());
  const [countdown, setCountdown] = useState(30);

  const qrValue = waQrCode ?? dummyQr;

  // Sync from hook: if the hook already knows the backend is connected
  // (e.g. from the REST fetch on mount or WS init event), update local state.
  const prevWaState = useRef(waState);
  const hasInitSynced = useRef(false);
  useEffect(() => {
    const changed = waState !== prevWaState.current;
    prevWaState.current = waState;

    // Initial sync — the hook learned the state from REST/WS before user interacted
    if (!hasInitSynced.current && waState === "connected") {
      hasInitSynced.current = true;
      setConnState("connected");
      return;
    }
    if (!hasInitSynced.current && waState === "scanning") {
      hasInitSynced.current = true;
      setConnState("scanning");
      return;
    }
    if (waState !== "disconnected") {
      hasInitSynced.current = true;
    }

    // Live transition: backend reports connected while user is scanning
    if (changed && wsConnected && waState === "connected" && connState === "scanning") {
      setConnState("connected");
      showToast("WhatsApp device connected successfully!");
    }
  }, [wsConnected, waState, connState, showToast]);

  /* QR refresh timer when scanning */
  useEffect(() => {
    if (connState !== "scanning") return;
    setCountdown(30);
    const iv = setInterval(() => {
      setCountdown((c) => {
        if (c <= 1) {
          setDummyQr("whatsapp://connect?dummy=" + Date.now());
          return 30;
        }
        return c - 1;
      });
    }, 1000);
    return () => clearInterval(iv);
  }, [connState]);

  const handleConnect = () => {
    setDummyQr("whatsapp://connect?dummy=" + Date.now());
    setConnState("scanning");
  };

  const handleSimulate = () => {
    setConnState("connected");
    showToast("WhatsApp device connected successfully!");
  };

  const handleDisconnect = () => {
    setConnState("disconnected");
    showToast("WhatsApp device disconnected");
  };

  return (
    <div className="settings-page" data-testid="settings-page">
      <div className="settings-header">
        <h2>Settings</h2>
        <p>Manage your WhatsApp connection and account preferences.</p>
      </div>

      <div className="settings-grid">
        {/* ── Device Connection Card ────── */}
        <div className="settings-card" data-testid="connection-card">
          <div className="settings-card-header">
            <div className="settings-card-icon">
              {connState === "connected" ? <Icon.Wifi /> : <Icon.WifiOff />}
            </div>
            <div>
              <h3>WhatsApp Connection</h3>
              <span className={"connection-status status-" + connState} data-testid="connection-status">
                {connState === "disconnected" && "Not connected"}
                {connState === "scanning" && "Waiting for scan…"}
                {connState === "connected" && "Connected"}
              </span>
            </div>
          </div>

          <div className="settings-card-body">
            {connState === "disconnected" && (
              <div className="qr-disconnected">
                <div className="qr-placeholder">
                  <Icon.QrCode />
                  <p>Scan the QR code with your WhatsApp mobile app to connect this device.</p>
                </div>
                <button className="settings-btn primary" onClick={handleConnect} data-testid="connect-btn">
                  <Icon.QrCode /> Show QR Code
                </button>
              </div>
            )}

            {connState === "scanning" && (
              <div className="qr-scanning" data-testid="qr-scanning">
                <div className="qr-code-wrapper">
                  <QRCode
                    value={qrValue}
                    size={200}
                    bgColor="#ffffff"
                    fgColor="#0d1117"
                    level="M"
                  />
                </div>
                <p className="qr-hint">
                  Open WhatsApp on your phone → Settings → Linked Devices → Link a Device
                </p>
                <div className="qr-timer">
                  <Icon.Refresh /> QR refreshes in <strong>{countdown}s</strong>
                </div>
                {wsConnected && (
                  <div className="qr-ws-status" style={{ color: "#10b981", fontSize: "0.85rem", marginTop: "0.5rem" }}>
                    ● Backend connected — waiting for phone scan
                  </div>
                )}
                <button className="settings-btn success" onClick={handleSimulate} data-testid="simulate-btn">
                  <Icon.CheckCircle /> Simulate Connection
                </button>
              </div>
            )}

            {connState === "connected" && (
              <div className="qr-connected" data-testid="qr-connected">
                <div className="connected-info">
                  <div className="connected-icon">
                    <Icon.CheckCircle />
                  </div>
                  <div>
                    <strong>WhatsApp Device</strong>
                    <p>Connected since {new Date().toLocaleDateString()}</p>
                  </div>
                </div>
                <button className="settings-btn danger" onClick={handleDisconnect} data-testid="disconnect-btn">
                  Disconnect Device
                </button>
              </div>
            )}
          </div>
        </div>

        {/* ── Account Card ──────────────── */}
        <div className="settings-card">
          <div className="settings-card-header">
            <div className="settings-card-icon">
              <Icon.User />
            </div>
            <div>
              <h3>Account</h3>
              <span className="settings-card-sub">Manage your profile and preferences</span>
            </div>
          </div>
          <div className="settings-card-body">
            <div className="settings-field">
              <label>Display Name</label>
              <input type="text" defaultValue="Frans D." placeholder="Your name" />
            </div>
            <div className="settings-field">
              <label>Email</label>
              <input type="email" defaultValue="frans@helpdesk.io" placeholder="Email address" />
            </div>
          </div>
        </div>

        {/* ── Notifications Card ────────── */}
        <div className="settings-card">
          <div className="settings-card-header">
            <div className="settings-card-icon">
              <Icon.Inbox />
            </div>
            <div>
              <h3>Notifications</h3>
              <span className="settings-card-sub">Configure alert preferences</span>
            </div>
          </div>
          <div className="settings-card-body">
            <label className="settings-toggle">
              <input type="checkbox" defaultChecked />
              <span className="toggle-slider" />
              Desktop notifications
            </label>
            <label className="settings-toggle">
              <input type="checkbox" defaultChecked />
              <span className="toggle-slider" />
              Sound alerts
            </label>
            <label className="settings-toggle">
              <input type="checkbox" />
              <span className="toggle-slider" />
              Email digest (daily)
            </label>
          </div>
        </div>
      </div>
    </div>
  );
}
