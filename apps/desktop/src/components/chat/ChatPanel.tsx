import { useState, useRef, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SystemSnapshot, ChatMessage } from '../../types';
import { useI18n } from '../../i18n';
import './ChatPanel.css';

interface ChatPanelProps {
  snapshot: SystemSnapshot | null;
}

function MessageBubble({ msg }: { msg: ChatMessage }) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    await navigator.clipboard.writeText(msg.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className={`msg msg-${msg.sender}`}>
      <div className="msg-bubble">
        {/* Render basic markdown-like bold/newlines */}
        <div
          className="msg-content"
          dangerouslySetInnerHTML={{
            __html: msg.content
              .replace(/&/g, '&amp;')
              .replace(/</g, '&lt;')
              .replace(/>/g, '&gt;')
              .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
              .replace(/### (.+)/g, '<h4>$1</h4>')
              .replace(/\n/g, '<br/>'),
          }}
        />
        <div className="msg-footer">
          <span className="msg-ts">{new Date(msg.timestamp).toLocaleTimeString()}</span>
          {msg.sender === 'assistant' && (
            <button className="copy-btn" onClick={copy} title={t.chat_copy}>
              {copied ? `✓ ${t.chat_copied}` : t.chat_copy}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

export function ChatPanel({ snapshot }: ChatPanelProps) {
  const { t, lang } = useI18n();
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: 'welcome',
      sender: 'assistant',
      content: lang === 'vi'
        ? 'Xin chào! Tôi là **TBK Core** — trợ lý chăm sóc máy tính thông minh của bạn. Tôi có thể phân tích dữ liệu CPU, RAM và ổ đĩa thực tế từ máy của bạn.\n\nBạn muốn hỏi gì về máy tính hôm nay?'
        : 'Hello! I am **TBK Core** — your AI-powered computer health assistant. I can analyze real CPU, RAM and disk data from your machine.\n\nWhat would you like to know about your computer today?',
      timestamp: new Date().toISOString(),
    }
  ]);
  const [input, setInput] = useState('');
  const [thinking, setThinking] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, thinking]);

  const sendMessage = useCallback(async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed || thinking) return;

    const userMsg: ChatMessage = {
      id: crypto.randomUUID(),
      sender: 'user',
      content: trimmed,
      timestamp: new Date().toISOString(),
    };
    setMessages(prev => [...prev, userMsg]);
    setInput('');
    setThinking(true);

    try {
      const resp = await invoke<{ reply: string; references: string[] }>('query_assistant', {
        prompt: trimmed,
        lang,
      });
      const assistantMsg: ChatMessage = {
        id: crypto.randomUUID(),
        sender: 'assistant',
        content: resp.reply,
        timestamp: new Date().toISOString(),
        references: resp.references,
      };
      setMessages(prev => [...prev, assistantMsg]);
    } catch (err) {
      const errMsg: ChatMessage = {
        id: crypto.randomUUID(),
        sender: 'assistant',
        content: `⚠️ Error: ${String(err)}`,
        timestamp: new Date().toISOString(),
      };
      setMessages(prev => [...prev, errMsg]);
    } finally {
      setThinking(false);
    }
  }, [thinking, lang]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage(input);
    }
  };

  const quickPrompts = [
    t.quick_health,
    t.quick_cpu,
    t.quick_ram,
    t.quick_disk,
    t.quick_maintenance,
  ];

  return (
    <div className="chat-panel">
      <h1 className="page-title">{t.nav_chat}</h1>

      {/* Context strip — shows live metrics */}
      {snapshot && (
        <div className="context-strip">
          <div className="ctx-chip">
            🖥️ CPU {snapshot.cpu.global_usage_percent.toFixed(0)}%
          </div>
          <div className="ctx-chip">
            💾 RAM {snapshot.memory.usage_percent.toFixed(0)}%
          </div>
          {snapshot.disks[0] && (
            <div className="ctx-chip">
              🗄️ {snapshot.disks[0].mount_point} {snapshot.disks[0].usage_percent.toFixed(0)}%
            </div>
          )}
          {snapshot.findings.length > 0 && (
            <div className="ctx-chip alert">
              ⚠️ {snapshot.findings.length} finding{snapshot.findings.length > 1 ? 's' : ''}
            </div>
          )}
        </div>
      )}

      {/* Message history */}
      <div className="chat-history">
        {messages.map(msg => <MessageBubble key={msg.id} msg={msg} />)}
        {thinking && (
          <div className="msg msg-assistant">
            <div className="msg-bubble thinking-bubble">
              <span className="dot" /><span className="dot" /><span className="dot" />
              <span className="thinking-label">{t.chat_thinking}</span>
            </div>
          </div>
        )}
        <div ref={bottomRef} />
      </div>

      {/* Quick prompts */}
      {messages.length <= 1 && (
        <div className="quick-prompts">
          <span className="quick-label">{t.chat_quick_ask}</span>
          <div className="quick-buttons">
            {quickPrompts.map((p, i) => (
              <button key={i} className="quick-btn" onClick={() => sendMessage(p)}>
                {p}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Input area */}
      <div className="chat-input-area">
        <textarea
          ref={textareaRef}
          className="chat-textarea"
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={t.chat_placeholder}
          rows={2}
          disabled={thinking}
          aria-label="Chat input"
        />
        <button
          className="send-btn"
          onClick={() => sendMessage(input)}
          disabled={thinking || !input.trim()}
          aria-label={t.chat_send}
        >
          {thinking ? '⏳' : '➤'}
        </button>
      </div>
    </div>
  );
}
