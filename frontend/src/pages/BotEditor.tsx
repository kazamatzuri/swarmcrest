import { useEffect, useState, useCallback, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import Editor from '@monaco-editor/react';
import type { editor as monacoEditor } from 'monaco-editor';
import { api } from '../api/client';
import type { Bot, BotVersion } from '../api/client';

const DEFAULT_CODE = `-- Your SwarmCrest bot (high-level API, coroutine style)
-- See Docs for the state machine style alternative

function Creature:onSpawned()
    -- Called when creature is created
end

function Creature:main()
    -- Main creature logic runs each tick
    if self:tile_food() > 0 and self:food() < self:max_food() then
        self:eat()
    elseif self:health() < 80 then
        self:heal()
    else
        local x1, y1, x2, y2 = world_size()
        self:moveto(math.random(x1, x2), math.random(y1, y2))
    end
end
`;

export function BotEditor() {
  const { botId } = useParams<{ botId: string }>();
  const navigate = useNavigate();

  const [bot, setBot] = useState<Bot | null>(null);
  const [versions, setVersions] = useState<BotVersion[]>([]);
  const [currentVersion, setCurrentVersion] = useState<BotVersion | null>(null);
  const [code, setCode] = useState(DEFAULT_CODE);
  const [botName, setBotName] = useState('');
  const [botDescription, setBotDescription] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  const editorRef = useRef<monacoEditor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<typeof import('monaco-editor') | null>(null);
  const validateTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const validateLuaCode = useCallback(async (codeToValidate: string) => {
    const monaco = monacoRef.current;
    const editor = editorRef.current;
    if (!monaco || !editor) return;
    const model = editor.getModel();
    if (!model) return;

    try {
      const result = await api.validateLua(codeToValidate);
      if (!result.valid && result.error) {
        // Parse line number from error like: [string "user_bot"]:5: <unexpected symbol near 'x'>
        const match = result.error.match(/\[string "[^"]+"\]:(\d+): (.+)$/);
        const line = match ? parseInt(match[1], 10) : 1;
        const message = match ? match[2] : result.error;
        monaco.editor.setModelMarkers(model, 'lua', [{
          severity: monaco.MarkerSeverity.Error,
          message,
          startLineNumber: line,
          startColumn: 1,
          endLineNumber: line,
          endColumn: model.getLineLength(line) + 1,
        }]);
      } else {
        monaco.editor.setModelMarkers(model, 'lua', []);
      }
    } catch {
      // Network error — clear markers rather than showing stale errors
      monaco.editor.setModelMarkers(model, 'lua', []);
    }
  }, []);

  const scheduleValidation = useCallback((codeToValidate: string) => {
    if (validateTimerRef.current) clearTimeout(validateTimerRef.current);
    validateTimerRef.current = setTimeout(() => validateLuaCode(codeToValidate), 600);
  }, [validateLuaCode]);

  const loadBot = useCallback(async (id: number) => {
    try {
      setError(null);
      const [botData, versionsData] = await Promise.all([
        api.getBot(id),
        api.listVersions(id),
      ]);
      setBot(botData);
      setBotName(botData.name);
      setBotDescription(botData.description || '');
      setVersions(versionsData);

      if (versionsData.length > 0) {
        const latest = versionsData[versionsData.length - 1];
        setCurrentVersion(latest);
        setCode(latest.code);
        scheduleValidation(latest.code);
      } else {
        setCode(DEFAULT_CODE);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load bot');
    }
  }, []);

  useEffect(() => {
    if (botId) {
      loadBot(parseInt(botId, 10));
    }
  }, [botId, loadBot]);

  // Cleanup validation timer on unmount
  useEffect(() => {
    return () => {
      if (validateTimerRef.current) clearTimeout(validateTimerRef.current);
    };
  }, []);

  const handleSaveVersion = async () => {
    if (!bot) return;
    setSaving(true);
    setError(null);
    setSuccessMsg(null);
    try {
      // Update bot name/description if changed
      if (botName !== bot.name || botDescription !== (bot.description || '')) {
        await api.updateBot(bot.id, botName, botDescription);
        setBot({ ...bot, name: botName, description: botDescription });
      }

      const newVersion = await api.createVersion(bot.id, code);
      setVersions(prev => [...prev, newVersion]);
      setCurrentVersion(newVersion);
      setSuccessMsg(`Saved as version ${newVersion.version}`);
      setTimeout(() => setSuccessMsg(null), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save');
    } finally {
      setSaving(false);
    }
  };

  const handleVersionChange = async (versionId: number) => {
    if (!bot) return;
    try {
      const ver = await api.getVersion(bot.id, versionId);
      setCurrentVersion(ver);
      setCode(ver.code);
      scheduleValidation(ver.code);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load version');
    }
  };

  if (!botId) {
    return (
      <div style={{ padding: '24px', textAlign: 'center', color: '#888' }}>
        <p>Select a bot from the library or create a new one.</p>
        <button onClick={() => navigate('/bots')} style={btnPrimary}>
          Go to Bot Library
        </button>
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Top bar */}
      <div style={{ padding: '12px 24px', background: '#16213e', borderBottom: '1px solid #333', display: 'flex', alignItems: 'center', gap: '16px', flexWrap: 'wrap' }}>
        <input
          value={botName}
          onChange={e => setBotName(e.target.value)}
          placeholder="Bot name"
          style={inputStyle}
        />
        <input
          value={botDescription}
          onChange={e => setBotDescription(e.target.value)}
          placeholder="Description"
          style={{ ...inputStyle, width: '250px' }}
        />
        <select
          value={currentVersion?.id || ''}
          onChange={e => handleVersionChange(parseInt(e.target.value, 10))}
          style={selectStyle}
          disabled={versions.length === 0}
        >
          {versions.length === 0 ? (
            <option value="">No versions</option>
          ) : (
            versions.map(v => (
              <option key={v.id} value={v.id}>
                Version {v.version} - {new Date(v.created_at).toLocaleString()}
              </option>
            ))
          )}
        </select>
        <button onClick={handleSaveVersion} disabled={saving} style={btnPrimary}>
          {saving ? 'Saving...' : 'Save Version'}
        </button>
      </div>

      {/* Messages */}
      {error && (
        <div style={{ padding: '8px 24px', background: '#5c1a1a', color: '#ff8a8a', fontSize: '13px' }}>
          {error}
        </div>
      )}
      {successMsg && (
        <div style={{ padding: '8px 24px', background: '#1a3a1a', color: '#16c79a', fontSize: '13px' }}>
          {successMsg}
        </div>
      )}

      {/* Editor */}
      <div style={{ flex: 1, minHeight: 0 }}>
        <Editor
          height="100%"
          defaultLanguage="lua"
          theme="vs-dark"
          value={code}
          onChange={value => {
            const newCode = value || '';
            setCode(newCode);
            scheduleValidation(newCode);
          }}
          onMount={(editor, monaco) => {
            editorRef.current = editor;
            monacoRef.current = monaco;
          }}
          options={{
            fontSize: 14,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            padding: { top: 12 },
            lineNumbers: 'on',
            renderLineHighlight: 'line',
            tabSize: 4,
            insertSpaces: true,
          }}
        />
      </div>
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  background: '#0a0a1a',
  color: '#e0e0e0',
  border: '1px solid #333',
  borderRadius: '4px',
  padding: '6px 12px',
  fontSize: '14px',
  width: '180px',
};

const selectStyle: React.CSSProperties = {
  background: '#0a0a1a',
  color: '#e0e0e0',
  border: '1px solid #333',
  borderRadius: '4px',
  padding: '6px 12px',
  fontSize: '14px',
};

const btnPrimary: React.CSSProperties = {
  background: '#16c79a',
  color: '#fff',
  border: 'none',
  padding: '8px 20px',
  borderRadius: '4px',
  cursor: 'pointer',
  fontWeight: 600,
  fontSize: '14px',
};
