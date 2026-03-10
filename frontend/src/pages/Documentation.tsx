import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';

type Section = 'getting-started' | 'lua-api' | 'rest-api' | 'strategy' | 'faq';

const sectionLabels: Record<Section, string> = {
  'getting-started': 'Getting Started',
  'lua-api': 'Lua API Reference',
  'rest-api': 'REST API & Auth',
  'strategy': 'Strategy Guide',
  'faq': 'FAQ / Troubleshooting',
};

function getInitialSection(): Section {
  const hash = window.location.hash.replace('#', '') as Section;
  if (hash && hash in sectionLabels) return hash;
  return 'getting-started';
}

export function Documentation() {
  const [activeSection, setActiveSection] = useState<Section>(getInitialSection);
  const [luaApiResult, setLuaApiResult] = useState<{ content?: string; error?: string } | null>(null);

  // Load Lua API content when that section is selected
  useEffect(() => {
    if (activeSection !== 'lua-api' || luaApiResult !== null) return;
    let cancelled = false;
    fetch('/api/docs/lua-api')
      .then(r => {
        if (!r.ok) throw new Error('Failed to load Lua API documentation');
        return r.text();
      })
      .then(text => { if (!cancelled) setLuaApiResult({ content: text }); })
      .catch(err => { if (!cancelled) setLuaApiResult({ error: err.message }); });
    return () => { cancelled = true; };
  }, [activeSection, luaApiResult]);

  const navigate = useNavigate();
  const switchSection = useCallback((section: Section) => {
    setActiveSection(section);
    navigate(`#${section}`, { replace: true });
  }, [navigate]);

  return (
    <div style={{ display: 'flex', flex: 1, minHeight: 0 }}>
      {/* Sidebar */}
      <nav style={{
        width: 220,
        minWidth: 220,
        background: '#0d1117',
        borderRight: '1px solid #1a3a5c',
        padding: '20px 0',
        overflowY: 'auto',
      }}>
        <h3 style={{ color: '#e0e0e0', padding: '0 16px', marginTop: 0, marginBottom: 16, fontSize: 15 }}>
          Documentation
        </h3>
        {(Object.keys(sectionLabels) as Section[]).map(key => (
          <button
            key={key}
            onClick={() => switchSection(key)}
            style={{
              display: 'block',
              width: '100%',
              padding: '10px 16px',
              background: activeSection === key ? 'rgba(22,199,154,0.12)' : 'transparent',
              border: 'none',
              borderLeft: activeSection === key ? '3px solid #16c79a' : '3px solid transparent',
              color: activeSection === key ? '#16c79a' : '#aaa',
              textAlign: 'left',
              cursor: 'pointer',
              fontSize: 14,
              fontFamily: 'inherit',
              transition: 'all 0.15s',
            }}
            onMouseOver={e => {
              if (activeSection !== key) e.currentTarget.style.color = '#e0e0e0';
            }}
            onMouseOut={e => {
              if (activeSection !== key) e.currentTarget.style.color = '#aaa';
            }}
          >
            {sectionLabels[key]}
          </button>
        ))}
      </nav>

      {/* Content */}
      <div style={{ flex: 1, overflowY: 'auto', padding: 32 }}>
        <div style={{ maxWidth: 800 }}>
          {activeSection === 'getting-started' && <GettingStarted />}
          {activeSection === 'lua-api' && (
            <LuaApiReference content={luaApiResult?.content ?? ''} loading={luaApiResult === null} error={luaApiResult?.error ?? null} />
          )}
          {activeSection === 'rest-api' && <RestApiDocs />}
          {activeSection === 'strategy' && <StrategyGuide />}
          {activeSection === 'faq' && <FAQ />}
        </div>
      </div>
    </div>
  );
}

/* ── Section Components ─────────────────────────────────────────────── */

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <>
      <h2 style={{ color: '#e0e0e0', marginTop: 0, marginBottom: 8 }}>{children}</h2>
      <div style={{
        width: 50,
        height: 3,
        background: 'linear-gradient(90deg, #16c79a, #f5a623)',
        borderRadius: 2,
        marginBottom: 24,
      }} />
    </>
  );
}

function Card({ title, children }: { title?: string; children: React.ReactNode }) {
  return (
    <div style={{
      background: '#16213e',
      borderRadius: 10,
      padding: 24,
      marginBottom: 20,
      border: '1px solid #1a3a5c',
    }}>
      {title && <h3 style={{ color: '#16c79a', marginTop: 0, marginBottom: 12, fontSize: 16 }}>{title}</h3>}
      {children}
    </div>
  );
}

function StepNumber({ n }: { n: number }) {
  return (
    <span style={{
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      width: 28,
      height: 28,
      borderRadius: '50%',
      background: '#16c79a',
      color: '#0a0a1a',
      fontWeight: 700,
      fontSize: 14,
      marginRight: 12,
      flexShrink: 0,
    }}>
      {n}
    </span>
  );
}

