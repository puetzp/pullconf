# Server

The following steps guide you on how to install and configure the Pullconf server component **pullconfd**.

Download and install the .deb package from GitHub:

```sh
wget https://github.com/puetzp/pullconf/releases/download/v0.1.0/pullconfd_0.1.0-1_amd64.deb
```

```sh
sudo dpkg -i pullconfd_0.1.0_amd64.deb
```

The installation script sets up a systemd service unit and a configuration and data directory. Check the unit's status:

```sh
sudo systemctl status pullconfd.service
```

If this is your first installation the unit will likely be in the "failed" state, because some mandatory configuration parameters may need to be set up, such as a path to a valid TLS certificate and corresponding private key. Refer to the log at `/var/log/pullconfd/pullconfd.log` to see what else might be missing to start the unit.

**pullconfd** is configured via environment variables. As you can see in the systemd unit file the unit reads environment variables from `/etc/pullconfd/environment` (the required format is documented [here](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html#EnvironmentFile=). Refer to the following table for all available parameters.

| Name | Description | Mandatory | Default |
| --- | --- | --- | --- |
| PULLCONF_LISTEN_ON | The socket address which the server should bind to. | yes | `127.0.0.1:443` |
| PULLCONF_TLS_CERTIFICATE | Path to a TLS certificate file in PEM format. | yes | `/etc/pullconfd/tls/server.crt` |
| PULLCONF_TLS_PRIVATE_KEY | Path to the corresponding private key in PEM format. | yes | `/etc/pullconfd/tls/server.key` |
| PULLCONF_RESOURCE_DIR | Directory containing StrictYAML files that define clients, groups, and their resources. Only files ending with `.yaml` or `.yml` will be parsed. Subdirectories may be nested up to ten levels. | yes | `/etc/pullconfd/conf.d` |
| PULLCONF_ASSET_DIR | Directory containing static file assets. This directory is served by the web server to enable clients to download file contents. Subdirectories may be arbitrarily nested. | yes | `/etc/pullconfd/assets` |
| RUST_LOG | This variable is read by the underlying logging library. Check their [documentation](https://docs.rs/env_logger/latest/env_logger/index.html#enabling-logging) for a complete overview of valid values. | no | `info` |

After adding or changing environment variables you need to restart the unit:

```sh
sudo systemctl restart pullconfd.service
```

Systemd will then re-apply the settings from the environment file. Whenever environment variables are changed the unit must be restarted. However when files in <code>$PULLCONF_RESOURCE_DIR</code> change a reload will suffice to re-read StrictYAML files from this directory:

```sh
sudo systemctl reload pullconfd.service
```

Note that if the changed configuration cannot be successfully validated, the server will continue to operate with the old configuration.




