#!/bin/bash
set -e

# Klyster Entrypoint Script
# Handles initialization and runs the application

# Default config if not provided
CONFIG_FILE="${CONFIG_FILE:-/app/klyster.toml}"

# Create data directory if it doesn't exist
mkdir -p /data

# If no config exists, create from example
if [ ! -f "$CONFIG_FILE" ]; then
    echo "No config found at $CONFIG_FILE, creating from example..."
    cp /app/klyster.example.toml "$CONFIG_FILE"
    
    # Update database path for Docker
    sed -i 's|sqlite_path = "./data/klyster.db"|sqlite_path = "/data/klyster.db"|' "$CONFIG_FILE"
    sed -i 's|host = "127.0.0.1"|host = "0.0.0.0"|' "$CONFIG_FILE"
fi

# Set log permissions
touch /var/log/klyster/app.log && chown klyster:klyster /var/log/klyster/app.log 2>/dev/null || true

# Run migrations on startup (optional - controlled by env var)
if [ "${AUTO_MIGRATE:-true}" = "true" ]; then
    echo "Running database migrations..."
    /app/klyster migrate --config "$CONFIG_FILE" || true
fi

# Execute the main application
exec /app/klyster "$@"