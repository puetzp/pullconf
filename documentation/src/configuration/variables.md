# Variables

Variables can be used to substitute the *values of parameters* in the `parameters` hash of a resource (see [resources](resources/index.md)). Variables are defined in the `variables` hash in a [client](client.md) configuration. Common resource keys such as `type` and `requires` *cannot* be substituted by variables.

Variables can be used in resource definitions by wrapping the variable name with the prefix `${pullconf::` and a closing brace `}`. For instance if a client defined a variable `sshd_listen: 172.16.10.5`, a user can use this variable inside a resource definition by refering to it as `${pullconf::sshd_listen}`.

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
	  ip_address: ${pullconf::ip_address}
	  hostname: ${pullconf::hostname}
```

While variables can be nested (variables inside variables) they must eventually resolve to the same type that a resource expects as if no variable substitution took place.

Remember that StrictYAML knows three types:

* string literals
* arrays
* hashes

Whole arrays and hashes can only be substituted by variables that match their type. So the `aliases` parameter of a [host](resources/host.md) can be substituted by a variable that resolves to an array, while the `ip_address` parameter must resolve to a string literal.

Array members and hash values can themselves be variables. String literals (as opposed to arrays or hashes) are special in that they can be interspersed with variables at arbitrary positions within the string. Given a string literal `My name is ${pullconf::hostname}!` within the client configuration of a client named `blechkiste`, this string would resolve to `My name is blechkiste!`.

Indexing array and hash variables (e.g. `${pullconf::some-array[0]}`) to substitute a parameter with a single item from a collection is *not supported* (yet?).

Also note that Pullconf fails to validate the configuration if a client is assigned to a group that contains resource definitions with variables that are not available in the context of this client.

Here is an example of using variables inside variables:

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
    - ${pullconf::alias}
	- ${pullconf::another_alias}
	- ${pullconf::hostname}.local

resources:
  - type: host
    parameters:
	  ensure: present
	  ip_address: ${pullconf::ip_address}
	  hostname: ${pullconf::hostname}
	  aliases: ${pullconf::aliases}
```

## Reserved variables

The following variables are reserved and can be used in resource definitions and other variables without having to declare them first:

| Name | Value | Example |
| --- | --- | --- |
| `hostname` | The hostname of the client. | `my.example.com` |
