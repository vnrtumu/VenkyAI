interface TranscriptEntry {
    role: string;
    content: string;
    timestamp: string;
}

interface SessionInfo {
    id: string;
    start_time: string;
    title: string;
    is_active: boolean;
}

interface TranscriptProps {
    entries: TranscriptEntry[];
    sessionInfo: SessionInfo | null;
    isActive: boolean;
}

function Transcript({ entries, sessionInfo, isActive }: TranscriptProps) {
    return (
        <div className="transcript-panel">
            {/* Session Info */}
            {sessionInfo && (
                <div className="session-info">
                    <h4>{isActive ? '🟢 Session Active' : '⚪ Session Ended'}</h4>
                    <p>{sessionInfo.title} — Started {new Date(sessionInfo.start_time).toLocaleTimeString()}</p>
                </div>
            )}

            {!sessionInfo && (
                <div className="transcript-empty">
                    <p>📝 No active session</p>
                    <p style={{ marginTop: '4px', fontSize: '11px' }}>
                        Press ▶ to start a session and capture the conversation
                    </p>
                </div>
            )}

            {/* Transcript Entries */}
            {entries.map((entry, i) => (
                <div key={i} className={`transcript-entry ${entry.role}`}>
                    <span className="entry-role">
                        {entry.role === 'transcription' ? '🎙 Transcription' : entry.role === 'user' ? '👤 You' : '⚡ AI'}
                    </span>
                    <span className="entry-time">{entry.timestamp}</span>
                    <div className="entry-content">{entry.content}</div>
                </div>
            ))}

            {entries.length === 0 && sessionInfo && (
                <div className="transcript-empty" style={{ paddingTop: '12px' }}>
                    <p>No transcript entries yet. Record audio and transcribe to start capturing.</p>
                </div>
            )}
        </div>
    );
}

export default Transcript;
