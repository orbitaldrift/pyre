# Build a crate's docker image
build CRATE:
    cargo build -p {{CRATE}} -r
    docker build --build-arg BINARY={{CRATE}} -t {{CRATE}} .