import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "WhatsApp Web",
  description: "WhatsApp Web Clone – Built with Next.js & Rust",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="antialiased bg-wa-bg">{children}</body>
    </html>
  );
}
