import { useState } from 'react';

type FeedbackCategory = 'bug' | 'feature' | 'general';

const categoryLabels: Record<FeedbackCategory, string> = {
  bug: 'Bug Report',
  feature: 'Feature Request',
  general: 'General Feedback',
};

export function FeedbackButton() {
  const [isOpen, setIsOpen] = useState(false);
  const [category, setCategory] = useState<FeedbackCategory>('general');
  const [description, setDescription] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function reset() {
    setCategory('general');
    setDescription('');
    setError(null);
    setSubmitted(false);
  }

  function handleClose() {
    setIsOpen(false);
    // Reset after animation
    setTimeout(reset, 200);
  }

  async function handleSubmit() {
    if (!description.trim()) {
      setError('Please enter a description.');
      return;
    }
    setSubmitting(true);
    setError(null);

    try {
      const headers: Record<string, string> = { 'Content-Type': 'application/json' };
      const token = localStorage.getItem('swarmcrest_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/feedback', {
        method: 'POST',
        headers,
        body: JSON.stringify({ category, description }),
      });

      if (!res.ok) {
        const body = await res.text().catch(() => 'Unknown error');
        throw new Error(body);
      }

      setSubmitted(true);
      setTimeout(handleClose, 1500);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to submit feedback');
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      {/* Floating button */}
      <button
        onClick={() => { reset(); setIsOpen(true); }}
        style={{
          position: 'fixed',
          bottom: 20,
          right: 20,
          width: 44,
          height: 44,
          borderRadius: '50%',
          background: '#16c79a',
          color: '#0a0a1a',
          border: 'none',
          cursor: 'pointer',
          fontSize: 20,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          boxShadow: '0 2px 8px rgba(0,0,0,0.3)',
          zIndex: 1000,
          transition: 'transform 0.15s, background 0.15s',
        }}
        onMouseOver={e => { e.currentTarget.style.transform = 'scale(1.1)'; e.currentTarget.style.background = '#1de9b6'; }}
        onMouseOut={e => { e.currentTarget.style.transform = 'scale(1)'; e.currentTarget.style.background = '#16c79a'; }}
        title="Send Feedback"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
        </svg>
      </button>

      {/* Modal overlay */}
      {isOpen && (
        <div
          onClick={handleClose}
          style={{
            position: 'fixed',
            inset: 0,
            background: 'rgba(0,0,0,0.5)',
            zIndex: 1001,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <div
            onClick={e => e.stopPropagation()}
            style={{
              background: '#16213e',
              borderRadius: 12,
              padding: 28,
              width: 420,
              maxWidth: '90vw',
              border: '1px solid #1a3a5c',
              boxShadow: '0 8px 32px rgba(0,0,0,0.4)',
            }}
          >
            {submitted ? (
              <div style={{ textAlign: 'center', padding: '20px 0' }}>
                <div style={{ fontSize: 36, marginBottom: 12 }}>&#10003;</div>
                <p style={{ color: '#16c79a', fontSize: 16, margin: 0 }}>
                  Thank you for your feedback!
                </p>
              </div>
            ) : (
              <>
                <h3 style={{ color: '#e0e0e0', marginTop: 0, marginBottom: 20 }}>Send Feedback</h3>

                <label style={{ color: '#aaa', fontSize: 13, display: 'block', marginBottom: 6 }}>
                  Category
                </label>
                <select
                  value={category}
                  onChange={e => setCategory(e.target.value as FeedbackCategory)}
                  style={{
                    width: '100%',
                    padding: '8px 12px',
                    background: '#0a0a1a',
                    color: '#e0e0e0',
                    border: '1px solid #333',
                    borderRadius: 6,
                    fontSize: 14,
                    marginBottom: 16,
                    fontFamily: 'inherit',
                  }}
                >
                  {(Object.keys(categoryLabels) as FeedbackCategory[]).map(key => (
                    <option key={key} value={key}>{categoryLabels[key]}</option>
                  ))}
                </select>

                <label style={{ color: '#aaa', fontSize: 13, display: 'block', marginBottom: 6 }}>
                  Description
                </label>
                <textarea
                  value={description}
                  onChange={e => setDescription(e.target.value)}
                  placeholder="Describe your feedback..."
                  rows={5}
                  style={{
                    width: '100%',
                    padding: '8px 12px',
                    background: '#0a0a1a',
                    color: '#e0e0e0',
                    border: '1px solid #333',
                    borderRadius: 6,
                    fontSize: 14,
                    marginBottom: 16,
                    fontFamily: 'inherit',
                    resize: 'vertical',
                    boxSizing: 'border-box',
                  }}
                />

                {error && (
                  <p style={{ color: '#e94560', fontSize: 13, margin: '0 0 12px 0' }}>{error}</p>
                )}

                <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end' }}>
                  <button
                    onClick={handleClose}
                    style={{
                      padding: '8px 16px',
                      background: 'transparent',
                      color: '#aaa',
                      border: '1px solid #333',
                      borderRadius: 6,
                      cursor: 'pointer',
                      fontSize: 14,
                      fontFamily: 'inherit',
                    }}
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleSubmit}
                    disabled={submitting}
                    style={{
                      padding: '8px 16px',
                      background: submitting ? '#555' : '#16c79a',
                      color: '#0a0a1a',
                      border: 'none',
                      borderRadius: 6,
                      cursor: submitting ? 'not-allowed' : 'pointer',
                      fontSize: 14,
                      fontWeight: 600,
                      fontFamily: 'inherit',
                    }}
                  >
                    {submitting ? 'Submitting...' : 'Submit'}
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </>
  );
}