function CodeBlock({ children }: { children: string }) {
  return (
    <pre style={{
      background: '#0a0a1a',
      border: '1px solid #333',
      borderRadius: 6,
      padding: 16,
      color: '#ccc',
      fontSize: 13,
      lineHeight: 1.5,
      overflowX: 'auto',
      whiteSpace: 'pre-wrap',
      wordWrap: 'break-word',
      margin: '12px 0',
    }}>
      {children}
    </pre>
  );
}

const textStyle: React.CSSProperties = { color: '#ccc', lineHeight: 1.7, margin: '0 0 12px 0' };

/* ── Getting Started ────────────────────────────────────────────────── */

function GettingStarted() {
  return (
    <>
      <SectionTitle>Getting Started</SectionTitle>

      <Card title="Welcome to SwarmCrest">
        <p style={textStyle}>
          SwarmCrest is a competitive programming game where you write Lua scripts to control swarms
          of creatures. Your bots eat food, grow, fight enemies, and compete for territory --
          all autonomously based on the code you write.
        </p>
      </Card>

      <Card title="LLM-Friendly Documentation">
        <p style={textStyle}>
          Building a bot with the help of an AI assistant? SwarmCrest provides machine-readable
          documentation endpoints following the{' '}
          <a href="https://llmstxt.org/" target="_blank" rel="noopener noreferrer" style={{ color: '#16c79a' }}>
            llms.txt standard
          </a>:
        </p>
        <ul style={{ color: '#ccc', lineHeight: 2, paddingLeft: 20, margin: '0 0 12px 0' }}>
          <li>
            <a href="/llms.txt" target="_blank" rel="noopener noreferrer" style={{ color: '#16c79a' }}>
              <code style={{ color: '#16c79a' }}>/llms.txt</code>
            </a>{' '}
            -- A concise summary of the API, endpoints, and bot programming basics.
          </li>
          <li>
            <a href="/llms-full.txt" target="_blank" rel="noopener noreferrer" style={{ color: '#16c79a' }}>
              <code style={{ color: '#16c79a' }}>/llms-full.txt</code>
            </a>{' '}
            -- Complete documentation including game mechanics, creature stats, combat math,
            and the full Lua API reference.
          </li>
        </ul>
        <p style={{ ...textStyle, fontSize: 13 }}>
          Paste either URL into your LLM's context, or point tools like Cursor, Claude Code, or
          ChatGPT at them so they can help you write better bots.
        </p>
      </Card>

      <Card title="Quick Start">
        <div style={{ display: 'flex', alignItems: 'flex-start', marginBottom: 16 }}>
          <StepNumber n={1} />
          <div>
            <strong style={{ color: '#e0e0e0' }}>Create an Account</strong>
            <p style={{ ...textStyle, marginTop: 4 }}>
              Register for a free account to start creating bots and competing.
            </p>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'flex-start', marginBottom: 16 }}>
          <StepNumber n={2} />
          <div>
            <strong style={{ color: '#e0e0e0' }}>Create a Bot</strong>
            <p style={{ ...textStyle, marginTop: 4 }}>
              Go to the Bot Library and click "New Bot". Give it a name and description.
            </p>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'flex-start', marginBottom: 16 }}>
          <StepNumber n={3} />
          <div>
            <strong style={{ color: '#e0e0e0' }}>Write Your Code</strong>
            <p style={{ ...textStyle, marginTop: 4 }}>
              Open the Editor and write Lua code. The simplest bot uses the coroutine-style high-level API:
            </p>
            <CodeBlock>{`function Creature:main()
    while true do
        if self:tile_food() > 0 then
            self:eat()
        else
            local x1, y1, x2, y2 = world_size()
            self:moveto(math.random(x1, x2), math.random(y1, y2))
        end
    end
end`}</CodeBlock>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'flex-start', marginBottom: 16 }}>
          <StepNumber n={4} />
          <div>
            <strong style={{ color: '#e0e0e0' }}>Save a Version</strong>
            <p style={{ ...textStyle, marginTop: 4 }}>
              Click "Save Version" in the editor. Each save creates a new version you can use in matches.
            </p>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'flex-start' }}>
          <StepNumber n={5} />
          <div>
            <strong style={{ color: '#e0e0e0' }}>Start a Match</strong>
            <p style={{ ...textStyle, marginTop: 4 }}>
              Go to the Game page, select your bot and an opponent, pick a map, and start a match.
              Watch the game unfold in real-time with the built-in viewer.
            </p>
          </div>
        </div>
      </Card>

      <Card title="Two High-Level API Styles">
        <p style={textStyle}>
          The low-level API exposes engine functions directly (set_path, get_pos, etc.).
          On top of that, two high-level API styles wrap these into more convenient patterns.
          The engine auto-detects which style your bot uses -- just pick whichever feels natural:
        </p>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
          <div>
            <h4 style={{ color: '#f5a623', marginTop: 0, marginBottom: 8, fontSize: 14 }}>
              Coroutine Style (oo.lua)
            </h4>
            <p style={{ ...textStyle, fontSize: 13 }}>
              Define <code style={{ color: '#16c79a' }}>Creature:main()</code> as a coroutine.
              Use blocking methods like <code style={{ color: '#16c79a' }}>self:eat()</code> and{' '}
              <code style={{ color: '#16c79a' }}>self:moveto(x, y)</code>.
            </p>
          </div>
          <div>
            <h4 style={{ color: '#f5a623', marginTop: 0, marginBottom: 8, fontSize: 14 }}>
              State Machine Style (state.lua)
            </h4>
            <p style={{ ...textStyle, fontSize: 13 }}>
              Define <code style={{ color: '#16c79a' }}>bot()</code> with state functions and
              event handlers like <code style={{ color: '#16c79a' }}>onIdle()</code> and{' '}
              <code style={{ color: '#16c79a' }}>onTileFood()</code>.
            </p>
          </div>
        </div>
      </Card>
    </>
  );
}

