import os
import requests
import json
from fastapi import FastAPI, HTTPException
from fastapi.responses import HTMLResponse, StreamingResponse
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

app = FastAPI(title="Monarch - AI Comedy Agent")

# Enable CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Configuration
OLLAMA_HOST = os.environ.get("OLLAMA_HOST", "http://localhost:11434")
OLLAMA_API_URL = f"{OLLAMA_HOST}/api/generate"
MODEL_NAME = "monarch"

class PromptRequest(BaseModel):
    prompt: str

@app.get("/", response_class=HTMLResponse)
async def root():
    """Serve the HTML frontend"""
    return """
    <!DOCTYPE html>
    <html>
    <head>
        <title>👑 Monarch - AI Comedy Agent</title>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <style>
            * {
                margin: 0;
                padding: 0;
                box-sizing: border-box;
            }
            body {
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
                background: linear-gradient(135deg, #1e1e1e 0%, #2d2d2d 100%);
                color: #ffffff;
                min-height: 100vh;
                display: flex;
                align-items: center;
                justify-content: center;
                padding: 20px;
            }
            .container {
                max-width: 800px;
                width: 100%;
                background: #2a2a2a;
                border-radius: 12px;
                box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
                overflow: hidden;
                display: flex;
                flex-direction: column;
                max-height: 90vh;
            }
            .header {
                background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                padding: 30px;
                text-align: center;
            }
            .header h1 {
                font-size: 2.5em;
                margin-bottom: 10px;
            }
            .header p {
                font-size: 1.1em;
                opacity: 0.9;
            }
            .chat-container {
                flex: 1;
                overflow-y: auto;
                padding: 20px;
                display: flex;
                flex-direction: column;
                gap: 15px;
            }
            .message {
                padding: 12px 16px;
                border-radius: 8px;
                max-width: 85%;
                word-wrap: break-word;
                animation: slideIn 0.3s ease;
            }
            @keyframes slideIn {
                from {
                    opacity: 0;
                    transform: translateY(10px);
                }
                to {
                    opacity: 1;
                    transform: translateY(0);
                }
            }
            .message.user {
                background: #475063;
                align-self: flex-end;
                border-bottom-right-radius: 0;
            }
            .message.bot {
                background: #3a5f7f;
                align-self: flex-start;
                border-bottom-left-radius: 0;
            }
            .message.loading {
                background: #464646;
                color: #999;
                font-style: italic;
            }
            .input-section {
                padding: 20px;
                border-top: 1px solid #444;
                display: flex;
                gap: 10px;
            }
            input {
                flex: 1;
                padding: 12px 16px;
                border: 1px solid #444;
                border-radius: 8px;
                background: #1a1a1a;
                color: #ffffff;
                font-size: 1em;
            }
            input:focus {
                outline: none;
                border-color: #667eea;
                box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
            }
            button {
                padding: 12px 24px;
                background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                color: white;
                border: none;
                border-radius: 8px;
                cursor: pointer;
                font-size: 1em;
                font-weight: 600;
                transition: transform 0.2s;
            }
            button:hover {
                transform: translateY(-2px);
            }
            button:active {
                transform: translateY(0);
            }
            button:disabled {
                opacity: 0.5;
                cursor: not-allowed;
            }
            .error {
                background: #8b3a3a;
                color: #ffcccc;
            }
        </style>
    </head>
    <body>
        <div class="container">
            <div class="header">
                <h1>👑 Monarch</h1>
                <p>Your Local AI Comedy Agent - Powered by Ollama</p>
            </div>
            <div class="chat-container" id="chatContainer"></div>
            <div class="input-section">
                <input type="text" id="promptInput" placeholder="Feed me a setup, I'll give you the punchline..." />
                <button id="sendBtn" onclick="sendMessage()">Send</button>
            </div>
        </div>

        <script>
            const chatContainer = document.getElementById('chatContainer');
            const promptInput = document.getElementById('promptInput');
            const sendBtn = document.getElementById('sendBtn');

            promptInput.addEventListener('keypress', (e) => {
                if (e.key === 'Enter') sendMessage();
            });

            function addMessage(text, isUser = false) {
                const msg = document.createElement('div');
                msg.className = `message ${isUser ? 'user' : 'bot'}`;
                msg.textContent = text;
                chatContainer.appendChild(msg);
                chatContainer.scrollTop = chatContainer.scrollHeight;
                return msg;
            }

            async function sendMessage() {
                const prompt = promptInput.value.trim();
                if (!prompt) return;

                addMessage(prompt, true);
                promptInput.value = '';
                sendBtn.disabled = true;

                const loadingMsg = addMessage('...', false);
                loadingMsg.classList.add('loading');

                try {
                    const response = await fetch('/api/generate', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ prompt })
                    });

                    if (!response.ok) throw new Error('Failed to get response');

                    const reader = response.body.getReader();
                    const decoder = new TextDecoder();
                    let fullResponse = '';

                    loadingMsg.remove();
                    const botMsg = addMessage('', false);

                    while (true) {
                        const { done, value } = await reader.read();
                        if (done) break;

                        const chunk = decoder.decode(value);
                        fullResponse += chunk;
                        botMsg.textContent = fullResponse;
                        chatContainer.scrollTop = chatContainer.scrollHeight;
                    }

                    if (fullResponse) {
                        loadingMsg.textContent = fullResponse;
                        loadingMsg.classList.remove('loading');
                    } else {
                        loadingMsg.textContent = 'No response received';
                    }
                } catch (error) {
                    loadingMsg.className = 'message bot error';
                    loadingMsg.textContent = `Error: ${error.message}`;
                } finally {
                    sendBtn.disabled = false;
                    promptInput.focus();
                }
            }
        </script>
    </body>
    </html>
    """

@app.post("/api/generate")
async def generate(request: PromptRequest):
    """Stream responses from Ollama"""
    # Format prompt for comedy generation
    comedy_prompt = f"""You are a world-class comedy writer. Your job is to respond with funny, clever punchlines.

Setup: {request.prompt}

Response:"""

    payload = {
        "model": MODEL_NAME,
        "prompt": comedy_prompt,
        "stream": True,
        "temperature": 0.9,
        "top_p": 0.95,
        "top_k": 50,
    }

    try:
        response = requests.post(OLLAMA_API_URL, json=payload, stream=True)
        response.raise_for_status()

        def generate_content():
            for line in response.iter_lines():
                if line:
                    try:
                        body = json.loads(line)
                        if "response" in body:
                            yield body["response"]
                        if body.get("done"):
                            break
                    except json.JSONDecodeError:
                        continue

        return StreamingResponse(generate_content(), media_type="text/plain")

    except requests.exceptions.RequestException as e:
        raise HTTPException(status_code=500, detail=f"Ollama connection failed: {str(e)}")

@app.get("/health")
async def health():
    """Health check endpoint"""
    return {"status": "ok"}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
