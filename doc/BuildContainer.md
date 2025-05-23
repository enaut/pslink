# Building and Running Pslink Server in a Container

This document describes how to build and run the Pslink server using containers.

## Prerequisites

- Docker or Podman installed on your system
- Rust toolchain (if building locally first)



## Running the Container

### Basic Run Command

This run command does start pslink with some demo data. The username and password are "demo". Be carful though as the data does **not** persist restarts

```bash
podman run -d --name pslink_container \
  -p 8080:8080 \
  ghcr.io/enaut/pslink:latest
```

When successfully started you can open http://localhost:8080/app/ to login.

### Run with Database and Environment File Persistence

For production use with persistent data, see the section [Database Setup](#database-setup-for-production-use) for instructions on generating the `.env` and `links.db` files.

To launch with persistent data run:

```bash
podman run -d --name pslink_container \
  -v ./links.db:/app/links.db:Z \
  -v ./.env:/app/.env:Z \
  -p 8080:8080 \
  ghcr.io/enaut/pslink:latest
```

Replace `./links.db` and `./.env` with absolute paths if necessary. If SElinux is not used the `:Z` parameters can be omitted.

### Configuration Options

When starting the container, you can specify command line arguments :

```bash
podman exec -it
  ghcr.io/enaut/pslink:latest /app/pslink --help
```

```
$ podman exec -it pslink_container /app/pslink --help
Usage: pslink [OPTIONS] [COMMAND]

Commands:
  runserver         Run the server
  create-admin      Create an admin user.
  generate-env      Generate an .env file template using default settings and exit
  migrate-database  Apply any pending migrations and exit
  help              Print this message or the help of the given subcommand(s)

Options:
      --db <database>                  The path of the sqlite database [env: PSLINK_DATABASE=/app/links.db] [default: ./links.db]
  -p, --port <port>                    The port the pslink service will run on [env: PSLINK_PORT=8080]
  -u, --public-url <public_url>        The host url or the page that will be part of the short urls. [env: PSLINK_PUBLIC_URL=localhost:8080] [default: 127.0.0.1:8080]
  -e, --empty-url <empty_forward_url>  The the url that / will redirect to. Usually your homepage. [env: PSLINK_EMPTY_FORWARD_URL=https://github.com/enaut/pslink] [default: https://github.com/enaut/pslink]
  -b, --brand-name <brand_name>        The Brandname that will apper in various places. [env: PSLINK_BRAND_NAME=Pslink] [default: Pslink]
  -i, --hostip <internal_ip>           The host (ip) that will run the pslink service [env: PSLINK_IP=localhost]
      --demo <demo>                    The host (ip) that will run the pslink service [env: DEMO=]
  -t, --protocol <protocol>            The protocol that is used in the qr-codes (http results in slightly smaller codes in some cases) [env: PSLINK_PROTOCOL=http] [default: http] [possible values: http, https]
      --secret <secret>                The secret that is used to encrypt the password database keep this as inaccessible as possible. As command line parameters are visible to all users it is not wise to use this as a command line parameter but rather as an environment variable. [env: PSLINK_SECRET=Slsgohetö<fgHSGHTRZAERTCNVbfoadhfgrziopüümbn,.] [default: ]
  -h, --help                           Print help
  -V, --version                        Print version
```

## Container Management

```bash
# View logs
podman logs pslink_container

# Stop the container
podman stop pslink_container

# Remove the container
podman rm pslink_container
```

## Database Setup for production use

To start with a fresh config and database intended for production use:

1. Navigate to the directory where you want to have the configuration and data files.
    ```bash
    mkdir ~/pslink-data
    cd ~/pslink-data
    ```

2. create empty files for mounting
    ```bash
    touch .env links.db
    ```
3. generate the .env-file contents with a default configuration
    ```bash
    podman run -it --name pslink_container \
      -v ./links.db:/app/links.db:Z \
      -v ./.env:/app/.env:Z \
      ghcr.io/enaut/pslink:latest /app/pslink generate-env
    ```
4. bring the database up to date
    ```bash
    podman run --replace -it --name  pslink_container \
      -v ./links.db:/app/links.db:Z \
      -v ./.env:/app/.env:Z \
      ghcr.io/enaut/pslink:latest /app/pslink migrate-database
    ```
5. start the container
    ```bash
    podman run --replace -d --name pslink_container \
      -v ./links.db:/app/links.db:Z \
      -v ./.env:/app/.env:Z \
      -p 8080:8080 \
      ghcr.io/enaut/pslink:latest
    ```
6. create an admin user
    ```bash
    podman exec -it pslink_container /app/pslink create-admin
    ```
7. change the DEMO env variable to false so that the warning hint is disabled.
    ```bash
    sed -i 's/DEMO="true"/DEMO="false"/' .env
    ```
8. restart the container
    ```bash
    podman restart pslink_container
    ```

## Building the Container Image

Navigate to the project root directory and build the image (podman and docker are interchangable use what you have installed):

```bash
# Using Podman
podman build -t pslink:latest .
```