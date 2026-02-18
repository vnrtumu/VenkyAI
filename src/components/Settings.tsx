import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface AppConfig {
    llm_provider: string;
    openai_api_key: string;
    openai_model: string;
    ollama_url: string;
    ollama_model: string;
    capture_interval_secs: number;
    whisper_model: string;
    hotkey: string;
}

interface CRMConfig {
    provider: string;
    api_key: string;
    instance_url: string;
}

interface CRMProvider {
    name: string;
    id: string;
    description: string;
    requiresInstanceUrl: boolean;
}

interface PromptTemplate {
    id: i32;
    name: string;
    content: string;
}

type i32 = number;

function Settings() {
    const [config, setConfig] = useState<AppConfig | null>(null);
    const [crmConfig, setCrmConfig] = useState<CRMConfig>({ provider: 'None', api_key: '', instance_url: '' });
    const [crmProviders, setCrmProviders] = useState<CRMProvider[]>([]);
    const [saveStatus, setSaveStatus] = useState('');
    const [activeSection, setActiveSection] = useState<'llm' | 'crm' | 'capture' | 'hotkey'>('llm');

    useEffect(() => {
        loadConfig();
        loadCRMConfig();
        loadCRMProviders();
    }, []);

    const loadConfig = async () => {
        try {
            const cfg = await invoke<AppConfig>('get_config');
            setConfig(cfg);
        } catch (e) {
            console.error('Error loading config:', e);
        }
    };

    const loadCRMConfig = async () => {
        try {
            const cfg = await invoke<CRMConfig>('get_crm_config');
            setCrmConfig(cfg);
        } catch (e) {
            console.error('Error loading CRM config:', e);
        }
    };

    const loadCRMProviders = async () => {
        try {
            const providers = await invoke<CRMProvider[]>('get_crm_providers');
            setCrmProviders(providers);
        } catch (e) {
            console.error('Error loading CRM providers:', e);
        }
    };

    const saveConfig = async () => {
        if (!config) return;
        try {
            await invoke('update_config', { newConfig: config });
            setSaveStatus('✅ Settings saved');
            setTimeout(() => setSaveStatus(''), 2000);
        } catch (e: any) {
            setSaveStatus(`❌ ${e}`);
        }
    };

    const saveCRMConfig = async () => {
        try {
            await invoke('update_crm_config', { config: crmConfig });
            setSaveStatus('✅ CRM config saved');
            setTimeout(() => setSaveStatus(''), 2000);
        } catch (e: any) {
            setSaveStatus(`❌ ${e}`);
        }
    };

    if (!config) return <div className="settings-panel"><p>Loading...</p></div>;

    const selectedCRM = crmProviders.find(p => p.id === crmConfig.provider);

    return (
        <div className="settings-panel">
            {/* Section Tabs */}
            <div className="settings-tabs">
                <button className={`settings-tab ${activeSection === 'llm' ? 'active' : ''}`} onClick={() => setActiveSection('llm')}>
                    🤖 LLM
                </button>
                <button className={`settings-tab ${activeSection === 'crm' ? 'active' : ''}`} onClick={() => setActiveSection('crm')}>
                    📊 CRM
                </button>
                <button className={`settings-tab ${activeSection === 'capture' ? 'active' : ''}`} onClick={() => setActiveSection('capture')}>
                    📷 Capture
                </button>
                <button className={`settings-tab ${activeSection === 'hotkey' ? 'active' : ''}`} onClick={() => setActiveSection('hotkey')}>
                    ⌨️ Hotkey
                </button>
            </div>

            {/* LLM Settings */}
            {activeSection === 'llm' && (
                <div className="settings-section">
                    <h3>LLM Provider</h3>
                    <div className="form-group">
                        <label>Provider</label>
                        <select
                            value={config.llm_provider}
                            onChange={(e) => setConfig({ ...config, llm_provider: e.target.value })}
                            className="form-select"
                        >
                            <option value="OpenAI">OpenAI</option>
                            <option value="Ollama">Ollama (Local)</option>
                        </select>
                    </div>

                    {config.llm_provider === 'OpenAI' && (
                        <>
                            <div className="form-group">
                                <label>API Key</label>
                                <input
                                    type="password"
                                    value={config.openai_api_key}
                                    onChange={(e) => setConfig({ ...config, openai_api_key: e.target.value })}
                                    placeholder="sk-..."
                                    className="form-input"
                                />
                            </div>
                            <div className="form-group">
                                <label>Model</label>
                                <select
                                    value={config.openai_model}
                                    onChange={(e) => setConfig({ ...config, openai_model: e.target.value })}
                                    className="form-select"
                                >
                                    <option value="gpt-4o">GPT-4o (Recommended)</option>
                                    <option value="gpt-4o-mini">GPT-4o Mini (Faster)</option>
                                    <option value="gpt-4-turbo">GPT-4 Turbo</option>
                                    <option value="o1">o1 (Reasoning)</option>
                                </select>
                            </div>
                        </>
                    )}

                    {config.llm_provider === 'Ollama' && (
                        <>
                            <div className="form-group">
                                <label>Ollama URL</label>
                                <input
                                    type="text"
                                    value={config.ollama_url}
                                    onChange={(e) => setConfig({ ...config, ollama_url: e.target.value })}
                                    className="form-input"
                                />
                            </div>
                            <div className="form-group">
                                <label>Model</label>
                                <input
                                    type="text"
                                    value={config.ollama_model}
                                    onChange={(e) => setConfig({ ...config, ollama_model: e.target.value })}
                                    placeholder="llama3"
                                    className="form-input"
                                />
                            </div>
                        </>
                    )}

                    <button className="save-btn" onClick={saveConfig}>💾 Save LLM Settings</button>
                </div>
            )}

            {/* CRM Settings */}
            {activeSection === 'crm' && (
                <div className="settings-section">
                    <h3>CRM Integration</h3>
                    <div className="form-group">
                        <label>Provider</label>
                        <select
                            value={crmConfig.provider}
                            onChange={(e) => setCrmConfig({ ...crmConfig, provider: e.target.value })}
                            className="form-select"
                        >
                            <option value="None">None</option>
                            {crmProviders.map(p => (
                                <option key={p.id} value={p.id}>{p.name}</option>
                            ))}
                        </select>
                    </div>

                    {selectedCRM && (
                        <p className="provider-desc">{selectedCRM.description}</p>
                    )}

                    {crmConfig.provider !== 'None' && (
                        <>
                            <div className="form-group">
                                <label>{crmConfig.provider === 'Salesforce' ? 'Access Token' : 'API Key'}</label>
                                <input
                                    type="password"
                                    value={crmConfig.api_key}
                                    onChange={(e) => setCrmConfig({ ...crmConfig, api_key: e.target.value })}
                                    placeholder={crmConfig.provider === 'Salesforce' ? 'Salesforce access token' : 'HubSpot API key'}
                                    className="form-input"
                                />
                            </div>

                            {selectedCRM?.requiresInstanceUrl && (
                                <div className="form-group">
                                    <label>Instance URL</label>
                                    <input
                                        type="text"
                                        value={crmConfig.instance_url}
                                        onChange={(e) => setCrmConfig({ ...crmConfig, instance_url: e.target.value })}
                                        placeholder="https://yourcompany.my.salesforce.com"
                                        className="form-input"
                                    />
                                </div>
                            )}

                            <button className="save-btn" onClick={saveCRMConfig}>💾 Save CRM Settings</button>
                        </>
                    )}
                </div>
            )}

            {/* Capture Settings */}
            {activeSection === 'capture' && (
                <div className="settings-section">
                    <h3>Capture Settings</h3>
                    <div className="form-group">
                        <label>Screen Capture Interval (seconds)</label>
                        <input
                            type="number"
                            min="1"
                            max="60"
                            value={config.capture_interval_secs}
                            onChange={(e) => setConfig({ ...config, capture_interval_secs: parseInt(e.target.value) || 5 })}
                            className="form-input"
                        />
                    </div>
                    <div className="form-group">
                        <label>Whisper STT Model</label>
                        <select
                            value={config.whisper_model}
                            onChange={(e) => setConfig({ ...config, whisper_model: e.target.value })}
                            className="form-select"
                        >
                            <option value="whisper-1">Whisper-1 (OpenAI API)</option>
                        </select>
                        <p className="form-hint">Uses OpenAI's Whisper API for speech-to-text</p>
                    </div>
                    <button className="save-btn" onClick={saveConfig}>💾 Save Capture Settings</button>
                </div>
            )}

            {/* Hotkey Settings */}
            {activeSection === 'hotkey' && (
                <div className="settings-section">
                    <h3>Keyboard Shortcuts</h3>
                    <div className="hotkey-display">
                        <div className="hotkey-item">
                            <span className="hotkey-label">Toggle Overlay</span>
                            <kbd className="hotkey-keys">⌘ + ⇧ + C</kbd>
                        </div>
                        <p className="form-hint">Press Cmd+Shift+C (macOS) or Ctrl+Shift+C (Windows/Linux) to show/hide the overlay window.</p>
                    </div>

                    <div className="hotkey-display" style={{ marginTop: '12px' }}>
                        <div className="hotkey-item">
                            <span className="hotkey-label">System Tray</span>
                            <span className="tray-info">✅ Active</span>
                        </div>
                        <p className="form-hint">Right-click the tray icon for Show/Hide/Quit options. The app stays in the background when the overlay is hidden.</p>
                    </div>
                </div>
            )}

            {/* Status */}
            {saveStatus && (
                <div className="save-status">{saveStatus}</div>
            )}
        </div>
    );
}

export default Settings;