/* ── Lua API Reference ──────────────────────────────────────────────── */

function LuaApiReference({ content, loading, error }: {
  content: string;
  loading: boolean;
  error: string | null;
}) {
  if (loading) return <div style={{ padding: 24, color: '#888' }}>Loading Lua API documentation...</div>;
  if (error) return <div style={{ padding: 24, color: '#e94560' }}>{error}</div>;

  // Parse markdown into styled sections
  return (
    <>
      <SectionTitle>Lua API Reference</SectionTitle>
      <Card>
        <div style={{ color: '#ccc', lineHeight: 1.7 }}>
          <MarkdownContent content={content} />
        </div>
      </Card>
    </>
  );
}

/** Simple markdown-to-JSX renderer for the Lua API docs */
function MarkdownContent({ content }: { content: string }) {
  const lines = content.split('\n');
  const elements: React.ReactNode[] = [];
  let i = 0;
  let key = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Code block
    if (line.startsWith('```')) {
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith('```')) {
        codeLines.push(lines[i]);
        i++;
      }
      i++; // skip closing ```
      elements.push(<CodeBlock key={key++}>{codeLines.join('\n')}</CodeBlock>);
      continue;
    }

    // Headers
    if (line.startsWith('### ')) {
      elements.push(
        <h4 key={key++} style={{ color: '#f5a623', marginTop: 20, marginBottom: 8, fontSize: 15 }}>
          {line.replace('### ', '')}
        </h4>
      );
      i++;
      continue;
    }
    if (line.startsWith('## ')) {
      elements.push(
        <h3 key={key++} style={{ color: '#16c79a', marginTop: 28, marginBottom: 12, fontSize: 18 }}>
          {line.replace('## ', '')}
        </h3>
      );
      i++;
      continue;
    }
    if (line.startsWith('# ')) {
      // Skip top-level header (we have our own)
      i++;
      continue;
    }

    // Horizontal rule
    if (line.match(/^---+$/)) {
      elements.push(<hr key={key++} style={{ border: 'none', borderTop: '1px solid #333', margin: '20px 0' }} />);
      i++;
      continue;
    }

    // Table
    if (line.includes('|') && i + 1 < lines.length && lines[i + 1].includes('---')) {
      const tableLines: string[] = [line];
      i++;
      while (i < lines.length && lines[i].includes('|')) {
        tableLines.push(lines[i]);
        i++;
      }
      elements.push(<MarkdownTable key={key++} lines={tableLines} />);
      continue;
    }

    // Empty line
    if (line.trim() === '') {
      i++;
      continue;
    }

    // Regular paragraph (may contain inline formatting)
    elements.push(
      <p key={key++} style={{ ...textStyle }}>
        <InlineMarkdown text={line} />
      </p>
    );
    i++;
  }

  return <>{elements}</>;
}

