import json
import os
import time

import requests
import streamlit as st

# --- Configuration ---
OLLAMA_HOST = os.environ.get("OLLAMA_HOST", "http://localhost:11434")
OLLAMA_API_URL = f"{OLLAMA_HOST}/api/generate"
MODEL_NAME = "monarch"  # Ensure this matches your local model name in Ollama

st.set_page_config(
    page_title="Monarch - AI Comedy Agent",
    page_icon="👑",
    layout="centered",
)

# --- Styling ---
st.markdown(
    """
    <style>
    .stApp {
        background-color: #1E1E1E;
        color: #FFFFFF;
    }
    .chat-message {
        padding: 1.5rem;
        border-radius: 0.5rem;
        margin-bottom: 1rem;
        display: flex;
    }
    .chat-message.user {
        background-color: #2b313e;
    }
    .chat-message.bot {
        background-color: #475063;
    }
    .chat-message .avatar {
        width: 40px;
        height: 40px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 20px;
        margin-right: 15px;
    }
    .chat-message.user .avatar {
        background-color: #1e81b0;
    }
    .chat-message.bot .avatar {
        background-color: #e28743;
    }
    .chat-message .content {
        line-height: 1.6;
    }
    </style>
    """,
    unsafe_allow_html=True,
)

# --- Header ---
st.title("👑 Monarch")
st.caption("Your Local AI Comedy Agent - Powered by Ollama")

# --- Session State for Chat History ---
if "messages" not in st.session_state:
    st.session_state.messages = []


# --- Helper Functions ---
def generate_response(prompt):
    """Calls the local Ollama API to generate a response."""
    payload = {
        "model": MODEL_NAME,
        "prompt": prompt,
        "stream": True,
    }

    try:
        response = requests.post(OLLAMA_API_URL, json=payload, stream=True)
        response.raise_for_status()

        for line in response.iter_lines():
            if line:
                body = json.loads(line)
                if "response" in body:
                    yield body["response"]
                if body.get("done"):
                    break
    except requests.exceptions.RequestException as e:
        yield f"\n\n*Error: Could not connect to Ollama. Is it running? Details: {e}*"


# --- Display Chat History ---
for message in st.session_state.messages:
    with st.chat_message(message["role"]):
        st.markdown(message["content"])

# --- User Input ---
if prompt := st.chat_input("Feed me a setup, I'll give you the punchline..."):
    # Add user message to state and display
    st.session_state.messages.append({"role": "user", "content": prompt})
    with st.chat_message("user"):
        st.markdown(prompt)

    # Display assistant response
    with st.chat_message("assistant"):
        message_placeholder = st.empty()
        full_response = ""

        # Stream the response
        for chunk in generate_response(prompt):
            full_response += chunk
            message_placeholder.markdown(full_response + "▌")

        message_placeholder.markdown(full_response)

    # Add assistant response to state
    st.session_state.messages.append({"role": "assistant", "content": full_response})
