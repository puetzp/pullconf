# Group

As mentioned in the section explaining [client configuration](client.md), a client can be a member of multiple groups. The client inherits the list of resources defined in a group (in addition to those defined in its own configuration). A little information on the relationship between clients and groups can be found in the [concepts section](../concepts.md#groups).

In general groups serve the purpose of deduplicating resources that apply to multiple clients and define them in a common collection of resources. For instance when clients share the same set of entries in the `/etc/hosts` file, these entries could be defined inside a group using the [host](resources/host.md) resource type.

Thus groups are just a list of resources under a common name that tell a user what they produce.

Just like client configuration a group is a StrictYAML document inside a file in `$PULLCONF_RESOURCE_DIR` or one of its subdirectories. The same restrictions that are explained at the top of the [client](client.md) section apply here as well.

Other than that the document must adhere to the following structure in order to be correctly parsed as a group:

```yaml
---
# Tells pullconfd to parse this document as a group.
type: group

# A proper name that allows clients to refer to this group.
name: <...>

# A list of resources.
resources: [...]
```

- `name`:

  Group names must adhere to the same rules as client hostnames as described in the [client](client.md) section.

- `resources`:

  *Optional*: A list of [resources](resources/index.md). These resources will be added to the resource list of every client that is assigned this group and evaluated per-client (using the client's variables if any).

After the group configuration is created and every time it is modified a reload of **pullconfd** is required to read and re-evaluate the configuration.

```sh
sudo systemctl reload pullconfd.service
```

## Example

The following is a simple and short example of a group configuration file.

```yaml
---
type: group
name: localdomain

resources:
  - type: host
    parameters:
	  ensure: present
	  ip_address: 192.168.1.55
	  hostname: some-webserver
	  aliases:
	    - some-webserver.local

  - type: file
    parameters:
	  ensure: present
	  path: /etc/logrotate.d/apt
	  source: /common/logrotate.d/apt
```

