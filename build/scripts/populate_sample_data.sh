#!/bin/bash
# Populate sample data for Klyster dashboard

BASE_URL="${KLYSTER_URL:-http://localhost:7272}"

echo "Populating sample data to $BASE_URL..."

# Create Prometheus source
echo "Creating Prometheus source..."
curl -X POST "$BASE_URL/api/v1/sources" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Local Prometheus",
    "source_type": "prometheus",
    "config": {
      "url": "http://prometheus:9292",
      "timeout_secs": 30
    }
  }'

echo -e "\n\nCreating resource group..."
curl -X POST "$BASE_URL/api/v1/resource-groups" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production Cluster",
    "description": "Main production Kubernetes cluster",
    "provider_type": "kubernetes",
    "provider_config": {
      "cluster_name": "prod-k8s-01"
    }
  }'

echo -e "\n\nSample data populated successfully!"
echo "Visit the dashboard to see your data."
