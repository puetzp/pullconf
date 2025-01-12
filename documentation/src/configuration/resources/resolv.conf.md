# resolv.conf

This resource manages the file contents of `/etc/resolv.conf`, the common resolver configuration file. There may only be one `resolv.conf` resource per client. For this reason `resolv.conf` does not need a primary parameter to be uniquely identifiable.

See `man resolv.conf` for more information.

## Relationship to other resources

If there is a [file](file.md) or [symlink](symlink.md) resource whose `path` parameter is `/etc/resolv.conf`, the resolv.conf resource depends on it implicitly. However there cannot be both a file resource managing the contents of `/etc/resolv.conf` and a resolv.conf resource. The `content` and `source` parameters of the file resource must be omitted in this case to avoid conflicting resource definitions.

## Parameters

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `ensure` | string | Determines the desired state of the resource. One of `present` or `absent`. | yes | `present` |
| `nameservers` | array | A list of IP addresses that each correspond to a name server. Array items must be strings. | no | |
| `search` | array | A list of domain names for hostname lookup. Array items must be strings. | no | |
| `sortlist` | array | A list of IP-address-netmask pairs where the netmask is optional and separated from the IP address by a slash. Array items must be strings. | no | |
| `options` | array | A list of resolver variables. Array items must be strings. Possible values are documented below. | no | |

Each item in `options` must be one of:

- `debug`
- `ndots:x` where x is a number between 0 and 15
- `timeout:x`where x is a number between 0 and 30
- `attempts:x` where x is a number between 0 and 5
- `rotate`
- `no-check-names`
- `inet6`
- `edns0`
- `single-request`
- `single-request-reopen`
- `no-tld-query`
- `use-vc`
- `no-reload`
- `trust-ad`

## Examples

```yaml
resources:
  - type: resolv.conf
    parameters:
      nameservers:
	    - 8.8.8.8
		- 4.4.4.4
	  search:
	    - domain.local
		- example.com
	  sortlist:
	    - 130.155.160.0/255.255.240.0
		- 130.155.0.0
	  options:
	    - ndots:2
		- timeout:5
		- inet6
```
