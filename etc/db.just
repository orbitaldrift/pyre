default: run wait create migrate

# Drop the database
reset:
    sqlx database drop

# Run the databases using docker-compose
run:
    docker compose -f deploy/docker-compose.yml -p pyre-db up -d --build

# Wait until the database is ready
wait:
    ./deploy/db-wait.sh

# Create the default database
create:
    sqlx database create

# Run migrations
[no-cd]
migrate:
    sqlx migrate run

# Prepare the SQLX query cache for development
prepare:
     cargo sqlx prepare --workspace