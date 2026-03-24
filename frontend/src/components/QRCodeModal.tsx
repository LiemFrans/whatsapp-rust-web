import { useEffect, useRef, useState } from 'react';
import { QRCodeSVG } from 'qrcode.react';
import { X, Loader2, Smartphone, CheckCircle2 } from 'lucide-react';

interface QRCodeModalProps {
  isOpen: boolean;
  onClose: () => void;
  qrCode: string | null;
  sessionName: string;
  status: 'connecting' | 'qr_code' | 'connected' | 'error';
}

export default function QRCodeModal({ isOpen, onClose, qrCode, sessionName, status }: QRCodeModalProps) {
  const [countdown, setCountdown] = useState(60);
  const timerRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    if (status === 'qr_code') {
      setCountdown(60);
      timerRef.current = window.setInterval(() => {
        setCountdown((prev: number) => {
          if (prev <= 1) {
            window.clearInterval(timerRef.current);
            return 0;
          }
          return prev - 1;
        });
      }, 1000);
    }
    return () => {
      if (timerRef.current) window.clearInterval(timerRef.current);
    };
  }, [status, qrCode]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="relative w-full max-w-md rounded-2xl bg-white p-8 shadow-2xl dark:bg-gray-800">
        <button
          onClick={onClose}
          className="absolute right-4 top-4 rounded-full p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-600 dark:hover:bg-gray-700"
        >
          <X size={20} />
        </button>

        <h2 className="mb-2 text-center text-xl font-semibold text-gray-800 dark:text-white">
          Connect WhatsApp
        </h2>
        <p className="mb-6 text-center text-sm text-gray-500 dark:text-gray-400">
          Session: {sessionName}
        </p>

        <div className="flex flex-col items-center">
          {status === 'connecting' && (
            <div className="flex flex-col items-center gap-4 py-12">
              <Loader2 size={48} className="animate-spin text-wa-green" />
              <p className="text-gray-600 dark:text-gray-300">Connecting to WhatsApp...</p>
            </div>
          )}

          {status === 'qr_code' && qrCode && (
            <>
              <div className="rounded-xl border-4 border-wa-green/20 bg-white p-4">
                <QRCodeSVG
                  value={qrCode}
                  size={256}
                  level="M"
                  includeMargin={false}
                  bgColor="#ffffff"
                  fgColor="#111b21"
                />
              </div>
              <div className="mt-4 flex items-center gap-2 text-sm text-gray-500">
                <Smartphone size={16} />
                <span>Scan with WhatsApp on your phone</span>
              </div>
              <div className="mt-2 text-xs text-gray-400">
                QR expires in {countdown}s
              </div>
              <ol className="mt-6 space-y-2 text-left text-sm text-gray-600 dark:text-gray-300">
                <li className="flex gap-2">
                  <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-wa-green/10 text-xs font-bold text-wa-green">
                    1
                  </span>
                  Open WhatsApp on your phone
                </li>
                <li className="flex gap-2">
                  <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-wa-green/10 text-xs font-bold text-wa-green">
                    2
                  </span>
                  Tap Menu or Settings → Linked Devices
                </li>
                <li className="flex gap-2">
                  <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-wa-green/10 text-xs font-bold text-wa-green">
                    3
                  </span>
                  Tap Link a Device and scan this QR code
                </li>
              </ol>
            </>
          )}

          {status === 'connected' && (
            <div className="flex flex-col items-center gap-4 py-12">
              <CheckCircle2 size={64} className="text-wa-green" />
              <p className="text-lg font-medium text-gray-800 dark:text-white">Connected!</p>
              <p className="text-sm text-gray-500">WhatsApp session is now active</p>
              <button
                onClick={onClose}
                className="mt-4 rounded-lg bg-wa-green px-6 py-2 text-white hover:bg-wa-green/90"
              >
                Continue
              </button>
            </div>
          )}

          {status === 'error' && (
            <div className="flex flex-col items-center gap-4 py-12">
              <X size={64} className="text-red-500" />
              <p className="text-lg font-medium text-gray-800 dark:text-white">Connection Failed</p>
              <p className="text-sm text-gray-500">Please try again</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
