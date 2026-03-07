const BASE_URL = '';

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('swarmcrest_token');
  if (token) {
    return { Authorization: `Bearer ${token}` };
  }
  return {};
}

export interface Bot {
  id: number;
  name: string;
  description: string;
  owner_id: number | null;
  owner_username?: string | null;
  visibility: string;
  active_version_id?: number | null;
  created_at: string;
  updated_at: string;
  version_count?: number;
  latest_version?: number | null;
  latest_elo_1v1?: number | null;
}

export interface BotVersion {
  id: number;
  bot_id: number;
  version: number;
  code: string;
  is_archived: boolean;
  is_faulty: boolean;
  elo_rating: number;
  elo_1v1: number;
  elo_peak: number;
  games_played: number;
  wins: number;
  losses: number;
  draws: number;
  ffa_placement_points: number;
  ffa_games: number;
  creatures_spawned: number;
  creatures_killed: number;
  creatures_lost: number;
  total_score: number;
  created_at: string;
}

export interface Tournament {
  id: number;
  name: string;
  status: string;
  map: string;
  config: string;
  format: string;
  current_round: number;
  total_rounds: number;
  created_at: string;
}

export interface TournamentEntry {
  id: number;
  tournament_id: number;
  bot_version_id: number;
  slot_name: string;
  bot_name?: string;
  version?: number;
}

export interface TournamentStanding {
  bot_version_id: number;
  bot_name: string;
  total_score: number;
  matches_played: number;
  wins: number;
  losses: number;
}

export interface LeaderboardEntry {
  rank: number;
  bot_version_id: number;
  bot_name: string;
  version: number;
  owner_username: string;
  rating: number;
  games_played: number;
  wins: number;
  losses: number;
  win_rate: number;
}

export interface MapInfo {
  name: string;
  width: number;
  height: number;
  description: string;
}

export interface ApiKey {
  id: number;
  name: string;
  scopes: string;
  last_used_at: string | null;
  created_at: string;
}

export interface ChallengeResult {
  match_id?: number;
  id?: number;
  status?: string;
}

export interface ActiveGameInfo {
  match_id: number | null;
  player_names: string[];
  format: string;
  map: string;
  start_time: string;
  spectator_count: number;
  game_time_seconds: number;
}

export interface Notification {
  id: number;
  user_id: number;
  type: string;
  title: string;
  message: string;
  data: string | null;
  read: boolean;
  created_at: string;
}

export interface NotificationsResponse {
  notifications: Notification[];
  unread_count: number;
}

export interface MatchDetail {
  match: {
    id: number;
    format: string;
    map: string;
    status: string;
    winner_bot_version_id: number | null;
    created_at: string;
    finished_at: string | null;
  };
  participants: {
    id: number;
    match_id: number;
    bot_version_id: number;
    player_slot: number;
    final_score: number;
    placement: number | null;
    elo_before: number | null;
    elo_after: number | null;
    creatures_spawned: number;
    creatures_killed: number;
    creatures_lost: number;
    bot_name: string | null;
    owner_name: string | null;
  }[];
}

export interface ReplayData {
  match_id: number;
  tick_count: number;
  messages: GameMessage[];
}

export interface Team {
  id: number;
  owner_id: number;
  name: string;
  created_at: string;
}

export interface TeamVersion {
  id: number;
  team_id: number;
  version: number;
  bot_version_a: number;
  bot_version_b: number;
  elo_rating: number;
  games_played: number;
  wins: number;
  losses: number;
  draws: number;
  created_at: string;
}

export interface TournamentResult {
  id: number;
  tournament_id: number;
  player_slot: number;
  bot_version_id: number;
  final_score: number;
  creatures_spawned: number;
  creatures_killed: number;
  creatures_lost: number;
  finished_at: string;
}

export interface TournamentMatchParticipant {
  bot_version_id: number;
  player_slot: number;
  final_score: number;
  bot_name: string | null;
  owner_name: string | null;
}

