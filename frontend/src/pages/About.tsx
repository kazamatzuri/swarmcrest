export function About() {
  return (
    <div style={{ padding: 32, maxWidth: 800, margin: '0 auto' }}>
      <h2 style={{ color: '#e0e0e0', marginBottom: 8, fontSize: 28 }}>About SwarmCrest</h2>
      <div style={{
        width: 60,
        height: 3,
        background: 'linear-gradient(90deg, #16c79a, #f5a623)',
        borderRadius: 2,
        marginBottom: 32,
      }} />

      <section style={{
        background: '#16213e',
        borderRadius: 12,
        padding: 28,
        marginBottom: 24,
        border: '1px solid #1a3a5c',
      }}>
        <h3 style={{ color: '#16c79a', marginTop: 0, marginBottom: 12 }}>The Game</h3>
        <p style={{ color: '#ccc', lineHeight: 1.7, margin: 0 }}>
          SwarmCrest is based on Infon Battle Arena, created by <strong style={{ color: '#f5a623' }}>Florian Wesch</strong>.
          It is a programming game where players write Lua scripts to control swarms of creatures competing for
          food and territory. This web version brings the classic gameplay to the browser.
        </p>
      </section>

      <section style={{
        background: '#16213e',
        borderRadius: 12,
        padding: 28,
        marginBottom: 24,
        border: '1px solid #1a3a5c',
      }}>
        <h3 style={{ color: '#16c79a', marginTop: 0, marginBottom: 12 }}>How It Works</h3>
        <p style={{ color: '#ccc', lineHeight: 1.7, margin: '0 0 12px 0' }}>
          Players write Lua 5.1 scripts that control autonomous creatures in a shared 2D world.
          The game runs in real-time ticks (100ms each), and every tick each creature's code is
          executed to make decisions: eat food, move, attack enemies, spawn new creatures, or
          compete for the King of the Hill.
        </p>
        <p style={{ color: '#ccc', lineHeight: 1.7, margin: 0 }}>
          There are three creature types -- Small (balanced), Big (tank), and Flyer (scout) --
          each with unique stats and abilities. Strategy emerges from managing resources, choosing
          when to grow your swarm, and how to engage opponents.
        </p>
      </section>

      <section style={{
        background: '#16213e',
        borderRadius: 12,
        padding: 28,
        marginBottom: 24,
        border: '1px solid #1a3a5c',
      }}>
        <h3 style={{ color: '#16c79a', marginTop: 0, marginBottom: 12 }}>Original Game</h3>
        <p style={{ color: '#ccc', lineHeight: 1.7, margin: '0 0 12px 0' }}>
          The original Infon Battle Arena is a C server with embedded Lua, featuring SDL/OpenGL
          renderers and telnet-based gameplay. It has been described as "like CoreWars on steroids."
        </p>
        <a
          href="http://infon.dividuum.de/"
          target="_blank"
          rel="noopener noreferrer"
          style={{
            display: 'inline-block',
            color: '#16c79a',
            textDecoration: 'none',
            padding: '8px 16px',
            border: '1px solid #16c79a',
            borderRadius: 6,
            fontSize: 14,
            transition: 'background 0.2s',
          }}
          onMouseOver={e => (e.currentTarget.style.background = 'rgba(22,199,154,0.1)')}
          onMouseOut={e => (e.currentTarget.style.background = 'transparent')}
        >
          Visit Original Infon Project
        </a>
      </section>

      <section style={{
        background: '#16213e',
        borderRadius: 12,
        padding: 28,
        marginBottom: 24,
        border: '1px solid #1a3a5c',
      }}>
        <h3 style={{ color: '#16c79a', marginTop: 0, marginBottom: 12 }}>Technology</h3>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
          <div>
            <h4 style={{ color: '#f5a623', marginTop: 0, marginBottom: 8, fontSize: 14 }}>Backend</h4>
            <ul style={{ color: '#ccc', margin: 0, paddingLeft: 20, lineHeight: 1.8 }}>
              <li>Rust with Axum</li>
              <li>Lua 5.1 via mlua (sandboxed)</li>
              <li>SQLite database</li>
              <li>WebSocket live streaming</li>
            </ul>
          </div>
          <div>
            <h4 style={{ color: '#f5a623', marginTop: 0, marginBottom: 8, fontSize: 14 }}>Frontend</h4>
            <ul style={{ color: '#ccc', margin: 0, paddingLeft: 20, lineHeight: 1.8 }}>
              <li>React + TypeScript</li>
              <li>Monaco Editor (Lua)</li>
              <li>Canvas 2D Renderer</li>
              <li>Real-time game viewer</li>
            </ul>
          </div>
        </div>
      </section>

      <section style={{
        background: '#16213e',
        borderRadius: 12,
        padding: 28,
        border: '1px solid #1a3a5c',
      }}>
        <h3 style={{ color: '#16c79a', marginTop: 0, marginBottom: 12 }}>License</h3>
        <p style={{ color: '#ccc', lineHeight: 1.7, margin: '0 0 16px 0' }}>
          SwarmCrest is open-source software released under the{' '}
          <strong style={{ color: '#e0e0e0' }}>GNU General Public License (GPL)</strong>,
          matching the license of the original game.
        </p>
        <a
          href="https://github.com/kazamatzuri/infon"
          target="_blank"
          rel="noopener noreferrer"
          style={{
            display: 'inline-block',
            color: '#f5a623',
            textDecoration: 'none',
            padding: '8px 16px',
            border: '1px solid #f5a623',
            borderRadius: 6,
            fontSize: 14,
            transition: 'background 0.2s',
          }}
          onMouseOver={e => (e.currentTarget.style.background = 'rgba(245,166,35,0.1)')}
          onMouseOut={e => (e.currentTarget.style.background = 'transparent')}
        >
          View Source on GitHub
        </a>
      </section>
    </div>
  );
}
