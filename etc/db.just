# Create the default database
create:
    sqlx database create

# Run migrations
migrate:
    sqlx migrate run

# Prepare the SQLX query cache for development
prepare:
     cargo sqlx prepare --workspace