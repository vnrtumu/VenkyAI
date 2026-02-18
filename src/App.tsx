import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import Settings from './components/Settings';
import Transcript from './components/Transcript';
import Overlay from './components/Overlay';
import SessionSetup from './components/SessionSetup';
import './index.css';

interface TranscriptEntry {
    role: string;
    content: string;
    timestamp: string;
}

interface Session {
    id: string;
    title: string;
    purpose: string;
    status: 'Active' | 'Paused' | 'Ended';
    start_time: string;
    end_time?: string;
}

type Tab = 'chat' | 'transcript' | 'settings';

function App() {
    const [activeTab, setActiveTab] = useState<Tab>('chat');
    const [isSessionActive, setIsSessionActive] = useState(false);
    const [sessionInfo, setSessionInfo] = useState<Session | null>(null);
    const [transcript, setTranscript] = useState<TranscriptEntry[]>([]);
    const [isRecording, setIsRecording] = useState(false);
    const [isCapturing, setIsCapturing] = useState(false);
    const [suggestions, setSuggestions] = useState<string[]>([]);
    const [isLoading, setIsLoading] = useState(false);
    const [isStreaming, setIsStreaming] = useState(false);
    const [streamingText, setStreamingText] = useState('');
    const [isTranscribing, setIsTranscribing] = useState(false);
    const [overlayVisible, setOverlayVisible] = useState(true);
    const [showSetup, setShowSetup] = useState(false);
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
            if (event.payload && !event.payload.includes('[SILENCE]')) {
                setSuggestions((prev: string[]) => [...prev, event.payload]);
            }
            setStreamingText('');
        });

        // Listen for overlay visibility changes (from hotkey)
        const unlistenVisibility = listen<boolean>('overlay-visibility', (event) => {
            setOverlayVisible(event.payload);
        });

        // Listen for meeting detection
        const unlistenMeeting = listen<string>('meeting-detected', (_event) => {
            // No longer displaying in chat as per user request
            // setSuggestions((prev: string[]) => [...prev, `🔍 Meeting Detected: ${event.payload}. VenkyAI is ready to assist.`]);
        });

        const unlistenAutoStart = listen<Session>('session-auto-started', (event) => {
            setSessionInfo(event.payload);
            setIsSessionActive(true);
            setTranscript([]);
            setIsRecording(true);
            setIsCapturing(true);
            // setSuggestions((prev: string[]) => [...prev, `🚀 Automated session started: ${event.payload.title}`]);
        });

        // Listen for background transcription chunks
        const unlistenTranscription = listen<string>('transcription-chunk', async (event) => {
            const text = event.payload;
            setTranscript((prev: TranscriptEntry[]) => [...prev, {
                role: 'transcription',
                content: text,
                timestamp: new Date().toLocaleTimeString()
            }]);

            // Persist to session in background
            invoke('add_transcript_entry', {
                speaker: 'transcription',
                text: text
            }).catch(e => console.error('Failed to save background transcript:', e));
        });

        // Listen for live AI suggestions
        const unlistenLiveSuggestion = listen<string>('live-suggestion', (event) => {
            const text = event.payload;
            if (text && !text.includes('[SILENCE]')) {
                setSuggestions((prev: string[]) => [...prev, text]);
            }
        });

        return () => {
            unlistenToken.then(fn => fn());
            unlistenStart.then(fn => fn());
            unlistenEnd.then(fn => fn());
            unlistenVisibility.then(fn => fn());
            unlistenMeeting.then(fn => fn());
            unlistenAutoStart.then(fn => fn());
            unlistenTranscription.then(fn => fn());
            unlistenLiveSuggestion.then(fn => fn());
        };
    }, []);

    // ─── Session Controls ───────────────────────────────────────────────
    const startSession = async (title: string, purpose: string, context: string) => {
        try {
            const info = await invoke<Session>('create_session', { title, purpose, context });
            setSessionInfo(info);
            setIsSessionActive(true);
            setTranscript([]);
            setShowSetup(false);
        } catch (e: any) {
            console.error('Failed to start session:', e);
            // Even if it fails, we should probably close the setup screen or show an error
            setShowSetup(false);
            alert(`Failed to start session: ${e}`);
        }
    };

    const endSession = async () => {
        try {
            await invoke('end_session');
            setIsSessionActive(false);

            // Cleanup: Stop all captures
            if (isRecording) {
                await invoke('stop_audio_capture').catch(console.error);
                setIsRecording(false);
            }
            if (isCapturing) {
                // System audio capture is also stopped here if active
                await invoke('stop_system_audio_capture').catch(console.error);
                setIsCapturing(false);
            }

            // Clear capture interval if exists
            if (captureInterval.current) {
                clearInterval(captureInterval.current);
                captureInterval.current = null;
            }

            setSuggestions([]);
            setStreamingText('');

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

                {isSessionActive && (
                    <div className="live-indicator-badge">
                        <span className="live-dot"></span>
                        LIVE ✨
                    </div>
                )}

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
                        onClick={isSessionActive ? endSession : () => setShowSetup(true)}
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

            {/* Session Setup Overlay */}
            {showSetup && (
                <SessionSetup
                    onStart={startSession}
                    onCancel={() => setShowSetup(false)}
                />
            )}
        </div>
    );
}

export default App;
