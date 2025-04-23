# Generate a random key for the master key
[no-cd]
@master PATH="etc/keys/.master.key":
    touch {{PATH}}
    openssl rand -hex 64 | tr -d '\n' > {{PATH}}

# Generate and install a local certificate
local-cert:
    mkcert -cert-file local.pem -key-file local-key.pem localhost 127.0.0.1 ::1
    mkcert -install

    openssl x509 -in local.pem -inform PEM -out localhost/local.der -outform DER
    openssl pkcs8 -topk8 -in local-key.pem -inform PEM -out localhost/local-key.der -outform DER -nocrypt

    rm ./*.pem