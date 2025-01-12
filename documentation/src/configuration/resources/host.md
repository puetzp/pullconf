# host

This resource manages a single entry/line in the file `/etc/hosts`.

## Relationship to other resources

Hosts (or host entries), as identified by the value of the `ip_address` parameter, must be unique. In other words there can only be one host entry per IP address.

If there is a [file](file.md) or [symlink](symlink.md) resource whose `path` parameter is `/etc/hosts`, all host resources depend on it implicitly. However it is not possible to manage the contents of the `/etc/hosts` file via a file resource and manage one or more host resources at the same time. The `content` and `source` parameters of the file resource must be omitted in this case to avoid conflicting resource definitions.

## Parameters

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `ensure` | string | Determines the desired state of the resource. One of `present` or `absent`. | yes | `present` |
| `ip_address` | string | *Primary parameter*: The IP address for this entry. Both IPv4 and IPv6 addresses are supported. | yes | |
| `hostname` | string | The canonical hostname associated with the IP address. | yes | |
| `aliases` | array | A list of host aliases associated with the IP address. Array items must be strings. | no | |

## Examples

```yaml
resources:
  - type: host
    parameters:
      ip_address: 172.16.0.2
	  hostname: my.example.com
```

```yaml
resources:
  - type: host
    parameters:
      ip_address: 172.16.0.2
	  hostname: my.example.com
	  aliases:
	    - my.example
		- example
```

