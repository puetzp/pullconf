# Variables

Variables are defined within the `variables` hash in a [client](client.md) configuration. Every resource has `parameters` (see [resources](resources/index.md)). Every parameter value from the `parameters` hash can be substituted with a variable.

Variables can be used in resource definitions by appending the prefix `$pullconf::` to the variable name. For instance if a client defined a variable `sshd_listen: 172.16.10.5`, a user can use this variable inside a resource definition by refering to it as `$pullconf::sshd_listen`.

Since clients inherit the resources from groups that they are assigned to, effectively merging them with their own list of resources, variable substitution applies to those resources as well. This allows a user to define a commonly used resource inside a group, but let it resolve differently based on the variables that are defined in each client:

```yaml
---

type: client
name: blechkiste
api_key: <...>

groups:
  - common

variables:
  ip_address: 172.16.10.5

---

type: client
name: blechbuechse
api_key: <...>

groups:
  - common

variables:
  ip_address: 172.16.10.10

---

type: group
name: common

resources:
  - type: host
    parameters:
	  ensure: present
	  ip_address: $pullconf::ip_address
	  hostname: $pullconf::hostname
```

Variables must resolve to the same type that a resource expects as if no variable substitution took place.

Using the example from above, if `$pullconf::ip_address` in either client declared an array instead of a string literal, parsing the [host](resources/host.md) resource type in the context of that client would result in an error, since [host](resources/host.md) expects a string literal for the `ip_address` parameter.

Also Pullconf fails to validate the configuration if a client is assigned to a group that contains resource definitions with variables that are not available in the context of this client.

Also note that variables can be nested up to a certain level. This allows users to use variables inside variables:

```yaml
---

type: client
name: blechkiste
api_key: <...>

variables:
  ip_address: 172.16.10.5
  alias: webserver
  another_alias: webserver.local
  aliases:
    - web
    - $pullconf::alias
	- $pullconf::another_alias

resources:
  - type: host
    parameters:
	  ensure: present
	  ip_address: $pullconf::ip_address
	  hostname: $pullconf::hostname
	  aliases: $pullconf::aliases
```

## Reserved variables

The following variables are reserved and can be used in resource definitions and other variables without having to declare them first:

| Name | Value | Example |
| --- | --- | --- |
| `hostname` | The hostname of the client. | `my.example.com` |

## Limitations

It is currently not possible to template values using variables. Variables can only be used to substitute the whole value of a given parameter. Partial substitution is not supported.

For example given a variable `myvar` with the string value `xyz`, the string `abc$pullconf::myvar` would not even be detected as a variable by **pullconfd**, much less substituted to `abcxyz`.