export interface TournamentMatchInfo {
  match_id: number;
  round: number;
  status: string;
  winner_bot_version_id: number | null;
  finished_at: string | null;
  participants: TournamentMatchParticipant[];
}

export interface TournamentRound {
  round: number;
  matches: TournamentMatchInfo[];
}

export interface TournamentMatchesResponse {
  rounds: TournamentRound[];
}

// Broadcast events for event ticker
export type BroadcastEvent =
  | { kind: 'Spawn'; creature_id: number; player_id: number; player_name: string; creature_type: number }
  | { kind: 'Kill'; creature_id: number; player_id: number; player_name: string; killer_player_id?: number; killer_player_name?: string; starvation: boolean }
  | { kind: 'PlayerJoined'; player_id: number; player_name: string };

// WebSocket message types
export interface WorldMsg {
  type: 'world';
  width: number;
  height: number;
  koth_x: number;
  koth_y: number;
  tiles: TileSnapshot[];
}

export interface SnapshotMsg {
  type: 'snapshot';
  game_time: number;
  creatures: CreatureSnapshot[];
  players: PlayerSnapshot[];
  king_player_id?: number;
  events?: BroadcastEvent[];
}

export interface PlayerEndStats {
  player_id: number;
  creatures_spawned: number;
  creatures_killed: number;
  creatures_lost: number;
}

export interface GameEndMsg {
  type: 'game_end';
  winner?: number;
  final_scores: PlayerSnapshot[];
  match_id?: number;
  player_stats?: PlayerEndStats[];
  game_duration_ticks?: number;
}

export interface PlayerLoadErrorMsg {
  type: 'player_load_error';
  player_name: string;
  error: string;
}

export interface ValidateLuaResult {
  valid: boolean;
  error?: string;
}

export interface SnapshotDeltaMsg {
  type: 'snapshot_delta';
  game_time: number;
  changed: CreatureSnapshot[];
  removed: number[];
  players: PlayerSnapshot[];
  king_player_id?: number;
  events?: BroadcastEvent[];
}

export interface Feedback {
  id: number;
  user_id: number | null;
  category: string;
  description: string;
  created_at: string;
}

export type GameMessage = WorldMsg | SnapshotMsg | SnapshotDeltaMsg | GameEndMsg | PlayerLoadErrorMsg;

export interface TileSnapshot {
  x: number;
  y: number;
  food: number;
  tile_type: number;
  gfx: number;
}

export interface CreatureSnapshot {
  id: number;
  x: number;
  y: number;
  creature_type: number;
  state: number;
  health: number;
  max_health: number;
  food: number;
  player_id: number;
  message: string;
  target_id?: number;
}

export interface PlayerSnapshot {
  id: number;
  name: string;
  score: number;
  color: number;
  num_creatures: number;
  output?: string[];
}

async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    if (response.status === 401) {
      throw new Error('Please log in to access this content.');
    }
    const text = await response.text().catch(() => 'Unknown error');
    throw new Error(`API error ${response.status}: ${text}`);
  }
  return response.json();
}

