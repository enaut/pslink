# Pslink a "Private Short Link page"

The target audience of this tool are small entities that need a url shortener. The shortened urls can be publicly resolved but only registered users can create short urls. Every registered user can see all shorted urls but ownly modify its own. Admin users can invite other accounts and edit everything that can be edited (also urls created by other accounts).

So in general this is more a shared short url bookmark webpage than a shorturl service.

![Screenshot](./doc/img/pslinkscreenshot.png)

The Page comes with a basic commandline interface to setup the environment.

> This branch is under development the commands below are slightly different. To build use: `cargo make build` to use pslink use the `target/debug/pslink` file.

## Usage

### install binary

The pslink binary can be downloaded from the latest release at: https://github.com/enaut/pslink/releases

These binaries are selfcontained and should run on any linux 64bit system. Just put them where you like them to be and make them executable. A sample install might be:

```bash
# mkdir -p /opt/pslink
# wget -o /opt/pslink/pslink https://github.com/enaut/pslink/releases/latest/download/pslink.linux.64bit
# chmod +x /opt/pslink/pslink
```

You could now adjust your `PATH` or setup an alias or just call the binary with the full path `/opt/pslink/pslink`

### Install with cargo

Pslink can be compiled and installed with cargo. Setup cargo as guided here: https://doc.rust-lang.org/cargo/getting-started/installation.html

After that install pslink using:

```bash
$ cargo install pslink
```

If that succeeds you should now be able to call pslink.

### Build from source

When building manually with cargo you have to have a sqlite database present or build it in offline mode. So on your first build you will most likely need to call:

```bash
SQLX_OFFLINE=1 cargo run
# or
$ export SQLX_OFFLINE=1
$ cargo run
```

If pslink is built with `cargo build release --target=x86_64-unknown-linux-musl` everything is embedded and it should be portable to any 64bit linux system.
Templates and migrations are embedded in the binary so it should run standalone without anything extra.

### Setup

To get Pslink up and running use the commands in the following order:

1. `pslink generate-env`

    this will generate a `.env` file in the curent directory with the default settings. Edit this file to your liking. You can however skip this step and provide all the parameters via commandline or environmentvariable. It is **not** recommended to provide PSLINK_SECRET with commandline parameters as they can be read by every user on the system.
2. `pslink migrate-database`

    will create a sqlite database in the location specified.
3. `pslink create-admin`

    create an initial admin user. As the page has no "register" function this is required to do anything usefull.
4. `pslink runserver`

    If everything is set up correctly this command will start the service.

### Run the service

If everything is correctly set up just do `pslink runserver` to launch the server.

### Update

To update to a newer version execute the commands in the following order

1. stop the service
2. download and install the new binary
3. run `pslink migrate-database`
4. run the server again `pslink runserver`

### Help

For a list of options use `pslink help`. If the help does not provide enough clues please file an issue at: https://github.com/enaut/pslink/issues/new

### Systemd service file

If you want to automatically start this with systemd you can adjust the following template unit to your system. In this case a dedicated `pslink` user and group is used with the users home directory at `/var/pslink`. Some additional settings are in place to protect the system a little should anything go wrong.

```systemd
# /etc/systemd/system/pslink.service
[Unit]
Description=Pslink the Urlshortener
Documentation=https://github.com/enaut/Pslink
Wants=network.target
After=network.target

[Service]
User=pslink
Group=pslink
EnvironmentFile=-/var/pslink/.env

ProtectHome=true
ProtectSystem=full
PrivateDevices=true
NoNewPrivileges=true
PrivateTmp=true
InaccessibleDirectories=/root /sys /srv -/opt /media -/lost+found
ReadWriteDirectories=/var/pslink
WorkingDirectory=/var/pslink
ExecStart=/var/pslink/pslink runserver

[Install]
WantedBy=multi-user.target
```
