import React, { useState, useEffect, useCallback } from 'react';
import type { AppConfig } from '../types';
import { api } from '../api';

// Small labeled input row
const Field: React.FC<{
  label: string;
  hint?: string;
  restart?: boolean;
  children: React.ReactNode;
}> = ({ label, hint, restart, children }) => (
  <label className="block">
    <span className="flex items-center gap-1.5 text-xs text-[var(--text-secondary)] mb-1">
      {label}
      {restart && (
        <span className="text-[10px] px-1 py-px rounded bg-[var(--bg)] border border-[var(--border)]" title="重启应用后生效">
          重启生效
        </span>
      )}
      {hint && <span className="opacity-60">{hint}</span>}
    </span>
    {children}
  </label>
);

const inputCls =
  'w-full px-2.5 py-1.5 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-lg text-sm text-[var(--text)] outline-none focus:ring-1 focus:ring-[var(--accent)]';

const Section: React.FC<{ icon: string; title: string; children: React.ReactNode }> = ({
  icon, title, children,
}) => (
  <div className="border border-[var(--border)] rounded-xl p-3 space-y-2.5">
    <div className="text-xs font-semibold text-[var(--text)]">
      {icon} {title}
    </div>
    {children}
  </div>
);

interface SettingsPanelProps {
  onMessage: (msg: string) => void;
}

