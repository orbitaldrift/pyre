run:
    docker compose -f etc/docker-compose.yml -p pyre-db up --build -d

create:
    sqlx database create

migrate:
    sqlx migrate run

prepare:
     cargo sqlx prepare --workspace