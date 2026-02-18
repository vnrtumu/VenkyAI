import { useState } from 'react';

interface SessionSetupProps {
    onStart: (title: string, purpose: string, context: string) => void;
    onCancel: () => void;
}

const PURPOSES = [
    { id: 'meeting', name: 'Meeting', icon: '👥', description: 'General meeting assistance' },
    { id: 'interview', name: 'Interview', icon: '👔', description: 'Tailored for interviews (Enter resume/profile below)' },
    { id: 'sales', name: 'Sales Call', icon: '💰', description: 'Focused on objections and value' },
    { id: 'casual', name: 'Casual', icon: '💬', description: 'Transcribe and help with casual talk' },
];

function SessionSetup({ onStart, onCancel }: SessionSetupProps) {
    const [title, setTitle] = useState('New Session');
    const [selectedPurpose, setSelectedPurpose] = useState('meeting');
    const [context, setContext] = useState('');

    return (
        <div className="session-setup-overlay">
            <div className="session-setup-card">
                <div className="setup-header">
                    <h3>🚀 Start New Session</h3>
                    <p>Tell us what this session is about for better AI assistance</p>
                </div>

                <div className="setup-body">
                    <div className="form-group">
                        <label>Session Title</label>
                        <input
                            type="text"
                            className="form-input"
                            value={title}
                            onChange={(e) => setTitle(e.target.value)}
                            placeholder="e.g. Design Sync, Tech Interview"
                        />
                    </div>

                    <div className="form-group">
                        <label>Purpose</label>
                        <div className="purpose-grid">
                            {PURPOSES.map((p) => (
                                <button
                                    key={p.id}
                                    className={`purpose-item ${selectedPurpose === p.id ? 'active' : ''}`}
                                    onClick={() => setSelectedPurpose(p.id)}
                                >
                                    <span className="purpose-icon">{p.icon}</span>
                                    <div className="purpose-info">
                                        <div className="purpose-name">{p.name}</div>
                                        <div className="purpose-desc">{p.description}</div>
                                    </div>
                                </button>
                            ))}
                        </div>
                    </div>

                    {selectedPurpose === 'interview' && (
                        <div className="form-group">
                            <label>Resume / LinkedIn Profile / Job Description</label>
                            <textarea
                                className="form-textarea"
                                value={context}
                                onChange={(e) => setContext(e.target.value)}
                                placeholder="Paste resume text or job details here to help the AI prepare answers..."
                                rows={4}
                            />
                        </div>
                    )}
                </div>

                <div className="setup-footer">
                    <button className="cancel-btn" onClick={onCancel}>Cancel</button>
                    <button
                        className="start-session-btn"
                        onClick={() => onStart(title, selectedPurpose, context)}
                        disabled={!title.trim()}
                    >
                        Start Session
                    </button>
                </div>
            </div>
        </div>
    );
}

export default SessionSetup;