export const api = {
  // Bots
  listBots: (): Promise<Bot[]> =>
    fetch(`${BASE_URL}/api/bots`, { headers: authHeaders() }).then(r => handleResponse<Bot[]>(r)),

  createBot: (name: string, description?: string): Promise<Bot> =>
    fetch(`${BASE_URL}/api/bots`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ name, description }),
    }).then(r => handleResponse<Bot>(r)),

  getBot: (id: number): Promise<Bot> =>
    fetch(`${BASE_URL}/api/bots/${id}`, { headers: authHeaders() }).then(r => handleResponse<Bot>(r)),

  updateBot: (id: number, name: string, description?: string): Promise<Bot> =>
    fetch(`${BASE_URL}/api/bots/${id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ name, description }),
    }).then(r => handleResponse<Bot>(r)),

  deleteBot: (id: number): Promise<void> =>
    fetch(`${BASE_URL}/api/bots/${id}`, { method: 'DELETE', headers: authHeaders() }).then(r => {
      if (!r.ok) throw new Error(`Delete failed: ${r.status}`);
    }),

  // Versions
  listVersions: (botId: number): Promise<BotVersion[]> =>
    fetch(`${BASE_URL}/api/bots/${botId}/versions`, { headers: authHeaders() }).then(r => handleResponse<BotVersion[]>(r)),

  createVersion: (botId: number, code: string): Promise<BotVersion> =>
    fetch(`${BASE_URL}/api/bots/${botId}/versions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ code }),
    }).then(r => handleResponse<BotVersion>(r)),

  getVersion: (botId: number, versionId: number): Promise<BotVersion> =>
    fetch(`${BASE_URL}/api/bots/${botId}/versions/${versionId}`, { headers: authHeaders() }).then(r => handleResponse<BotVersion>(r)),

  // Tournaments
  listTournaments: (): Promise<Tournament[]> =>
    fetch(`${BASE_URL}/api/tournaments`, { headers: authHeaders() }).then(r => handleResponse<Tournament[]>(r)),

  createTournament: (name: string, map?: string): Promise<Tournament> =>
    fetch(`${BASE_URL}/api/tournaments`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ name, map }),
    }).then(r => handleResponse<Tournament>(r)),

  getTournament: (id: number): Promise<Tournament> =>
    fetch(`${BASE_URL}/api/tournaments/${id}`, { headers: authHeaders() }).then(r => handleResponse<Tournament>(r)),

  updateTournament: (id: number, data: { name?: string; map?: string; format?: string; config?: string }): Promise<Tournament> =>
    fetch(`${BASE_URL}/api/tournaments/${id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify(data),
    }).then(r => handleResponse<Tournament>(r)),

  getStandings: (tournamentId: number): Promise<TournamentStanding[]> =>
    fetch(`${BASE_URL}/api/tournaments/${tournamentId}/standings`, { headers: authHeaders() }).then(r => handleResponse<TournamentStanding[]>(r)),

  listEntries: (tournamentId: number): Promise<TournamentEntry[]> =>
    fetch(`${BASE_URL}/api/tournaments/${tournamentId}/entries`, { headers: authHeaders() }).then(r => handleResponse<TournamentEntry[]>(r)),

  addEntry: (tournamentId: number, botVersionId: number, slotName?: string): Promise<TournamentEntry> =>
    fetch(`${BASE_URL}/api/tournaments/${tournamentId}/entries`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ bot_version_id: botVersionId, slot_name: slotName }),
    }).then(r => handleResponse<TournamentEntry>(r)),

  removeEntry: (tournamentId: number, entryId: number): Promise<void> =>
    fetch(`${BASE_URL}/api/tournaments/${tournamentId}/entries/${entryId}`, {
      method: 'DELETE',
      headers: authHeaders(),
    }).then(r => {
      if (!r.ok) throw new Error(`Delete failed: ${r.status}`);
    }),

  runTournament: (id: number): Promise<void> =>
    fetch(`${BASE_URL}/api/tournaments/${id}/run`, { method: 'POST', headers: authHeaders() }).then(r => {
      if (!r.ok) throw new Error(`Run failed: ${r.status}`);
    }),

  getResults: (tournamentId: number): Promise<TournamentResult[]> =>
    fetch(`${BASE_URL}/api/tournaments/${tournamentId}/results`, { headers: authHeaders() }).then(r => handleResponse<TournamentResult[]>(r)),

  getTournamentMatches: (tournamentId: number): Promise<TournamentMatchesResponse> =>
    fetch(`${BASE_URL}/api/tournaments/${tournamentId}/matches`, { headers: authHeaders() }).then(r => handleResponse<TournamentMatchesResponse>(r)),

  // Game
  gameStatus: (): Promise<{ running: boolean }> =>
    fetch(`${BASE_URL}/api/game/status`).then(r => handleResponse<{ running: boolean }>(r)),

  listMaps: (): Promise<MapInfo[]> =>
    fetch(`${BASE_URL}/api/maps`).then(r => handleResponse<MapInfo[]>(r)),

  startGame: (players: { bot_version_id: number; name?: string }[], map?: string, headless?: boolean, map_params?: { width?: number; height?: number; num_food_spots?: number }): Promise<{ status: string; message: string; match_id?: number }> =>
    fetch(`${BASE_URL}/api/game/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ players, map, headless, map_params }),
    }).then(r => handleResponse<{ status: string; message: string; match_id?: number }>(r)),

  stopGame: (): Promise<void> =>
    fetch(`${BASE_URL}/api/game/stop`, { method: 'POST', headers: authHeaders() }).then(r => {
      if (!r.ok) throw new Error(`Stop failed: ${r.status}`);
    }),

  // Leaderboards
  leaderboard1v1: (limit = 50, offset = 0): Promise<LeaderboardEntry[]> =>
    fetch(`${BASE_URL}/api/leaderboards/1v1?limit=${limit}&offset=${offset}`, { headers: authHeaders() })
      .then(r => handleResponse<LeaderboardEntry[]>(r)),

  leaderboardFfa: (limit = 50, offset = 0): Promise<LeaderboardEntry[]> =>
    fetch(`${BASE_URL}/api/leaderboards/ffa?limit=${limit}&offset=${offset}`, { headers: authHeaders() })
      .then(r => handleResponse<LeaderboardEntry[]>(r)),

  leaderboard2v2: (limit = 50, offset = 0): Promise<LeaderboardEntry[]> =>
    fetch(`${BASE_URL}/api/leaderboards/2v2?limit=${limit}&offset=${offset}`, { headers: authHeaders() })
      .then(r => handleResponse<LeaderboardEntry[]>(r)),

  // Match detail & replay
  getMatch: (id: number): Promise<MatchDetail> =>
    fetch(`${BASE_URL}/api/matches/${id}`, { headers: authHeaders() }).then(r => handleResponse<MatchDetail>(r)),

  getReplay: (matchId: number): Promise<ReplayData> =>
    fetch(`${BASE_URL}/api/matches/${matchId}/replay`, { headers: authHeaders() }).then(r => handleResponse<ReplayData>(r)),

  // Match listing
  listMatches: (opts?: { limit?: number; offset?: number; bot_id?: number; user_id?: number; username?: string; sort?: 'newest' | 'oldest'; status?: string; map?: string }): Promise<(MatchDetail['match'] & { players?: string[] })[]> => {
    const params = new URLSearchParams();
    if (opts?.limit != null) params.set('limit', String(opts.limit));
    if (opts?.offset != null) params.set('offset', String(opts.offset));
    if (opts?.bot_id != null) params.set('bot_id', String(opts.bot_id));
    if (opts?.user_id != null) params.set('user_id', String(opts.user_id));
    if (opts?.username) params.set('username', opts.username);
    if (opts?.sort) params.set('sort', opts.sort);
    if (opts?.status) params.set('status', opts.status);
    if (opts?.map) params.set('map', opts.map);
    const qs = params.toString();
    return fetch(`${BASE_URL}/api/matches${qs ? '?' + qs : ''}`, { headers: authHeaders() }).then(r => handleResponse<(MatchDetail['match'] & { players?: string[] })[]>(r));
  },

  listMyMatches: (limit = 50, offset = 0): Promise<MatchDetail['match'][]> =>
    fetch(`${BASE_URL}/api/matches/mine?limit=${limit}&offset=${offset}`, { headers: authHeaders() }).then(r => handleResponse<MatchDetail['match'][]>(r)),

  // Teams
  listTeams: (): Promise<Team[]> =>
    fetch(`${BASE_URL}/api/teams`, { headers: authHeaders() }).then(r => handleResponse<Team[]>(r)),

  createTeam: (name: string): Promise<Team> =>
    fetch(`${BASE_URL}/api/teams`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ name }),
    }).then(r => handleResponse<Team>(r)),

  getTeam: (id: number): Promise<Team> =>
    fetch(`${BASE_URL}/api/teams/${id}`, { headers: authHeaders() }).then(r => handleResponse<Team>(r)),

  deleteTeam: (id: number): Promise<void> =>
    fetch(`${BASE_URL}/api/teams/${id}`, { method: 'DELETE', headers: authHeaders() }).then(r => {
      if (!r.ok) throw new Error(`Delete failed: ${r.status}`);
    }),

  listTeamVersions: (teamId: number): Promise<TeamVersion[]> =>
    fetch(`${BASE_URL}/api/teams/${teamId}/versions`, { headers: authHeaders() }).then(r => handleResponse<TeamVersion[]>(r)),

  createTeamVersion: (teamId: number, botVersionA: number, botVersionB: number): Promise<TeamVersion> =>
    fetch(`${BASE_URL}/api/teams/${teamId}/versions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ bot_version_a: botVersionA, bot_version_b: botVersionB }),
    }).then(r => handleResponse<TeamVersion>(r)),

  // API Keys
  listApiKeys: (): Promise<ApiKey[]> =>
    fetch(`${BASE_URL}/api/api-keys`, { headers: authHeaders() }).then(r => handleResponse<ApiKey[]>(r)),

  createApiKey: (name: string, scopes?: string): Promise<ApiKey & { token: string }> =>
    fetch(`${BASE_URL}/api/api-keys`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ name, scopes }),
    }).then(r => handleResponse<ApiKey & { token: string }>(r)),

  deleteApiKey: (id: number): Promise<void> =>
    fetch(`${BASE_URL}/api/api-keys/${id}`, {
      method: 'DELETE',
      headers: authHeaders(),
    }).then(r => {
      if (!r.ok) throw new Error(`Delete failed: ${r.status}`);
    }),

  // Lua validation
  validateLua: (code: string): Promise<ValidateLuaResult> =>
    fetch(`${BASE_URL}/api/validate-lua`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ code }),
    }).then(r => handleResponse<ValidateLuaResult>(r)),

  // Challenges
  createChallenge: (botVersionId: number, opponentBotVersionId: number, options?: { format?: string; headless?: boolean; map?: string }): Promise<ChallengeResult> =>
    fetch(`${BASE_URL}/api/matches/challenge`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({
        bot_version_id: botVersionId,
        opponent_bot_version_id: opponentBotVersionId,
        format: options?.format,
        headless: options?.headless,
        map: options?.map,
      }),
    }).then(r => handleResponse<ChallengeResult>(r)),

  // Active games
  listActiveGames: (): Promise<ActiveGameInfo[]> =>
    fetch(`${BASE_URL}/api/games/active`).then(r => handleResponse<ActiveGameInfo[]>(r)),

  // Notifications
  listNotifications: (): Promise<NotificationsResponse> =>
    fetch(`${BASE_URL}/api/notifications`, { headers: authHeaders() }).then(r => handleResponse<NotificationsResponse>(r)),

  markNotificationRead: (id: number): Promise<void> =>
    fetch(`${BASE_URL}/api/notifications/${id}/read`, {
      method: 'POST',
      headers: authHeaders(),
    }).then(r => {
      if (!r.ok) throw new Error(`Mark read failed: ${r.status}`);
    }),

  // Feedback
  submitFeedback: (category: string, description: string): Promise<Feedback> =>
    fetch(`${BASE_URL}/api/feedback`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ category, description }),
    }).then(r => handleResponse<Feedback>(r)),

  listFeedback: (): Promise<Feedback[]> =>
    fetch(`${BASE_URL}/api/feedback`, { headers: authHeaders() }).then(r => handleResponse<Feedback[]>(r)),
};
