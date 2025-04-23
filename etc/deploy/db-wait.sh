#!/bin/bash
set -e

# Max number of attempts
MAX_ATTEMPTS=30
# Time to wait between attempts (in seconds)
WAIT_TIME=2

POSTGRES_CONTAINER="pyre-db-postgres"
REDIS_CONTAINER="pyre-db-redis"

echo "Waiting for PostgreSQL and Redis to be ready..."

# Check if PostgreSQL is ready
postgres_ready=false
attempt=1
while [ $attempt -le $MAX_ATTEMPTS ]; do
  echo "Checking PostgreSQL connection (attempt $attempt/$MAX_ATTEMPTS)..."
  if docker exec $(docker ps -q -f name=$POSTGRES_CONTAINER) pg_isready -U postgres > /dev/null 2>&1; then
    postgres_ready=true
    echo "PostgreSQL is ready!"
    break
  fi
  echo "PostgreSQL not ready yet. Waiting $WAIT_TIME seconds..."
  sleep $WAIT_TIME
  attempt=$((attempt+1))
done

if [ "$postgres_ready" != "true" ]; then
  echo "Error: PostgreSQL did not become ready within the allowed time"
  exit 1
fi

# Check if Redis is ready
redis_ready=false
attempt=1
while [ $attempt -le $MAX_ATTEMPTS ]; do
  echo "Checking Redis connection (attempt $attempt/$MAX_ATTEMPTS)..."
  if docker exec $(docker ps -q -f name=$REDIS_CONTAINER) redis-cli ping | grep -q "PONG"; then
    redis_ready=true
    echo "Redis is ready!"
    break
  fi
  echo "Redis not ready yet. Waiting $WAIT_TIME seconds..."
  sleep $WAIT_TIME
  attempt=$((attempt+1))
done

if [ "$redis_ready" != "true" ]; then
  echo "Error: Redis did not become ready within the allowed time"
  exit 1
fi

echo "All databases are ready!"
exit 0