function InlineMarkdown({ text }: { text: string }) {
  // Handle **bold**, `code`, and plain text
  const parts: React.ReactNode[] = [];
  let remaining = text;
  let k = 0;

  while (remaining.length > 0) {
    // Bold
    const boldMatch = remaining.match(/^(.*?)\*\*(.*?)\*\*(.*)/s);
    if (boldMatch) {
      if (boldMatch[1]) parts.push(<InlineMarkdown key={k++} text={boldMatch[1]} />);
      parts.push(<strong key={k++} style={{ color: '#e0e0e0' }}>{boldMatch[2]}</strong>);
      remaining = boldMatch[3];
      continue;
    }

    // Inline code
    const codeMatch = remaining.match(/^(.*?)`(.*?)`(.*)/s);
    if (codeMatch) {
      if (codeMatch[1]) parts.push(<span key={k++}>{codeMatch[1]}</span>);
      parts.push(
        <code key={k++} style={{
          color: '#16c79a',
          background: 'rgba(22,199,154,0.1)',
          padding: '1px 5px',
          borderRadius: 3,
          fontSize: '0.9em',
        }}>
          {codeMatch[2]}
        </code>
      );
      remaining = codeMatch[3];
      continue;
    }

    // Plain text
    parts.push(<span key={k++}>{remaining}</span>);
    break;
  }

  return <>{parts}</>;
}