export const SettingsPanel: React.FC<SettingsPanelProps> = ({ onMessage }) => {
  const [expanded, setExpanded] = useState(false);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loadError, setLoadError] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError('');
    try {
      setConfig(await api.getConfig());
    } catch (e) {
      setLoadError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (expanded && !config && !loading) load();
  }, [expanded, config, loading, load]);

  // Patch a nested section of the config immutably
  const patch = <K extends keyof AppConfig>(section: K, partial: Partial<AppConfig[K]>) => {
    setConfig((prev) =>
      prev ? { ...prev, [section]: { ...prev[section], ...partial } } : prev
    );
  };

  const handleSave = async () => {
    if (!config || saving) return;
    setSaving(true);
    try {
      const res = await api.saveConfig(config);
      onMessage(res.success ? `✅ ${res.message}` : `⚠️ ${res.message}`);
    } catch (e) {
      onMessage(`⚠️ 保存失败: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const listToText = (list: string[]) => list.join(', ');
  const textToList = (text: string) =>
    text.split(/[,\n]/).map((s) => s.trim()).filter(Boolean);

  return (
    <div>
      <button
        onClick={() => setExpanded(!expanded)}
        className="text-xs font-semibold text-[var(--text-secondary)] hover:text-[var(--accent)] transition-colors"
      >
        {expanded ? '▾' : '▸'} ⚙️ 配置选项(服务 / 索引 / 解析 / 监控 / 日志, 保存到 anywords.yml)
      </button>

      {expanded && (
        <div className="mt-2 space-y-3">
          {loading && (
            <p className="text-xs text-[var(--text-secondary)] px-1 py-2">加载配置中...</p>
          )}
          {loadError && (
            <p className="text-xs text-[var(--danger)] px-1 py-2">
              配置加载失败: {loadError}
              <button onClick={load} className="ml-2 text-[var(--accent)] hover:underline">重试</button>
            </p>
          )}

          {config && (
            <>
              {/* ── Server ── */}
              <Section icon="🌐" title="服务">
                <div className="grid grid-cols-2 gap-2.5">
                  <Field label="端口" restart>
                    <input
                      type="number" min={1} max={65535} className={inputCls}
                      value={config.server.port}
                      onChange={(e) => patch('server', { port: Number(e.target.value) })}
                    />
                  </Field>
                  <Field label="监听地址" restart>
                    <input
                      type="text" className={inputCls}
                      value={config.server.host}
                      onChange={(e) => patch('server', { host: e.target.value })}
                    />
                  </Field>
                </div>
              </Section>

              {/* ── Index ── */}
              <Section icon="🗄" title="索引">
                <Field label="索引存储目录" restart>
                  <input
                    type="text" className={inputCls}
                    value={config.index.dir}
                    onChange={(e) => patch('index', { dir: e.target.value })}
                  />
                </Field>
                <div className="grid grid-cols-2 gap-2.5">
                  <Field label="单文件大小上限 (MB)" hint="超过则跳过">
                    <input
                      type="number" min={1} className={inputCls}
                      value={Math.round(config.index.max_file_size_bytes / 1024 / 1024)}
                      onChange={(e) =>
                        patch('index', { max_file_size_bytes: Number(e.target.value) * 1024 * 1024 })
                      }
                    />
                  </Field>
                  <Field label="写入缓冲区 (MB)" restart>
                    <input
                      type="number" min={10} className={inputCls}
                      value={Math.round(config.index.writer_buffer_bytes / 1000 / 1000)}
                      onChange={(e) =>
                        patch('index', { writer_buffer_bytes: Number(e.target.value) * 1000 * 1000 })
                      }
                    />
                  </Field>
                </div>
              </Section>

              {/* ── Parser / Tika ── */}
              <Section icon="📑" title="文档解析 (Tika)">
                <div className="grid grid-cols-2 gap-2.5">
                  <Field label="Tika 服务地址" hint="如 http://localhost:9998" restart>
                    <input
                      type="text" className={inputCls} placeholder="留空 = 不启用"
                      value={config.parser.tika_server_url ?? ''}
                      onChange={(e) =>
                        patch('parser', { tika_server_url: e.target.value || null })
                      }
                    />
                  </Field>
                  <Field label="Tika JAR 路径" restart>
                    <input
                      type="text" className={inputCls} placeholder="留空 = 不启用"
                      value={config.parser.tika_jar_path ?? ''}
                      onChange={(e) =>
                        patch('parser', { tika_jar_path: e.target.value || null })
                      }
                    />
                  </Field>
                </div>
                <label className="flex items-center gap-2 text-xs text-[var(--text-secondary)] cursor-pointer">
                  <input
                    type="checkbox"
                    checked={config.parser.fallback_basic}
                    onChange={(e) => patch('parser', { fallback_basic: e.target.checked })}
                    className="accent-[var(--accent)]"
                  />
                  Tika 不可用时回退到内置基础解析器
                </label>
                <p className="text-[11px] text-[var(--text-secondary)] opacity-70">
                  配置 Tika 后可高质量解析 mobi / wps 等复杂格式; 不配置也能用内置解析兜底
                </p>
              </Section>

              {/* ── Watcher ── */}
              <Section icon="👁" title="文件监控">
                <div className="grid grid-cols-3 gap-2.5">
                  <label className="flex items-center gap-2 text-xs text-[var(--text-secondary)] cursor-pointer col-span-1 self-end pb-2">
                    <input
                      type="checkbox"
                      checked={config.watcher.enabled}
                      onChange={(e) => patch('watcher', { enabled: e.target.checked })}
                      className="accent-[var(--accent)]"
                    />
                    启用实时监控
                  </label>
                  <Field label="防抖 (毫秒)" restart>
                    <input
                      type="number" min={100} step={100} className={inputCls}
                      value={config.watcher.debounce_ms}
                      onChange={(e) => patch('watcher', { debounce_ms: Number(e.target.value) })}
                    />
                  </Field>
                  <Field label="定时全量扫描 (秒)" hint="0 = 关闭" restart>
                    <input
                      type="number" min={0} className={inputCls}
                      value={config.watcher.full_scan_interval_secs}
                      onChange={(e) =>
                        patch('watcher', { full_scan_interval_secs: Number(e.target.value) })
                      }
                    />
                  </Field>
                </div>
                <Field label="排除扩展名" hint="逗号分隔, 索引时跳过">
                  <textarea
                    rows={2} className={inputCls}
                    value={listToText(config.watcher.exclude_extensions)}
                    onChange={(e) =>
                      patch('watcher', { exclude_extensions: textToList(e.target.value) })
                    }
                  />
                </Field>
                <Field label="仅包含扩展名" hint="留空 = 索引全部类型; 逗号分隔">
                  <textarea
                    rows={2} className={inputCls} placeholder="如: pdf, docx, txt, md"
                    value={listToText(config.watcher.include_extensions)}
                    onChange={(e) =>
                      patch('watcher', { include_extensions: textToList(e.target.value) })
                    }
                  />
                </Field>
                <Field label="排除路径模式" hint="路径中包含即跳过, 逗号分隔">
                  <textarea
                    rows={2} className={inputCls}
                    value={listToText(config.watcher.exclude_patterns)}
                    onChange={(e) =>
                      patch('watcher', { exclude_patterns: textToList(e.target.value) })
                    }
                  />
                </Field>
              </Section>

              {/* ── Logging ── */}
              <Section icon="📋" title="日志">
                <Field label="日志级别" restart>
                  <select
                    className={inputCls}
                    value={config.logging.level}
                    onChange={(e) => patch('logging', { level: e.target.value })}
                  >
                    {['error', 'warn', 'info', 'debug', 'trace'].map((l) => (
                      <option key={l} value={l}>{l}</option>
                    ))}
                  </select>
                </Field>
              </Section>

              {/* ── Actions ── */}
              <div className="flex items-center gap-3 pt-1">
                <button
                  onClick={handleSave}
                  disabled={saving}
                  className="bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white px-5 py-1.5 rounded-lg text-sm font-medium transition-all disabled:opacity-50"
                >
                  {saving ? '保存中...' : '💾 保存配置'}
                </button>
                <button
                  onClick={load}
                  disabled={loading}
                  className="text-xs text-[var(--text-secondary)] hover:text-[var(--accent)] transition-colors"
                >
                  放弃修改, 重新加载
                </button>
                <span className="text-[11px] text-[var(--text-secondary)] opacity-70">
                  扩展名/路径过滤与大小上限保存后立即生效; 标注「重启生效」的项需重启应用
                </span>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
};
