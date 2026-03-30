#!/bin/bash
set -euo pipefail

echo "================================================"
echo "  NeuroGraph v${NEUROGRAPH_VERSION:-dev}"
echo "  Temporal Knowledge Graph Engine"
echo "================================================"
echo ""
echo "Configuration:"
echo "  Storage:    ${NEUROGRAPH_STORAGE:-embedded}"
echo "  API Port:   ${NEUROGRAPH_API_PORT:-8000}"
echo "  Dashboard:  ${NEUROGRAPH_DASHBOARD_PORT:-3000}"
echo "  Data Dir:   ${NEUROGRAPH_DATA_DIR:-/app/data}"
echo "  Log Level:  ${NEUROGRAPH_LOG_LEVEL:-info}"
echo ""

case "${1:-serve}" in
    serve)
        echo "Starting NeuroGraph server..."
        exec neurograph-server \
            --host "${NEUROGRAPH_HOST:-0.0.0.0}" \
            --port "${NEUROGRAPH_API_PORT:-8000}" \
            --dashboard-port "${NEUROGRAPH_DASHBOARD_PORT:-3000}" \
            --dashboard-dir "/app/dashboard" \
            --data-dir "${NEUROGRAPH_DATA_DIR:-/app/data}" \
            --log-level "${NEUROGRAPH_LOG_LEVEL:-info}"
        ;;
    api-only)
        echo "Starting NeuroGraph API server (no dashboard)..."
        exec neurograph-server \
            --host "${NEUROGRAPH_HOST:-0.0.0.0}" \
            --port "${NEUROGRAPH_API_PORT:-8000}" \
            --no-dashboard \
            --data-dir "${NEUROGRAPH_DATA_DIR:-/app/data}"
        ;;
    mcp)
        echo "Starting NeuroGraph MCP server..."
        exec neurograph-mcp \
            --host "${NEUROGRAPH_HOST:-0.0.0.0}" \
            --port "${NEUROGRAPH_MCP_PORT:-8001}"
        ;;
    cli)
        shift
        exec neurograph-cli "$@"
        ;;
    bench)
        shift
        exec neurograph-cli bench "$@"
        ;;
    migrate)
        echo "Running database migrations..."
        exec neurograph-cli migrate \
            --data-dir "${NEUROGRAPH_DATA_DIR:-/app/data}"
        ;;
    healthcheck)
        exec curl -sf http://localhost:${NEUROGRAPH_API_PORT:-8000}/health || exit 1
        ;;
    shell)
        exec /bin/bash
        ;;
    *)
        exec "$@"
        ;;
esac
