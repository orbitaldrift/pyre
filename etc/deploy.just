# Run the databases using docker-compose
run:
    docker compose -f deploy/docker-compose.yml -p pyre-db up -d --build
