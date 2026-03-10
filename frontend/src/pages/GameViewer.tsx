import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { GameCanvas } from '../components/GameCanvas';
import { api } from '../api/client';
import type { Bot, BotVersion, MapInfo } from '../api/client';

function getWsUrl() {
  const base = `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws/game`;
  const token = localStorage.getItem('swarmcrest_token');
  return token ? `${base}?token=${encodeURIComponent(token)}` : base;
}

interface PlayerSlot {
  botId: number | null;
  versionId: number | null;
  name: string;
}

export function GameViewer() {
  const navigate = useNavigate();
  const [phase, setPhase] = useState<'loading' | 'setup' | 'running'>('loading');
  const [bots, setBots] = useState<Bot[]>([]);
  const [versions, setVersions] = useState<Record<number, BotVersion[]>>({});
  const [slots, setSlots] = useState<PlayerSlot[]>([
    { botId: null, versionId: null, name: 'Player 1' },
    { botId: null, versionId: null, name: 'Player 2' },
  ]);
  const [maps, setMaps] = useState<MapInfo[]>([]);
  const [selectedMap, setSelectedMap] = useState<string>('random');
  const [error, setError] = useState('');
  const [starting, setStarting] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [gameEnded, setGameEnded] = useState(false);
  const [headless, setHeadless] = useState(false);
  const [queuedMatchId, setQueuedMatchId] = useState<number | null>(null);
  const [mapWidth, setMapWidth] = useState(30);
  const [mapHeight, setMapHeight] = useState(30);
  const [mapFoodSpots, setMapFoodSpots] = useState(10);

  const isRandomMap = !selectedMap || selectedMap === 'random' || selectedMap === 'default';

  // Check game status and load bots on mount
  useEffect(() => {
    (async () => {
      try {
        const [status, botList, mapList] = await Promise.all([
          api.gameStatus(),
          api.listBots(),
          api.listMaps().catch(() => [] as MapInfo[]),
        ]);
        setBots(botList);
        setMaps(mapList);
        if (status.running) {
          setPhase('running');
        } else {
          setPhase('setup');
        }
      } catch {
        setPhase('setup');
      }
    })();
  }, []);

  // Load versions when a bot is selected
  const loadVersions = async (botId: number) => {
    if (versions[botId]) return;
    try {
      const v = await api.listVersions(botId);
      setVersions(prev => ({ ...prev, [botId]: v }));
    } catch {
      // ignore
    }
  };

  const updateSlot = (index: number, updates: Partial<PlayerSlot>) => {
    setSlots(prev => prev.map((s, i) => i === index ? { ...s, ...updates } : s));
  };

  const handleBotSelect = async (index: number, botId: number) => {
    const bot = bots.find(b => b.id === botId);
    updateSlot(index, { botId, versionId: null, name: bot?.name || `Player ${index + 1}` });
    await loadVersions(botId);
    // Auto-select latest version
    const v = versions[botId];
    if (v && v.length > 0) {
      updateSlot(index, { botId, versionId: v[v.length - 1].id });
    }
  };

  // Re-select latest version once versions load
  useEffect(() => {
    setSlots(prev => prev.map(s => {
      if (s.botId && !s.versionId && versions[s.botId]?.length) {
        return { ...s, versionId: versions[s.botId][versions[s.botId].length - 1].id };
      }
      return s;
    }));
  }, [versions]);

  const addSlot = () => {
    setSlots(prev => [...prev, { botId: null, versionId: null, name: `Player ${prev.length + 1}` }]);
  };

  const removeSlot = (index: number) => {
    if (slots.length <= 2) return;
    setSlots(prev => prev.filter((_, i) => i !== index));
  };

  const handleStart = async () => {
    setError('');
    setQueuedMatchId(null);
    const players = slots
      .filter(s => s.versionId !== null)
      .map(s => ({ bot_version_id: s.versionId!, name: s.name || undefined }));

    if (players.length < 2) {
      setError('Select at least 2 bots to start a game.');
      return;
    }

    setStarting(true);
    try {
      const mapParams = isRandomMap ? { width: mapWidth, height: mapHeight, num_food_spots: mapFoodSpots } : undefined;
      const result = await api.startGame(players, selectedMap, headless, mapParams);
      if (result.status === 'queued' && result.match_id) {
        setQueuedMatchId(result.match_id);
      } else {
        setPhase('running');
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to start game');
    } finally {
      setStarting(false);
    }
  };

  const handleStop = async () => {
    setStopping(true);
    try {
      await api.stopGame();
    } catch {
      // Game may have already ended — that's fine
    }
    // Always transition back to setup
    setTimeout(() => {
      setPhase('setup');
      setStopping(false);
    }, 500);
  };

  if (phase === 'loading') {
    return <div style={{ padding: 40, color: '#888', textAlign: 'center' }}>Loading...</div>;
  }

  if (phase === 'running') {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
        <div style={{ padding: '8px 24px', background: '#16213e', borderBottom: '1px solid #333', display: 'flex', alignItems: 'center', gap: '16px' }}>
          <span style={{ color: '#e0e0e0', fontWeight: 600, fontSize: '14px' }}>
            {gameEnded ? 'Game Finished' : 'Live Game'}
          </span>
          <div style={{ flex: 1 }} />
          {!gameEnded && (
            <button onClick={handleStop} disabled={stopping} style={btnStop}>
              {stopping ? 'Stopping...' : 'Stop Game'}
            </button>
          )}
        </div>
        <div style={{ flex: 1, minHeight: 0 }}>
          <GameCanvas
            wsUrl={getWsUrl()}
            onGameEnd={() => setGameEnded(true)}
            onNewGame={() => {
              setGameEnded(false);
              setPhase('setup');
            }}
          />
        </div>
      </div>
    );
  }

  // Setup phase
  return (
    <div style={{ display: 'flex', justifyContent: 'center', padding: '32px', gap: '0', maxWidth: '1000px', margin: '0 auto' }}>
      {/* Main form */}
      <div style={{ maxWidth: '700px', width: '100%', flexShrink: 1, minWidth: 0 }}>
      <h2 style={{ color: '#e0e0e0', marginBottom: '8px' }}>Start a Game</h2>
      <p style={{ color: '#888', fontSize: '14px', marginBottom: '24px' }}>
        Select bots from your library to compete against each other.
      </p>

      {error && (
        <div style={{ background: '#e9456020', border: '1px solid #e94560', borderRadius: '6px', padding: '10px 16px', marginBottom: '16px', color: '#e94560', fontSize: '13px' }}>
          {error}
        </div>
      )}

      {queuedMatchId && (
        <div style={{ background: '#16c79a20', border: '1px solid #16c79a', borderRadius: '6px', padding: '12px 16px', marginBottom: '16px', color: '#16c79a', fontSize: '13px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <span>Headless game queued (Match #{queuedMatchId}).</span>
          <button
            onClick={() => navigate(`/matches/${queuedMatchId}`)}
            style={{ background: '#16c79a', color: '#fff', border: 'none', padding: '4px 14px', borderRadius: '4px', cursor: 'pointer', fontWeight: 600, fontSize: '12px' }}
          >
            View Match
          </button>
        </div>
      )}

      {bots.length === 0 ? (
        <div style={{ background: '#16213e', borderRadius: '8px', padding: '32px', textAlign: 'center' }}>
          <p style={{ color: '#888', marginBottom: '8px' }}>No bots in your library yet.</p>
          <a href="/editor" style={{ color: '#f5a623' }}>Create a bot first</a>
        </div>
      ) : (
        <>
          <div style={{ marginBottom: '1rem', display: 'flex', gap: '24px', alignItems: 'center', flexWrap: 'wrap' }}>
            <div>
              <label htmlFor="map-select" style={{ fontWeight: 'bold', marginRight: '0.5rem' }}>Map:</label>
              <select
                id="map-select"
                value={selectedMap}
                onChange={(e) => setSelectedMap(e.target.value)}
                style={inputStyle}
              >
                <option value="random">Random</option>
                {maps.filter(m => m.name !== 'random').map(m => (
                  <option key={m.name} value={m.name}>
                    {m.name} ({m.width}x{m.height})
                  </option>
                ))}
              </select>
            </div>
            <label style={{ display: 'flex', alignItems: 'center', gap: '6px', cursor: 'pointer', fontSize: '13px', color: '#ccc' }}>
              <input
                type="checkbox"
                checked={headless}
                onChange={e => { setHeadless(e.target.checked); setQueuedMatchId(null); }}
                style={{ accentColor: '#f5a623' }}
              />
              Headless (faster, no live view)
            </label>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', marginBottom: '20px' }}>
            {slots.map((slot, i) => (
              <div key={i} style={{ background: '#16213e', borderRadius: '8px', padding: '12px 16px', display: 'flex', alignItems: 'center', gap: '12px' }}>
                <input
                  type="text"
                  value={slot.name}
                  onChange={e => updateSlot(i, { name: e.target.value })}
                  style={{ ...inputStyle, width: '120px', flexShrink: 0 }}
                  placeholder="Name"
                />

                <select
                  value={slot.botId ?? ''}
                  onChange={e => {
                    const id = Number(e.target.value);
                    if (id) handleBotSelect(i, id);
                  }}
                  style={{ ...inputStyle, flex: 1 }}
                >
                  <option value="">-- Select Bot --</option>
                  {bots.map(b => (
                    <option key={b.id} value={b.id}>{b.name}</option>
                  ))}
                </select>

                {slot.botId && versions[slot.botId] && (
                  <select
                    value={slot.versionId ?? ''}
                    onChange={e => updateSlot(i, { versionId: Number(e.target.value) })}
                    style={{ ...inputStyle, width: '100px' }}
                  >
                    <option value="">Version</option>
                    {versions[slot.botId].map(v => (
                      <option key={v.id} value={v.id}>v{v.version}</option>
                    ))}
                  </select>
                )}

                {slots.length > 2 && (
                  <button onClick={() => removeSlot(i)} style={btnRemove} title="Remove player">
                    X
                  </button>
                )}
              </div>
            ))}
          </div>

          <div style={{ display: 'flex', gap: '12px' }}>
            <button onClick={addSlot} style={btnSecondary}>
              + Add Player
            </button>
            <div style={{ flex: 1 }} />
            <button onClick={handleStart} disabled={starting} style={btnStart}>
              {starting ? 'Starting...' : 'Start Game'}
            </button>
          </div>
        </>
      )}
      </div>

      {/* Sliding settings panel on the right */}
      <div style={{
        marginLeft: isRandomMap ? '24px' : '0px',
        flexShrink: 0,
        opacity: isRandomMap ? 1 : 0,
        pointerEvents: isRandomMap ? 'auto' : 'none',
        width: isRandomMap ? '220px' : '0px',
        overflow: isRandomMap ? 'visible' : 'hidden',
        transition: 'width 0.3s ease, opacity 0.25s ease, margin-left 0.3s ease',
      }}>
        <div style={{
          width: '220px',
          background: '#16213e',
          borderRadius: '8px',
          padding: '20px',
          marginTop: '52px',
          border: '1px solid #333',
          boxSizing: 'border-box',
        }}>
          <div style={{ color: '#e0e0e0', fontWeight: 600, fontSize: '14px', marginBottom: '20px' }}>
            Map Settings
          </div>

          <div style={{ marginBottom: '18px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '6px' }}>
              <label style={{ color: '#888', fontSize: '12px' }}>Width</label>
              <span style={{ color: '#f5a623', fontSize: '12px', fontWeight: 600 }}>{mapWidth}</span>
            </div>
            <input type="range" min={20} max={150} value={mapWidth}
              onChange={e => setMapWidth(Number(e.target.value))}
              style={{ width: '100%', accentColor: '#f5a623' }} />
          </div>

          <div style={{ marginBottom: '18px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '6px' }}>
              <label style={{ color: '#888', fontSize: '12px' }}>Height</label>
              <span style={{ color: '#f5a623', fontSize: '12px', fontWeight: 600 }}>{mapHeight}</span>
            </div>
            <input type="range" min={20} max={150} value={mapHeight}
              onChange={e => setMapHeight(Number(e.target.value))}
              style={{ width: '100%', accentColor: '#f5a623' }} />
          </div>

          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '6px' }}>
              <label style={{ color: '#888', fontSize: '12px' }}>Food Patches</label>
              <span style={{ color: '#f5a623', fontSize: '12px', fontWeight: 600 }}>{mapFoodSpots}</span>
            </div>
            <input type="range" min={1} max={200} value={mapFoodSpots}
              onChange={e => setMapFoodSpots(Number(e.target.value))}
              style={{ width: '100%', accentColor: '#f5a623' }} />
          </div>
        </div>
      </div>
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  background: '#0a0a1a',
  border: '1px solid #333',
  borderRadius: '4px',
  color: '#e0e0e0',
  padding: '6px 10px',
  fontSize: '13px',
};

const btnStart: React.CSSProperties = {
  background: '#16c79a',
  color: '#fff',
  border: 'none',
  padding: '10px 32px',
  borderRadius: '6px',
  cursor: 'pointer',
  fontWeight: 700,
  fontSize: '14px',
};

const btnStop: React.CSSProperties = {
  background: '#e94560',
  color: '#fff',
  border: 'none',
  padding: '6px 20px',
  borderRadius: '4px',
  cursor: 'pointer',
  fontWeight: 600,
  fontSize: '13px',
};

const btnSecondary: React.CSSProperties = {
  background: 'transparent',
  color: '#f5a623',
  border: '1px solid #f5a623',
  padding: '8px 20px',
  borderRadius: '6px',
  cursor: 'pointer',
  fontWeight: 600,
  fontSize: '13px',
};

const btnRemove: React.CSSProperties = {
  background: 'transparent',
  color: '#e94560',
  border: '1px solid #e94560',
  borderRadius: '4px',
  padding: '4px 8px',
  cursor: 'pointer',
  fontSize: '12px',
  fontWeight: 700,
};
