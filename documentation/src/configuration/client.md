# Client

Once **pullconf** is installed on the client system a client configuration must be created on the server. This makes the client known to the server and **pullconf** can successfully connect to **pullconfd**.

Configuration files are parsed by **pullconfd** according to these rules:

- only files in `$PULLCONF_RESOURCE_DIR` are parsed.
- this directory may contain subdirectories that can be nested up to ten levels.
- in this directory only files ending with a `.yaml` or `.yml` extension are parsed, other files are ignored. This enables the usage of `git` or other version control software.
- files may contain one or more StrictYAML documents. Every document is expected to define either a client or a group.

For example one straightforward way to manage configuration files in a small environment is to create one file per client in a subdirectory such as `$PULLCONF_RESOURCE_DIR/clients`. The files in this directory could be named after the clients they define. Thus a client with the hostname `blechkiste.local` would be defined in the file `$PULLCONF_RESOURCE_DIR/clients/blechkiste.local`.

The following structure defines a client inside a StrictYAML document:

```yaml
---
# Tells pullconfd to parse this document as a client.
type: client

# A hostname such as "blechkiste.local".
name: <...>

# The sha256 hash of the client API key.
api_key: <...>

# A list of groups that the client is assigned to.
groups: [...]

# A hash of variables that can be used inside resource definitions.
variables: {...}

# A list of resources.
resources: [...]
```

- `name`:
 
  Client hostnames must adhere to these rules:
    - cannot be an empty string
    - cannot be more than 253 characters long
    - cannot start with a hyphen
    - cannot contain characters other than `[\-a-zA-Z0-9\.]`
    - cannot have segments that exceed 63 characters: a hostname `my.example.com` has three distinct segments, `my`, `example` and `com`

- `api_key`: 

  The API key corresponds to the sha256 hash of the environment variable `$PULLCONF_API_KEY` that is provided to **pullconf**. See [client installation](../installation/client.md) for more information.

- `groups`:

  *Optional*: A list of [group](group.md) names. By assigning the client to a group it inherits all resources from that group. Every group names in this array must match an existing group.

- `variables`:

  *Optional*: A hash of variables that are relevant in the context of this client. See the section on [variables](variables.md) on details how to use variables inside resource definitions.

- `resources`:

  *Optional*: A list of [resources](resources/index.md). Resources inherited from groups will be merged with resources listed here and then evaluated in the context of this client (using the variables above).

After defining at least the bare minimum (`type`, `name` and `api_key`) the file can be saved and the server reloaded:

```sh
sudo systemctl reload pullconfd.service
```

The next time the *pullconf.service* is excecuted on the client system, **pullconf** will successfully connect to **pullconfd**, since the server is now able to identify the client by means of its API key and hostname.

## Multiple documents per file

As mentioned earlier the StrictYAML parser parses every YAML document inside a configuration file. Thus it is also possible to define multiple clients in a single file using documents.

```yaml
---

type: client
name: blechkiste
api_key: 20b5094257d70c8d126cf278510b6443d5139e86e18be1389b90a28d526c8236
<...>

---

type: client
name: blechbuechse
api_key: e858777f14b11e7c2eb6f60032fbb6b943a54a2011b5ad43b3ad6f0c6557d2bb
<...>

---

type: client
<...>

```

## Example

The following is a simple and short example of a client configuration file.

```yaml
# $PULLCONF_RESOURCE_DIR/clients/blechkiste.local.yaml
---
type: client
name: blechkiste.local
api_key: 50d858e0985ecc7f60418aaf0cc5ab587f42c2570a884095a9e8ccacd0f6545c

groups:
  - linux
  - debian
  - nginx
  - ssh

variables:
  ip_address: 192.168.1.55
  logrotate-dir: /etc/logrotate.d

resources:
  - type: host
    parameters:
	  ensure: present
	  ip_address: ${pullconf::ip_address}
	  hostname: ${pullconf::hostname}
	  aliases:
	    - webserver
	    - webserver.local

  - type: file
    parameters:
	  ensure: present
	  path: ${pullconf::logrotate-dir}/apt
	  content:
	    value: |
          /var/log/apt/term.log {
            rotate 12
            monthly
            compress
            missingok
            notifempty
          }

          /var/log/apt/history.log {
            rotate 12
            monthly
            compress
            missingok
            notifempty
          }
```
