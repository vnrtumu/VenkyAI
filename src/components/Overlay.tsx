import { useState, useRef, useEffect } from 'react';

interface OverlayProps {
    suggestions: string[];
    onSendMessage: (message: string) => void;
    isLoading: boolean;
    isStreaming: boolean;
    streamingText: string;
}

function Overlay({ suggestions, onSendMessage, isLoading, isStreaming, streamingText }: OverlayProps) {
    const [input, setInput] = useState('');
    const messagesRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (messagesRef.current) {
            messagesRef.current.scrollTop = messagesRef.current.scrollHeight;
        }
    }, [suggestions, streamingText]);

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        if (!input.trim() || isLoading) return;
        onSendMessage(input.trim());
        setInput('');
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            handleSubmit(e);
        }
    };

    return (
        <div className="overlay-panel">
            <div className="messages-container" ref={messagesRef}>
                {suggestions.length === 0 && !isStreaming && (
                    <div className="empty-state">
                        <div className="empty-icon">⚡</div>
                        <p className="empty-title">VenkyAI Assistant</p>
                        <p className="empty-subtitle">Ask anything, get real-time help during meetings</p>
                        <div className="quick-actions">
                            <button className="quick-action" onClick={() => onSendMessage('Summarize the meeting so far')}>
                                📋 Summarize
                            </button>
                            <button className="quick-action" onClick={() => onSendMessage('What questions should I ask?')}>
                                ❓ Questions
                            </button>
                            <button className="quick-action" onClick={() => onSendMessage('Generate action items')}>
                                ✅ Actions
                            </button>
                        </div>
                    </div>
                )}

                {suggestions.map((msg, idx) => {
                    const isUser = msg.startsWith('You: ');
                    const isError = msg.startsWith('Error: ');

                    let content = msg;
                    if (isUser) content = msg.slice(5);
                    if (isError) content = msg.slice(7);

                    return (
                        <div key={idx} className={`message ${isUser ? 'user' : isError ? 'error' : 'assistant'}`}>
                            <div className="message-avatar">
                                {isUser ? '👤' : isError ? '⚠️' : '⚡'}
                            </div>
                            <div className="message-content">
                                <div className="message-text">{content}</div>
                            </div>
                        </div>
                    );
                })}

                {/* Streaming response */}
                {isStreaming && streamingText && (
                    <div className="message assistant streaming">
                        <div className="message-avatar">⚡</div>
                        <div className="message-content">
                            <div className="message-text">{streamingText}<span className="cursor-blink">▋</span></div>
                        </div>
                    </div>
                )}

                {/* Loading indicator / Pre-streaming state */}
                {(isLoading || (isStreaming && !streamingText)) && (
                    <div className="message assistant loading-msg">
                        <div className="message-avatar">⚡</div>
                        <div className="message-content">
                            <div className="typing-indicator">
                                <span></span><span></span><span></span>
                            </div>
                        </div>
                    </div>
                )}
            </div>

            {/* Input */}
            <form className="input-area" onSubmit={handleSubmit}>
                <textarea
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder="Ask VenkyAI anything..."
                    rows={1}
                    disabled={isLoading}
                    className="chat-input"
                />
                <button type="submit" disabled={!input.trim() || isLoading} className="send-btn">
                    {isLoading ? '⏳' : '↑'}
                </button>
            </form>
        </div>
    );
}

export default Overlay;
