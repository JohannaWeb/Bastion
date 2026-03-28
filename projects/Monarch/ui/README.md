# Monarch AI Comedy Agent - UI Wrapper

This directory contains the Streamlit-based web user interface for **Monarch**, your local AI comedy agent powered by [Ollama](https://ollama.ai/).

## Prerequisites

Before running the UI, ensure you have the following installed and running:

1. **Ollama**: Must be installed and running on your host machine.
2. **Monarch Model**: You must have the `monarch` model built and available in your local Ollama instance.
   ```bash
   ollama run monarch
   ```
3. **Docker & Docker Compose** (Optional, for containerized deployment)
4. **Python 3.10+** (Optional, for local/bare-metal deployment)

## Deployment Options

### Option 1: Run with Docker Compose (Recommended)

The easiest way to deploy the UI is using Docker Compose. The provided configuration automatically bridges the container to your host machine's Ollama instance (port `11434`).

1. Navigate to the `ui` directory:
   ```bash
   cd ui
   ```
2. Build and start the container in detached mode:
   ```bash
   docker-compose up -d --build
   ```
3. Access the UI in your browser at:
   [http://localhost:8501](http://localhost:8501)

To stop the container, run:
```bash
docker-compose down
```

### Option 2: Run Locally (Bare-metal)

If you prefer to run the Streamlit app directly on your host machine without Docker:

1. Navigate to the `ui` directory:
   ```bash
   cd ui
   ```
2. Create and activate a virtual environment (recommended):
   ```bash
   python -m venv venv
   source venv/bin/activate  # On Windows use `venv\Scripts\activate`
   ```
3. Install the dependencies:
   ```bash
   pip install -r requirements.txt
   ```
4. Run the Streamlit application:
   ```bash
   streamlit run app.py
   ```
5. Access the UI in your browser at the local URL provided in the terminal (usually [http://localhost:8501](http://localhost:8501)).

## Configuration

By default, the UI attempts to connect to Ollama at `http://localhost:11434/api/generate` and requests the model named `monarch`. 

If your Ollama instance is hosted elsewhere, or if you named your model differently, you can modify the following constants at the top of `app.py`:

```python
OLLAMA_API_URL = "http://localhost:11434/api/generate"
MODEL_NAME = "monarch" 
```

*(Note: When running via Docker Compose, Docker handles the `localhost` mapping to the host machine via `host.docker.internal`, so changes to the URL are usually unnecessary for standard local deployments).*