import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import ChatBubble from '@/components/ChatBubble';
import type { Message } from '@/types';

// Mock the api module
vi.mock('@/services/api', () => ({
  chatApi: {
    mediaUrl: (chatId: string, messageId: string) =>
      `/api/chats/${chatId}/messages/${messageId}/media`,
  },
}));

function makeMessage(overrides: Partial<Message> = {}): Message {
  return {
    id: 'msg-1',
    message_id: 'wa-msg-1',
    sender: '6281380888035@s.whatsapp.net',
    sender_name: null,
    content: 'Hello world',
    message_type: 'text' as const,
    media_url: null,
    media_mime_type: null,
    media_filename: null,
    thumbnail_base64: null,
    status: 'delivered' as const,
    is_from_me: false,
    is_forwarded: false,
    is_starred: false,
    reply_to_message_id: null,
    reply_to: null,
    quote_content: null,
    quote_sender: null,
    quote_sender_name: null,
    is_deleted: false,
    timestamp: '2024-01-15T10:30:00Z',
    edited_at: null,
    ...overrides,
  };
}

describe('ChatBubble', () => {
  const chatId = 'chat-1';

  it('renders text message content', () => {
    const msg = makeMessage({ content: 'Hello world' });
    render(<ChatBubble message={msg} chatId={chatId} />);
    expect(screen.getByText('Hello world')).toBeInTheDocument();
  });

  it('renders deleted message placeholder', () => {
    const msg = makeMessage({ is_deleted: true });
    render(<ChatBubble message={msg} chatId={chatId} />);
    expect(screen.getByText('🚫 This message was deleted')).toBeInTheDocument();
  });

  it('shows sender name in groups', () => {
    const msg = makeMessage({ sender_name: 'Alice' });
    render(<ChatBubble message={msg} chatId={chatId} showSender />);
    expect(screen.getByText(/Alice/)).toBeInTheDocument();
  });

  it('displays quoted message content for replies', () => {
    const msg = makeMessage({
      content: 'My reply text',
      reply_to_message_id: 'original-msg-id',
      quote_content: 'This is the original message',
      quote_sender_name: 'Bob',
    });
    const { container } = render(<ChatBubble message={msg} chatId={chatId} />);
    // Find the quote block by its border-l-4 class
    const quoteBlock = container.querySelector('.border-wa-green');
    expect(quoteBlock).toBeTruthy();
    expect(quoteBlock!.textContent).toContain('Bob');
    expect(quoteBlock!.textContent).toContain('This is the original message');
  });

  it('shows "Replied message" fallback when no quote content', () => {
    const msg = makeMessage({
      content: 'My reply text',
      reply_to_message_id: 'original-msg-id',
      quote_content: null,
      reply_to: 'original-msg-id',
    });
    const { container } = render(<ChatBubble message={msg} chatId={chatId} />);
    const quoteBlock = container.querySelector('.border-wa-green');
    expect(quoteBlock).toBeTruthy();
    expect(quoteBlock!.textContent).toContain('Replied message');
  });

  it('shows forwarded indicator for forwarded messages', () => {
    const msg = makeMessage({ is_forwarded: true, content: 'Fwd content' });
    const { container } = render(<ChatBubble message={msg} chatId={chatId} />);
    // The forwarded label contains an SVG + text, search by text content
    const forwarded = container.querySelector('p.italic');
    expect(forwarded).toBeTruthy();
    expect(forwarded!.textContent).toContain('Forwarded');
  });

  it('does not show forwarded indicator for non-forwarded messages', () => {
    const msg = makeMessage({ is_forwarded: false });
    const { container } = render(<ChatBubble message={msg} chatId={chatId} />);
    const forwarded = Array.from(container.querySelectorAll('p.italic')).find(
      (el) => el.textContent?.includes('Forwarded')
    );
    expect(forwarded).toBeUndefined();
  });

  it('calls onReply when reply button clicked', () => {
    const onReply = vi.fn();
    const msg = makeMessage();
    render(<ChatBubble message={msg} chatId={chatId} onReply={onReply} />);

    const replyBtn = screen.getByTitle('Reply');
    fireEvent.click(replyBtn);
    expect(onReply).toHaveBeenCalledWith(msg);
  });

  it('renders image with media proxy URL', () => {
    const msg = makeMessage({
      message_type: 'image',
      content: 'A photo caption',
    });
    const { container } = render(<ChatBubble message={msg} chatId={chatId} />);
    const img = container.querySelector('img[alt="Image"]');
    expect(img).toBeTruthy();
    expect(img!.getAttribute('src')).toBe('/api/chats/chat-1/messages/msg-1/media');
    expect(screen.getByText('A photo caption')).toBeInTheDocument();
  });

  it('renders document with download link', () => {
    const msg = makeMessage({
      message_type: 'document',
      media_filename: 'report.pdf',
      media_mime_type: 'application/pdf',
      content: 'report.pdf',
    });
    const { container } = render(<ChatBubble message={msg} chatId={chatId} />);
    const link = container.querySelector('a');
    expect(link).toBeTruthy();
    expect(link!.getAttribute('href')).toBe('/api/chats/chat-1/messages/msg-1/media');
  });

  it('renders sticker with media proxy URL', () => {
    const msg = makeMessage({
      message_type: 'sticker',
    });
    const { container } = render(<ChatBubble message={msg} chatId={chatId} />);
    const img = container.querySelector('img[alt="Sticker"]');
    expect(img).toBeTruthy();
    expect(img!.getAttribute('src')).toBe('/api/chats/chat-1/messages/msg-1/media');
  });

  it('shows time for valid timestamps', () => {
    const msg = makeMessage({ timestamp: '2024-01-15T10:30:00Z' });
    render(<ChatBubble message={msg} chatId={chatId} />);
    const timeEl = screen.getByText(/\d{1,2}:\d{2}/);
    expect(timeEl).toBeInTheDocument();
  });

  it('aligns own messages to the right', () => {
    const msg = makeMessage({ is_from_me: true });
    const { container } = render(<ChatBubble message={msg} chatId={chatId} />);
    const wrapper = container.firstElementChild as HTMLElement;
    expect(wrapper.className).toContain('justify-end');
  });

  it('aligns received messages to the left', () => {
    const msg = makeMessage({ is_from_me: false });
    const { container } = render(<ChatBubble message={msg} chatId={chatId} />);
    const wrapper = container.firstElementChild as HTMLElement;
    expect(wrapper.className).toContain('justify-start');
  });
});
