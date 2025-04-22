# Run the pyre-telemetry docker container
run:
    docker compose -f telemetry/docker-compose.yml -p pyre-telemetry up -d --build