function MarkdownTable({ lines }: { lines: string[] }) {
  const parseRow = (line: string) =>
    line.split('|').map(c => c.trim()).filter(c => c.length > 0);

  const headers = parseRow(lines[0]);
  // Skip separator line (index 1)
  const rows = lines.slice(2).map(parseRow);

  return (
    <div style={{ overflowX: 'auto', margin: '12px 0' }}>
      <table style={{
        width: '100%',
        borderCollapse: 'collapse',
        fontSize: 13,
      }}>
        <thead>
          <tr>
            {headers.map((h, i) => (
              <th key={i} style={{
                textAlign: 'left',
                padding: '8px 12px',
                borderBottom: '2px solid #333',
                color: '#f5a623',
                fontWeight: 600,
              }}>
                <InlineMarkdown text={h} />
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, ri) => (
            <tr key={ri}>
              {row.map((cell, ci) => (
                <td key={ci} style={{
                  padding: '6px 12px',
                  borderBottom: '1px solid #1a3a5c',
                  color: '#ccc',
                }}>
                  <InlineMarkdown text={cell} />
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/* ── REST API & Authentication ──────────────────────────────────────── */

function RestApiDocs() {
  return (
    <>
      <SectionTitle>REST API & Authentication</SectionTitle>

      <Card title="Authentication with API Keys">
        <p style={textStyle}>
          To use the SwarmCrest API programmatically, you need an API key. API keys are long-lived tokens
          that don't expire (but can be revoked at any time).
        </p>

        <h4 style={{ color: '#f5a623', marginTop: 16, marginBottom: 8, fontSize: 14 }}>
          Creating an API Key
        </h4>
        <p style={textStyle}>
          Go to the <strong style={{ color: '#e0e0e0' }}>API Keys</strong> page in the web UI (nav bar, top right).
          Give your key a name and select the scopes you need. The key token (prefixed with{' '}
          <code style={{ color: '#16c79a' }}>sc_</code>) is shown only once -- copy and store it securely.
        </p>

        <h4 style={{ color: '#f5a623', marginTop: 16, marginBottom: 8, fontSize: 14 }}>
          Using Your API Key
        </h4>
        <p style={textStyle}>
          Include your API key in the <code style={{ color: '#16c79a' }}>Authorization</code> header of every request:
        </p>
        <CodeBlock>{`Authorization: Bearer sc_<your_key>`}</CodeBlock>
        <p style={textStyle}>Example:</p>
        <CodeBlock>{`# List your bots
curl http://localhost:3000/api/bots \\
  -H "Authorization: Bearer sc_a1b2c3..."

# Create a headless challenge match
curl -X POST http://localhost:3000/api/matches/challenge \\
  -H "Authorization: Bearer sc_a1b2c3..." \\
  -H "Content-Type: application/json" \\
  -d '{"bot_version_id": 1, "opponent_bot_version_id": 2, "format": "1v1", "headless": true}'`}</CodeBlock>
      </Card>

      <Card title="API Key Scopes">
        <p style={textStyle}>
          Scopes control what an API key can access. Specify them as a comma-separated string when
          creating the key. Available scopes:
        </p>
        <div style={{ overflowX: 'auto', margin: '12px 0' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
            <thead>
              <tr>
                <th style={apiThStyle}>Scope</th>
                <th style={apiThStyle}>Allows</th>
              </tr>
            </thead>
            <tbody>
              {[
                ['bots:read', 'List and view bots and versions'],
                ['bots:write', 'Create/update/delete bots and versions'],
                ['matches:read', 'View matches, replays, and leaderboards'],
                ['matches:write', 'Create challenges and start games'],
                ['teams:write', 'Create/manage teams'],
                ['api_keys:write', 'Create new API keys'],
                ['leaderboard:read', 'View leaderboard rankings'],
              ].map(([scope, desc]) => (
                <tr key={scope}>
                  <td style={apiTdStyle}><code style={{ color: '#16c79a' }}>{scope}</code></td>
                  <td style={apiTdStyle}>{desc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p style={{ ...textStyle, fontSize: 13 }}>
          Default scopes (if not specified): <code style={{ color: '#16c79a' }}>bots:read,matches:read,leaderboard:read</code>
        </p>
      </Card>

      <Card title="Key Endpoints">
        <h4 style={{ color: '#f5a623', marginTop: 0, marginBottom: 8, fontSize: 14 }}>
          Bots & Versions
        </h4>
        <CodeBlock>{`GET/POST /api/bots                    - List/create bots
GET/PUT/DELETE /api/bots/{id}         - Get/update/delete bot
GET/POST /api/bots/{id}/versions      - List/create versions
PUT /api/bots/{id}/active-version     - Set active version
GET /api/bots/{id}/stats              - Version stats`}</CodeBlock>

        <h4 style={{ color: '#f5a623', marginTop: 16, marginBottom: 8, fontSize: 14 }}>
          Matches & Challenges
        </h4>
        <CodeBlock>{`GET  /api/matches              - Recent matches
GET  /api/matches/mine         - Your match history (auth required)
GET  /api/matches/{id}         - Match details + participants
GET  /api/matches/{id}/replay  - Match replay data
POST /api/matches/challenge    - Create a challenge`}</CodeBlock>

        <h4 style={{ color: '#f5a623', marginTop: 16, marginBottom: 8, fontSize: 14 }}>
          Games, Tournaments & Leaderboards
        </h4>
        <CodeBlock>{`POST /api/game/start           - Start a live game
GET  /api/game/status          - Check if game is running
POST /api/game/stop            - Stop current game
GET  /api/games/active         - List active games
GET  /api/queue/status         - Match queue status
GET/POST /api/tournaments      - List/create tournaments
POST /api/tournaments/{id}/run - Run tournament
GET  /api/leaderboards/1v1     - 1v1 rankings
GET  /api/leaderboards/ffa     - FFA rankings
GET  /api/leaderboards/2v2     - 2v2 rankings`}</CodeBlock>

        <h4 style={{ color: '#f5a623', marginTop: 16, marginBottom: 8, fontSize: 14 }}>
          Other
        </h4>
        <CodeBlock>{`GET/POST /api/api-keys          - List/create API keys
DELETE   /api/api-keys/{id}     - Revoke an API key
GET/POST /api/teams             - List/create teams
GET      /api/notifications     - Your notifications
POST     /api/validate-lua      - Validate Lua code
GET      /api/maps              - Available maps
POST     /api/feedback          - Submit feedback
GET      /api/docs/lua-api      - Lua API reference (Markdown)
WS       /ws/game               - Live game WebSocket`}</CodeBlock>
      </Card>

      <Card title="Example: Automated Challenge via API Key">
        <p style={textStyle}>
          Create an API key from the web UI with <code style={{ color: '#16c79a' }}>bots:read,matches:read,matches:write</code> scopes,
          then use it to automate matches:
        </p>
        <CodeBlock>{`# Set your API key (created from the API Keys page in the web UI)
API_KEY="sc_your_key_here"

# List your bots to find version IDs
curl http://localhost:3000/api/bots \\
  -H "Authorization: Bearer $API_KEY"

# Create a headless 1v1 challenge
curl -X POST http://localhost:3000/api/matches/challenge \\
  -H "Authorization: Bearer $API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "bot_version_id": 1,
    "opponent_bot_version_id": 2,
    "format": "1v1",
    "headless": true
  }'

# Check your match history
curl http://localhost:3000/api/matches/mine \\
  -H "Authorization: Bearer $API_KEY"

# View match replay
curl http://localhost:3000/api/matches/1/replay \\
  -H "Authorization: Bearer $API_KEY"`}</CodeBlock>
      </Card>
    </>
  );
}

const apiThStyle: React.CSSProperties = {
  textAlign: 'left',
  padding: '8px 12px',
  borderBottom: '2px solid #333',
  color: '#f5a623',
  fontWeight: 600,
  fontSize: 13,
};

const apiTdStyle: React.CSSProperties = {
  padding: '6px 12px',
  borderBottom: '1px solid #1a3a5c',
  color: '#ccc',
  fontSize: 13,
};

/* ── Strategy Guide ─────────────────────────────────────────────────── */

function StrategyGuide() {
  return (
    <>
      <SectionTitle>Strategy Guide</SectionTitle>

      <Card title="Creature Type Stats">
        <p style={textStyle}>
          Understanding the exact stats is essential. All values below are from the engine source code.
        </p>
        <div style={{ overflowX: 'auto', margin: '12px 0' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
            <thead>
              <tr>
                <th style={apiThStyle}>Stat</th>
                <th style={apiThStyle}>Small (Type 0)</th>
                <th style={apiThStyle}>Big (Type 1)</th>
                <th style={apiThStyle}>Flyer (Type 2)</th>
              </tr>
            </thead>
            <tbody>
              {[
                ['Max Health', '10,000', '20,000', '5,000'],
                ['Max Food', '10,000', '20,000', '5,000'],
                ['Base Speed', '200 px/s', '400 px/s', '800 px/s'],
                ['Speed Bonus', '+625 × health/max', 'None', 'None'],
                ['Health Drain', '50/s (5/tick)', '70/s (7/tick)', '50/s (5/tick)'],
                ['Heal Rate', '500 HP/s', '300 HP/s', '600 HP/s'],
                ['Eat Rate', '800 food/s', '400 food/s', '600 food/s'],
                ['Can Spawn', 'No', 'Yes (Type 0)', 'No'],
                ['Can Feed', 'Yes (256 px range)', 'No', 'Yes (256 px range)'],
                ['Feed Speed', '400 food/s', '--', '400 food/s'],
                ['Flies Over Walls', 'No', 'No', 'Yes'],
              ].map(([stat, small, big, flyer]) => (
                <tr key={stat}>
                  <td style={{ ...apiTdStyle, fontWeight: 600 }}>{stat}</td>
                  <td style={apiTdStyle}>{small}</td>
                  <td style={apiTdStyle}>{big}</td>
                  <td style={apiTdStyle}>{flyer}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p style={{ ...textStyle, fontSize: 13 }}>
          Small creatures get a unique speed bonus based on current health: their effective speed is{' '}
          <code style={{ color: '#16c79a' }}>200 + 625 × (current_health / max_health)</code>, up to a
          max of 1,000 px/s. At full health a Small moves at 825 px/s -- nearly as fast as a Flyer.
        </p>
      </Card>

      <Card title="Combat Mechanics">
        <p style={textStyle}>
          Combat uses continuous DPS (damage per second). While a creature is in the ATTACK state and
          its target is within range, damage is applied every tick (100ms) as{' '}
          <code style={{ color: '#16c79a' }}>damage_per_sec × tick_delta / 1000</code>.
          Range is checked each tick using Euclidean (straight-line) distance in pixels.
          If the target moves out of range or dies, the attacker goes IDLE.
        </p>

        <h4 style={{ color: '#f5a623', marginTop: 16, marginBottom: 8, fontSize: 14 }}>
          Damage Table (DPS by attacker vs target type)
        </h4>
        <div style={{ overflowX: 'auto', margin: '12px 0' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
            <thead>
              <tr>
                <th style={apiThStyle}>Attacker</th>
                <th style={apiThStyle}>vs Small</th>
                <th style={apiThStyle}>vs Big</th>
                <th style={apiThStyle}>vs Flyer</th>
                <th style={apiThStyle}>Range (px)</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td style={{ ...apiTdStyle, fontWeight: 600 }}>Small</td>
                <td style={{ ...apiTdStyle, color: '#888' }}>0 (can't)</td>
                <td style={{ ...apiTdStyle, color: '#888' }}>0 (can't)</td>
                <td style={{ ...apiTdStyle, color: '#4caf50' }}>1,000/s</td>
                <td style={apiTdStyle}>768</td>
              </tr>
              <tr>
                <td style={{ ...apiTdStyle, fontWeight: 600 }}>Big</td>
                <td style={{ ...apiTdStyle, color: '#4caf50' }}>1,500/s</td>
                <td style={{ ...apiTdStyle, color: '#4caf50' }}>1,500/s</td>
                <td style={{ ...apiTdStyle, color: '#4caf50' }}>1,500/s</td>
                <td style={apiTdStyle}>512</td>
              </tr>
              <tr>
                <td style={{ ...apiTdStyle, fontWeight: 600 }}>Flyer</td>
                <td style={{ ...apiTdStyle, color: '#888' }}>0 (can't)</td>
                <td style={{ ...apiTdStyle, color: '#888' }}>0 (can't)</td>
                <td style={{ ...apiTdStyle, color: '#888' }}>0 (can't)</td>
                <td style={{ ...apiTdStyle, color: '#888' }}>--</td>
              </tr>
            </tbody>
          </table>
        </div>
        <p style={{ ...textStyle, fontSize: 13 }}>
          At 1,500 DPS, a Big kills another full-health Big (20,000 HP) in ~13.3 seconds.
          A Big kills a Small (10,000 HP) in ~6.7 seconds. A Small kills a Flyer (5,000 HP) in 5 seconds.
        </p>
      </Card>

      <Card title="Conversion & Spawning Costs">
        <h4 style={{ color: '#f5a623', marginTop: 0, marginBottom: 8, fontSize: 14 }}>
          Conversion (food cost, consumed at 1,000 food/s)
        </h4>
        <ul style={{ color: '#ccc', lineHeight: 2, paddingLeft: 20, margin: '0 0 16px 0' }}>
          <li>Small to Big: <strong style={{ color: '#e0e0e0' }}>8,000 food</strong></li>
          <li>Small to Flyer: <strong style={{ color: '#e0e0e0' }}>5,000 food</strong></li>
          <li>Big to Small: <strong style={{ color: '#e0e0e0' }}>8,000 food</strong></li>
          <li>Flyer to Small: <strong style={{ color: '#e0e0e0' }}>5,000 food</strong></li>
          <li>Big to Flyer / Flyer to Big: <strong style={{ color: '#e94560' }}>not allowed</strong></li>
        </ul>

        <h4 style={{ color: '#f5a623', marginTop: 0, marginBottom: 8, fontSize: 14 }}>
          Spawning (Big only)
        </h4>
        <ul style={{ color: '#ccc', lineHeight: 2, paddingLeft: 20, margin: 0 }}>
          <li>Food cost: <strong style={{ color: '#e0e0e0' }}>5,000</strong> (consumed at 2,000 food/s)</li>
          <li>Health cost: <strong style={{ color: '#e0e0e0' }}>4,000 HP</strong> (deducted at spawn start)</li>
          <li>Offspring type: Small (Type 0)</li>
        </ul>
      </Card>

      <Card title="Food & Tile Economy">
        <p style={textStyle}>
          Food spawns on tiles from map-defined spawner points. Food does not grow continuously --
          each spawner periodically places a chunk of food on a random tile within its radius.
        </p>
        <ul style={{ color: '#ccc', lineHeight: 2, paddingLeft: 20, margin: 0 }}>
          <li>Max food per tile: <strong style={{ color: '#e0e0e0' }}>9,999</strong></li>
          <li>Initial food at each spawner: <strong style={{ color: '#e0e0e0' }}>9,000</strong></li>
          <li>Respawn interval: map-dependent (typically 3,000-5,000ms per spawner)</li>
          <li>Respawn amount: map-dependent (typically 800 food per spawn event for generated maps)</li>
          <li>Food appears at a random tile within the spawner's radius (2-4 tiles)</li>
          <li>Food is a finite, contested resource. Eating rates vary by type: Small eats fastest (800/s),
            Flyer moderate (600/s), Big slowest (400/s).</li>
        </ul>
      </Card>

      <Card title="CPU Limits">
        <p style={textStyle}>
          Each player's Lua code is limited to <strong style={{ color: '#e0e0e0' }}>500,000 VM instructions per tick</strong>.
          If your bot exceeds this limit:
        </p>
        <ul style={{ color: '#ccc', lineHeight: 2, paddingLeft: 20, margin: 0 }}>
          <li>The current tick's <code style={{ color: '#16c79a' }}>player_think()</code> call is aborted.</li>
          <li>An error message <code style={{ color: '#16c79a' }}>"lua vm cycles exceeded"</code> is logged to your console output.</li>
          <li>No creature actions are executed for that tick.</li>
          <li>Your creatures are <strong style={{ color: '#e0e0e0' }}>not killed</strong> and your bot is <strong style={{ color: '#e0e0e0' }}>not kicked</strong> -- the game continues normally next tick.</li>
        </ul>
        <p style={{ ...textStyle, fontSize: 13, marginTop: 8 }}>
          Note: <code style={{ color: '#16c79a' }}>get_cpu_usage()</code> currently returns 0 (stub).
          To stay within limits, avoid expensive operations in tight loops and minimize per-tick computation.
        </p>
      </Card>

      <Card title="Growth Strategy">
        <ol style={{ color: '#ccc', lineHeight: 2, paddingLeft: 20, margin: 0 }}>
          <li>Start by eating food as Small creatures to build reserves (800 food/s eat rate).</li>
          <li>Convert to Big (8,000 food, takes 8 seconds at 1,000/s conversion speed).</li>
          <li>Spawn new Smalls from your Big (5,000 food + 4,000 HP cost).</li>
          <li>New Smalls repeat the cycle: eat, convert, spawn.</li>
          <li>Exponential growth is the key to domination.</li>
        </ol>
      </Card>

      <Card title="King of the Hill Tips">
        <ul style={{ color: '#ccc', lineHeight: 2, paddingLeft: 20, margin: 0 }}>
          <li>
            Use <code style={{ color: '#16c79a' }}>get_koth_pos()</code> to find the KotH tile.
          </li>
          <li>A creature must be IDLE on the tile to score. Moving or eating does not count.</li>
          <li>Keep a well-fed Big nearby to defend your king creature.</li>
          <li>
            Check <code style={{ color: '#16c79a' }}>king_player()</code> to see who currently holds the hill.
          </li>
        </ul>
      </Card>
    </>
  );
}

/* ── FAQ / Troubleshooting ──────────────────────────────────────────── */

function FAQ() {
  const faqs: { q: string; a: React.ReactNode }[] = [
    {
      q: 'Why isn\'t my bot moving?',
      a: (
        <>
          <p style={textStyle}>Common causes:</p>
          <ul style={{ color: '#ccc', lineHeight: 1.8, paddingLeft: 20, margin: 0 }}>
            <li>
              You are not calling <code style={{ color: '#16c79a' }}>self:moveto(x, y)</code> or{' '}
              <code style={{ color: '#16c79a' }}>set_path(id, x, y)</code>.
            </li>
            <li>The destination is inside a wall (TILE_SOLID). Check with <code style={{ color: '#16c79a' }}>get_tile_type()</code>.</li>
            <li>Your creature is in a different state (eating, healing, etc.). Set state to WALK first.</li>
            <li>Your main loop exited. Use <code style={{ color: '#16c79a' }}>while true do ... end</code>.</li>
          </ul>
        </>
      ),
    },
    {
      q: 'I get a Lua error "attempt to call a nil value"',
      a: (
        <p style={textStyle}>
          This usually means you misspelled a function name, or you are mixing API styles.
          In coroutine-style bots (oo.lua), use <code style={{ color: '#16c79a' }}>self:eat()</code>.
          In state-machine-style bots (state.lua), use <code style={{ color: '#16c79a' }}>eat()</code> directly.
          Check the API Reference for the correct function names.
        </p>
      ),
    },
    {
      q: 'My creatures keep dying of starvation',
      a: (
        <p style={textStyle}>
          All creatures continuously lose health (50-70 HP/sec depending on type). You need to
          eat food and heal regularly. Prioritize eating when food is available, and heal when health
          drops below 50%.
        </p>
      ),
    },
    {
      q: 'How do I spawn new creatures?',
      a: (
        <p style={textStyle}>
          Only Type 1 (Big) creatures can spawn. Convert a Type 0 to Type 1 using{' '}
          <code style={{ color: '#16c79a' }}>self:convert(1)</code> (costs 8,000 food), then call{' '}
          <code style={{ color: '#16c79a' }}>self:spawn()</code> (costs 5,000 food + 20% health).
          The new creature will be Type 0.
        </p>
      ),
    },
    {
      q: 'What\'s the difference between the two high-level API styles?',
      a: (
        <p style={textStyle}>
          Both are wrappers around the same low-level API. The coroutine style (oo.lua)
          uses <code style={{ color: '#16c79a' }}>Creature:main()</code> with
          blocking methods (moveto, eat, etc.). The state machine style (state.lua) uses{' '}
          <code style={{ color: '#16c79a' }}>bot()</code> with named state functions and event callbacks.
          The engine auto-detects which style your code uses -- no configuration needed.
        </p>
      ),
    },
    {
      q: 'How do I attack other players\' creatures?',
      a: (
        <>
          <p style={textStyle}>
            Use <code style={{ color: '#16c79a' }}>self:attack(target)</code> (OO) or{' '}
            <code style={{ color: '#16c79a' }}>attack(target)</code> (State). Note:
          </p>
          <ul style={{ color: '#ccc', lineHeight: 1.8, paddingLeft: 20, margin: 0 }}>
            <li>Type 0 (Small) can only attack Type 2 (Flyers).</li>
            <li>Type 1 (Big) can attack everything.</li>
            <li>Type 2 (Flyers) cannot attack at all.</li>
          </ul>
          <p style={{ ...textStyle, marginTop: 8 }}>
            Find enemies with <code style={{ color: '#16c79a' }}>self:nearest_enemy()</code>.
          </p>
        </>
      ),
    },
    {
      q: 'My bot code seems correct but nothing happens',
      a: (
        <p style={textStyle}>
          Make sure you saved a version (not just edited code). The game uses the saved bot version,
          not the editor contents. Also verify your bot has no syntax errors using the editor's
          validation feature.
        </p>
      ),
    },
    {
      q: 'Can I use external Lua libraries?',
      a: (
        <p style={textStyle}>
          No. Bots run in a sandboxed Lua 5.1 environment. Standard Lua libraries (math, string, table)
          are available, but you cannot require external modules. File I/O, OS access, and networking
          are disabled for security.
        </p>
      ),
    },
  ];

  return (
    <>
      <SectionTitle>FAQ / Troubleshooting</SectionTitle>

      {faqs.map((faq, i) => (
        <Card key={i}>
          <h4 style={{ color: '#f5a623', marginTop: 0, marginBottom: 12, fontSize: 15 }}>
            {faq.q}
          </h4>
          {faq.a}
        </Card>
      ))}
    </>
  );
}
