import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import Settings from './components/Settings';
import Transcript from './components/Transcript';
import Overlay from './components/Overlay';
import './index.css';

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

type Tab = 'chat' | 'transcript' | 'settings';

function App() {
    const [activeTab, setActiveTab] = useState<Tab>('chat');
    const [isSessionActive, setIsSessionActive] = useState(false);
    const [sessionInfo, setSessionInfo] = useState<SessionInfo | null>(null);
    const [transcript, setTranscript] = useState<TranscriptEntry[]>([]);
    const [isRecording, setIsRecording] = useState(false);
    const [isCapturing, setIsCapturing] = useState(false);
    const [suggestions, setSuggestions] = useState<string[]>([]);
    const [isLoading, setIsLoading] = useState(false);
    const [isStreaming, setIsStreaming] = useState(false);
    const [streamingText, setStreamingText] = useState('');
    const [isTranscribing, setIsTranscribing] = useState(false);
    const [overlayVisible, setOverlayVisible] = useState(true);
    const captureInterval = useRef<ReturnType<typeof setInterval> | null>(null);

    // ─── Event Listeners ────────────────────────────────────────────────
    useEffect(() => {
        // Listen for streaming tokens
        const unlistenToken = listen<string>('llm-token', (event) => {
            setStreamingText((prev: string) => prev + event.payload);
        });

        const unlistenStart = listen('llm-stream-start', () => {
            setIsStreaming(true);
            setStreamingText('');
        });

        const unlistenEnd = listen<string>('llm-stream-end', (event) => {
            setIsStreaming(false);
            setSuggestions((prev: string[]) => [...prev, event.payload]);
            setStreamingText('');
        });

        // Listen for overlay visibility changes (from hotkey)
        const unlistenVisibility = listen<boolean>('overlay-visibility', (event) => {
            setOverlayVisible(event.payload);
        });

        // Listen for meeting detection
        const unlistenMeeting = listen<string>('meeting-detected', (event) => {
            setSuggestions((prev: string[]) => [...prev, `🔍 Meeting detected: ${event.payload}. Click ▶ to start or wait for auto-start.`]);
        });

        const unlistenAutoStart = listen<SessionInfo>('session-auto-started', (event) => {
            setSessionInfo(event.payload);
            setIsSessionActive(true);
            setTranscript([]);
            setIsRecording(true);
            setIsCapturing(true);
            setSuggestions((prev: string[]) => [...prev, `🚀 Automated session started: ${event.payload.title}`]);
        });

        return () => {
            unlistenToken.then(fn => fn());
            unlistenStart.then(fn => fn());
            unlistenEnd.then(fn => fn());
            unlistenVisibility.then(fn => fn());
            unlistenMeeting.then(fn => fn());
            unlistenAutoStart.then(fn => fn());
        };
    }, []);

    // ─── Session Controls ───────────────────────────────────────────────
    const startSession = async () => {
        try {
            const info = await invoke<SessionInfo>('create_session', { title: 'Meeting' });
            setSessionInfo(info);
            setIsSessionActive(true);
            setTranscript([]);
        } catch (e: any) {
            console.error('Failed to start session:', e);
        }
    };

    const endSession = async () => {
        try {
            await invoke('end_session');
            setIsSessionActive(false);
        } catch (e: any) {
            console.error('Failed to end session:', e);
        }
    };

    // ─── Audio Recording ───────────────────────────────────────────────
    const toggleRecording = async () => {
        try {
            if (isRecording) {
                await invoke('stop_audio_capture');
                setIsRecording(false);
            } else {
                await invoke('start_audio_capture');
                setIsRecording(true);
            }
        } catch (e: any) {
            console.error('Audio error:', e);
        }
    };

    // ─── Speech-to-Text ─────────────────────────────────────────────────
    const transcribeAudio = async () => {
        try {
            setIsTranscribing(true);
            const text = await invoke<string>('transcribe_audio');
            if (text) {
                setTranscript((prev: TranscriptEntry[]) => [...prev, {
                    role: 'transcription',
                    content: text,
                    timestamp: new Date().toLocaleTimeString()
                }]);
                // Add to session transcript
                if (isSessionActive) {
                    await invoke('add_transcript_entry', {
                        speaker: 'user',
                        text: text
                    });
                }
            }
        } catch (e: any) {
            console.error('Transcription error:', e);
        } finally {
            setIsTranscribing(false);
        }
    };

    // ─── Screen Capture ──────────────────────────────────────────────────
    const toggleCapture = () => {
        if (isCapturing) {
            if (captureInterval.current) {
                clearInterval(captureInterval.current);
                captureInterval.current = null;
            }
            setIsCapturing(false);
        } else {
            setIsCapturing(true);
            captureInterval.current = setInterval(async () => {
                try {
                    await invoke('capture_screen');
                } catch (e: any) {
                    console.error('Capture error:', e);
                }
            }, 5000);
        }
    };

    // ─── AI Chat (Streaming) ─────────────────────────────────────────────
    const sendMessage = async (message: string) => {
        setIsLoading(true);
        setSuggestions((prev: string[]) => [...prev, `You: ${message}`]);
        setTranscript((prev: TranscriptEntry[]) => [...prev, {
            role: 'user',
            content: message,
            timestamp: new Date().toLocaleTimeString()
        }]);

        try {
            // Use streaming endpoint
            const response = await invoke<string>('stream_chat', {
                messages: [{ role: 'user', content: message }],
                systemPrompt: 'You are VenkyAI, an AI meeting assistant. Help the user with their meeting, interview, or sales call. Be concise, helpful, and actionable.',
            });

            setTranscript((prev: TranscriptEntry[]) => [...prev, {
                role: 'assistant',
                content: response,
                timestamp: new Date().toLocaleTimeString()
            }]);

            if (isSessionActive) {
                await invoke('add_transcript_entry', { speaker: 'user', text: message });
                await invoke('add_transcript_entry', { speaker: 'assistant', text: response });
            }
        } catch (e: any) {
            setSuggestions((prev: string[]) => [...prev, `Error: ${e}`]);
        } finally {
            setIsLoading(false);
        }
    };



    // ─── Render ──────────────────────────────────────────────────────────
    if (!overlayVisible) return null;

    return (
        <div className="app-container" data-tauri-drag-region>
            {/* Header */}
            <div className="app-header" data-tauri-drag-region>
                <div className="header-left">
                    <span className="app-logo">⚡</span>
                    <span className="app-title">VenkyAI</span>
                    <span className="hotkey-badge" title="Press Cmd/Ctrl+Shift+C to toggle">⌘⇧C</span>
                </div>
                <div className="header-controls">
                    {/* STT Button */}
                    <button
                        className={`control-btn ${isTranscribing ? 'active' : ''}`}
                        onClick={transcribeAudio}
                        disabled={isTranscribing || !isRecording}
                        title="Transcribe audio (record first)"
                    >
                        {isTranscribing ? '⏳' : '📝'}
                    </button>
                    {/* Audio */}
                    <button
                        className={`control-btn ${isRecording ? 'recording' : ''}`}
                        onClick={toggleRecording}
                        title={isRecording ? 'Stop recording' : 'Start recording'}
                    >
                        {isRecording ? '⏹' : '🎙'}
                    </button>
                    {/* Screen Capture */}
                    <button
                        className={`control-btn ${isCapturing ? 'capturing' : ''}`}
                        onClick={toggleCapture}
                        title={isCapturing ? 'Stop capture' : 'Start screen capture'}
                    >
                        {isCapturing ? '🔴' : '📷'}
                    </button>
                    {/* Session */}
                    <button
                        className={`control-btn ${isSessionActive ? 'session-active' : ''}`}
                        onClick={isSessionActive ? endSession : startSession}
                        title={isSessionActive ? 'End session' : 'Start session'}
                    >
                        {isSessionActive ? '⏸' : '▶'}
                    </button>
                </div>
            </div>

            {/* Tab Navigation */}
            <div className="tab-bar">
                <button className={`tab ${activeTab === 'chat' ? 'active' : ''}`} onClick={() => setActiveTab('chat')}>
                    💬 Chat
                </button>
                <button className={`tab ${activeTab === 'transcript' ? 'active' : ''}`} onClick={() => setActiveTab('transcript')}>
                    📜 Transcript
                </button>
                <button className={`tab ${activeTab === 'settings' ? 'active' : ''}`} onClick={() => setActiveTab('settings')}>
                    ⚙️ Settings
                </button>
            </div>

            {/* Tab Content */}
            <div className="tab-content">
                {activeTab === 'chat' && (
                    <Overlay
                        suggestions={suggestions}
                        onSendMessage={sendMessage}
                        isLoading={isLoading}
                        isStreaming={isStreaming}
                        streamingText={streamingText}
                        isLive={isSessionActive}
                    />
                )}
                {activeTab === 'transcript' && (
                    <Transcript
                        entries={transcript}
                        sessionInfo={sessionInfo}
                        isActive={isSessionActive}
                    />
                )}
                {activeTab === 'settings' && <Settings />}
            </div>

            {/* Status Bar */}
            <div className="status-bar">
                <span className={`status-dot ${isSessionActive ? 'active' : ''}`}></span>
                <span className="status-text">
                    {isSessionActive ? 'Session active' : 'Ready'}
                    {isRecording && ' • 🎙 Recording'}
                    {isCapturing && ' • 📷 Capturing'}
                    {isStreaming && ' • 🤖 Generating...'}
                    {isTranscribing && ' • 📝 Transcribing...'}
                </span>
            </div>
        </div>
    );
}

export default App;
