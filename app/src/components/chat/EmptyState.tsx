import React from 'react';
import { Sparkles, Terminal, BookOpen, Lightbulb } from 'lucide-react';

interface EmptyStateProps {
  selectedModel: string;
  onSelectSuggestion: (prompt: string) => void;
}

export const EmptyState: React.FC<EmptyStateProps> = ({
  selectedModel,
  onSelectSuggestion,
}) => {
  const suggestions = [
    {
      icon: <Terminal size={16} color="#38bdf8" />,
      title: 'Rust Async Architecture',
      prompt: 'Write an asynchronous stream processor in Rust using tokio and futures-util with backpressure handling.',
    },
    {
      icon: <Lightbulb size={16} color="#facc15" />,
      title: 'Algorithm Optimization',
      prompt: 'Explain the difference between Dijkstra and A* search algorithms with practical performance trade-offs.',
    },
    {
      icon: <BookOpen size={16} color="#a78bfa" />,
      title: 'Explain Complex Systems',
      prompt: 'Explain how SQLite implements ACID transactions through Write-Ahead Logging (WAL) in concise terms.',
    },
    {
      icon: <Sparkles size={16} color="#34d399" />,
      title: 'UI Design Patterns',
      prompt: 'What are the key interaction design patterns for desktop AI assistants to maximize productivity?',
    },
  ];

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '3rem 1.5rem',
        maxWidth: '720px',
        margin: 'auto',
        textAlign: 'center',
      }}
    >
      {/* Hero Badge */}
      <div
        style={{
          width: '52px',
          height: '52px',
          borderRadius: 'var(--radius-lg)',
          background: 'linear-gradient(135deg, rgba(99, 102, 241, 0.2) 0%, rgba(59, 130, 246, 0.2) 100%)',
          border: '1px solid rgba(99, 102, 241, 0.35)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          marginBottom: '1.25rem',
          boxShadow: '0 8px 24px rgba(99, 102, 241, 0.15)',
        }}
      >
        <Sparkles size={24} color="#818cf8" />
      </div>

      <h1
        style={{
          fontSize: '1.6rem',
          fontWeight: 700,
          letterSpacing: '-0.02em',
          marginBottom: '0.5rem',
          color: 'var(--text-primary)',
        }}
      >
        What would you like to explore?
      </h1>

      <p
        style={{
          fontSize: '0.88rem',
          color: 'var(--text-secondary)',
          lineHeight: 1.5,
          marginBottom: '2rem',
          maxWidth: '520px',
        }}
      >
        Powered by your local <span style={{ color: 'var(--accent-primary)', fontWeight: 600 }}>{selectedModel}</span> model.
        Private, offline-capable, and accelerated directly on your device.
      </p>

      {/* Suggestion Cards Grid */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
          gap: '0.75rem',
          width: '100%',
        }}
      >
        {suggestions.map((item, idx) => (
          <button
            key={idx}
            type="button"
            onClick={() => onSelectSuggestion(item.prompt)}
            style={{
              display: 'flex',
              flexDirection: 'column',
              textAlign: 'left',
              padding: '0.9rem',
              backgroundColor: 'var(--bg-surface)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-md)',
              transition: 'all var(--transition-fast)',
              cursor: 'pointer',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.backgroundColor = 'var(--bg-surface-elevated)';
              e.currentTarget.style.borderColor = 'var(--border-medium)';
              e.currentTarget.style.transform = 'translateY(-2px)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.backgroundColor = 'var(--bg-surface)';
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
              e.currentTarget.style.transform = 'translateY(0)';
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.35rem' }}>
              {item.icon}
              <span style={{ fontWeight: 600, fontSize: '0.84rem', color: 'var(--text-primary)' }}>
                {item.title}
              </span>
            </div>
            <p
              style={{
                fontSize: '0.76rem',
                color: 'var(--text-secondary)',
                lineHeight: 1.45,
                margin: 0,
                display: '-webkit-box',
                WebkitLineClamp: 2,
                WebkitBoxOrient: 'vertical',
                overflow: 'hidden',
              }}
            >
              {item.prompt}
            </p>
          </button>
        ))}
      </div>
    </div>
  );
};
