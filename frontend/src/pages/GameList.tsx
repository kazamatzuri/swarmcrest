import { useEffect, useState, useCallback, useRef } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { api } from '../api/client';
import type { ActiveGameInfo, Bot, MapInfo, MatchDetail } from '../api/client';

type MatchWithPlayers = MatchDetail['match'] & { players?: string[] };

interface MatchFilters {
  bot_id?: number;
  username?: string;
  status?: string;
  map?: string;
  sort: 'newest' | 'oldest';
}

const PAGE_SIZE = 20;

function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}

function timeAgo(dateStr: string): string {
  const now = Date.now();
  // Backend stores UTC timestamps without timezone suffix; append 'Z' so JS parses as UTC
  const utcStr = dateStr.endsWith('Z') || dateStr.includes('+') ? dateStr : dateStr + 'Z';
  const then = new Date(utcStr).getTime();
  const diffSec = Math.floor((now - then) / 1000);
  if (diffSec < 60) return 'just now';
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
  return `${Math.floor(diffSec / 86400)}d ago`;
}

function friendlyMap(map: string): string {
  if (map === 'random') return 'Random Generated';
  if (map === 'random_pool') return 'Random from Pool';
  return map;
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, { bg: string; fg: string }> = {
    finished: { bg: '#16c79a22', fg: '#16c79a' },
    running: { bg: '#f5a62322', fg: '#f5a623' },
    pending: { bg: '#f5a62322', fg: '#f5a623' },
    queued: { bg: '#f5a62322', fg: '#f5a623' },
    abandoned: { bg: '#e9456022', fg: '#e94560' },
  };
  const c = colors[status] || colors.abandoned;
  return (
    <span style={{
      background: c.bg,
      color: c.fg,
      padding: '2px 8px',
      borderRadius: '10px',
      fontSize: '11px',
      fontWeight: 600,
      textTransform: 'capitalize',
    }}>
      {status}
    </span>
  );
}

export function GameList() {
  const navigate = useNavigate();
  const [activeGames, setActiveGames] = useState<ActiveGameInfo[]>([]);
  const [recentMatches, setRecentMatches] = useState<MatchWithPlayers[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [allBots, setAllBots] = useState<Bot[]>([]);
  const [maps, setMaps] = useState<MapInfo[]>([]);
  const [filters, setFilters] = useState<MatchFilters>({ sort: 'newest' });
  const [usernameInput, setUsernameInput] = useState('');
  const filtersRef = useRef(filters);
  filtersRef.current = filters;

  useEffect(() => {
    // Fetch all bots for the filter dropdown (with ?all=true)
    fetch('/api/bots?all=true', { headers: { Authorization: `Bearer ${localStorage.getItem('swarmcrest_token') || ''}` } })
      .then(r => r.ok ? r.json() : [])
      .then(setAllBots)
      .catch(() => {});
    // Fetch maps for map filter dropdown
    api.listMaps().then(setMaps).catch(() => {});
  }, []);

  const buildOpts = useCallback((f: MatchFilters, offset: number) => {
    const opts: Parameters<typeof api.listMatches>[0] = { limit: PAGE_SIZE, offset };
    if (f.bot_id) opts.bot_id = f.bot_id;
    if (f.username) opts.username = f.username;
    if (f.status) opts.status = f.status;
    if (f.map) opts.map = f.map;
    opts.sort = f.sort;
    return opts;
  }, []);

  const loadData = useCallback(async () => {
    try {
      setError(null);
      const f = filtersRef.current;
      const [active, recent] = await Promise.all([
        api.listActiveGames(),
        api.listMatches(buildOpts(f, 0)),
      ]);
      setActiveGames(active);
      setRecentMatches(recent);
      setHasMore(recent.length === PAGE_SIZE);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load games');
    } finally {
      setLoading(false);
    }
  }, [buildOpts]);

  const loadMore = async () => {
    setLoadingMore(true);
    try {
      const more = await api.listMatches(buildOpts(filters, recentMatches.length));
      setRecentMatches(prev => [...prev, ...more]);
      setHasMore(more.length === PAGE_SIZE);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load more');
    } finally {
      setLoadingMore(false);
    }
  };

  // Re-fetch when filters change
  useEffect(() => {
    setLoading(true);
    setRecentMatches([]);
    loadData();
  }, [filters, loadData]);

  // Poll for active games
  useEffect(() => {
    const interval = setInterval(loadData, 5000);
    return () => clearInterval(interval);
  }, [loadData]);

  const updateFilter = (patch: Partial<MatchFilters>) => {
    setFilters(prev => ({ ...prev, ...patch }));
  };

  const applyUsername = () => {
    const trimmed = usernameInput.trim();
    updateFilter({ username: trimmed || undefined });
  };

  return (
    <div style={{ padding: '24px', maxWidth: '960px', margin: '0 auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '24px' }}>
        <h2 style={{ color: '#e0e0e0', margin: 0 }}>Games</h2>
        <Link
          to="/game"
          style={{
            padding: '8px 20px',
            background: '#16c79a',
            color: '#fff',
            borderRadius: '4px',
            textDecoration: 'none',
            fontWeight: 600,
            fontSize: '14px',
          }}
        >
          New Game
        </Link>
      </div>

      {error && (
        <div style={{
          padding: '12px',
          background: '#5c1a1a',
          border: '1px solid #e94560',
          borderRadius: '4px',
          marginBottom: '16px',
          color: '#ff8a8a',
        }}>
          {error}
        </div>
      )}

      {/* Active Games */}
      <h3 style={{ color: '#16c79a', marginBottom: '12px', fontSize: '16px' }}>
        Live Games {activeGames.length > 0 && (
          <span style={{
            background: '#16c79a',
            color: '#0a0a1a',
            borderRadius: '10px',
            padding: '2px 8px',
            fontSize: '12px',
            fontWeight: 700,
            marginLeft: '8px',
          }}>
            {activeGames.length}
          </span>
        )}
      </h3>

      {activeGames.length === 0 ? (
        <div style={{
          padding: '32px',
          background: '#16213e',
          borderRadius: '8px',
          textAlign: 'center',
          color: '#666',
          marginBottom: '32px',
        }}>
          No games currently running
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', marginBottom: '32px' }}>
          {activeGames.map((game, i) => (
            <div
              key={game.match_id ?? i}
              style={{
                padding: '16px 20px',
                background: '#16213e',
                borderRadius: '8px',
                border: '1px solid #16c79a33',
                display: 'flex',
                alignItems: 'center',
                gap: '16px',
              }}
            >
              {/* Live indicator */}
              <div style={{
                width: '10px',
                height: '10px',
                borderRadius: '50%',
                background: '#16c79a',
                boxShadow: '0 0 6px #16c79a',
                flexShrink: 0,
              }} />

              {/* Players */}
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ color: '#e0e0e0', fontWeight: 600, fontSize: '15px', marginBottom: '4px' }}>
                  {game.player_names.join(' vs ')}
                </div>
                <div style={{ color: '#888', fontSize: '13px', display: 'flex', gap: '16px', flexWrap: 'wrap' }}>
                  <span>{game.format.toUpperCase()}</span>
                  <span>Map: {friendlyMap(game.map)}</span>
                  <span>Duration: {formatDuration(game.game_time_seconds)}</span>
                  <span style={{ color: '#f5a623' }}>
                    {game.spectator_count} {game.spectator_count === 1 ? 'spectator' : 'spectators'}
                  </span>
                </div>
              </div>

              {/* Watch button */}
              <button
                onClick={() => navigate('/game')}
                style={{
                  padding: '8px 20px',
                  borderRadius: '4px',
                  border: 'none',
                  cursor: 'pointer',
                  fontWeight: 600,
                  fontSize: '14px',
                  background: '#16c79a',
                  color: '#fff',
                  flexShrink: 0,
                }}
              >
                Watch
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Recent Matches */}
      <h3 style={{ color: '#e0e0e0', marginBottom: '12px', fontSize: '16px' }}>Recent Matches</h3>

      {/* Filter bar */}
      <div style={{
        display: 'flex',
        gap: '12px',
        marginBottom: '16px',
        flexWrap: 'wrap',
        alignItems: 'center',
      }}>
        {/* Bot filter */}
        <select
          value={filters.bot_id ?? ''}
          onChange={e => updateFilter({ bot_id: e.target.value ? Number(e.target.value) : undefined })}
          style={{
            padding: '6px 10px',
            background: '#16213e',
            color: '#e0e0e0',
            border: '1px solid #333',
            borderRadius: '4px',
            fontSize: '13px',
          }}
        >
          <option value="">All Bots</option>
          {allBots.map(b => (
            <option key={b.id} value={b.id}>{b.name}</option>
          ))}
        </select>

        {/* Username filter */}
        <input
          type="text"
          placeholder="Username"
          value={usernameInput}
          onChange={e => setUsernameInput(e.target.value)}
          onBlur={applyUsername}
          onKeyDown={e => { if (e.key === 'Enter') applyUsername(); }}
          style={{
            padding: '6px 10px',
            background: '#16213e',
            color: '#e0e0e0',
            border: '1px solid #333',
            borderRadius: '4px',
            fontSize: '13px',
            width: '130px',
          }}
        />

        {/* Status filter */}
        <select
          value={filters.status ?? ''}
          onChange={e => updateFilter({ status: e.target.value || undefined })}
          style={{
            padding: '6px 10px',
            background: '#16213e',
            color: '#e0e0e0',
            border: '1px solid #333',
            borderRadius: '4px',
            fontSize: '13px',
          }}
        >
          <option value="">All Status</option>
          <option value="finished">Finished</option>
          <option value="running">Running</option>
          <option value="pending">Pending</option>
          <option value="abandoned">Abandoned</option>
        </select>

        {/* Map filter */}
        <select
          value={filters.map ?? ''}
          onChange={e => updateFilter({ map: e.target.value || undefined })}
          style={{
            padding: '6px 10px',
            background: '#16213e',
            color: '#e0e0e0',
            border: '1px solid #333',
            borderRadius: '4px',
            fontSize: '13px',
          }}
        >
          <option value="">All Maps</option>
          <option value="random">Random Generated</option>
          {maps.map(m => (
            <option key={m.name} value={m.name}>{m.name}</option>
          ))}
        </select>

        {/* Sort toggle */}
        <button
          onClick={() => updateFilter({ sort: filters.sort === 'newest' ? 'oldest' : 'newest' })}
          style={{
            padding: '6px 14px',
            background: 'transparent',
            color: '#888',
            border: '1px solid #333',
            borderRadius: '4px',
            cursor: 'pointer',
            fontSize: '13px',
          }}
        >
          {filters.sort === 'newest' ? 'Newest first' : 'Oldest first'}
        </button>
      </div>

      {loading && recentMatches.length === 0 ? (
        <div style={{ color: '#888', padding: '16px' }}>Loading...</div>
      ) : recentMatches.length === 0 ? (
        <div style={{
          padding: '32px',
          background: '#16213e',
          borderRadius: '8px',
          textAlign: 'center',
          color: '#666',
        }}>
          No matches yet
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {recentMatches.map(m => (
            <div
              key={m.id}
              onClick={() => navigate(`/matches/${m.id}`)}
              style={{
                padding: '12px 16px',
                background: '#16213e',
                borderRadius: '8px',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '12px',
                transition: 'background 0.15s',
              }}
              onMouseOver={e => (e.currentTarget.style.background = '#1a1a3e')}
              onMouseOut={e => (e.currentTarget.style.background = '#16213e')}
            >
              {/* Match ID */}
              <span style={{ color: '#555', fontSize: '12px', fontFamily: 'monospace', width: '40px', flexShrink: 0 }}>
                #{m.id}
              </span>

              {/* Player names - primary content */}
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ color: '#e0e0e0', fontWeight: 600, fontSize: '14px', marginBottom: '2px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {m.players && m.players.length > 0
                    ? m.players.join(' vs ')
                    : <span style={{ color: '#666', fontStyle: 'italic' }}>Unknown players</span>
                  }
                </div>
                <div style={{ display: 'flex', gap: '10px', alignItems: 'center', flexWrap: 'wrap' }}>
                  <span style={{
                    color: '#888',
                    fontSize: '11px',
                    background: '#0a0a1a',
                    padding: '1px 6px',
                    borderRadius: '3px',
                  }}>
                    {m.format.toUpperCase()}
                  </span>
                  <span style={{ color: '#666', fontSize: '12px' }}>
                    {friendlyMap(m.map)}
                  </span>
                </div>
              </div>

              {/* Status badge */}
              <StatusBadge status={m.status} />

              {/* Time */}
              <span style={{ color: '#666', fontSize: '12px', flexShrink: 0, width: '55px', textAlign: 'right' }}>
                {timeAgo(m.finished_at ?? m.created_at)}
              </span>
            </div>
          ))}

          {/* Load More */}
          {hasMore && (
            <div style={{ textAlign: 'center', padding: '12px' }}>
              <button
                onClick={loadMore}
                disabled={loadingMore}
                style={{
                  padding: '8px 24px',
                  background: 'transparent',
                  color: '#f5a623',
                  border: '1px solid #f5a623',
                  borderRadius: '4px',
                  cursor: loadingMore ? 'default' : 'pointer',
                  fontSize: '13px',
                  fontWeight: 600,
                  opacity: loadingMore ? 0.6 : 1,
                }}
              >
                {loadingMore ? 'Loading...' : 'Load More'}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
