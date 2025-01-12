# Resources

Resources are the main building blocks of your configuration. They define which [files](file.md), [directories](directory.md) etc. are created and managed on the client.

Resources can be defined inside the `resources` array in [clients](../client.md) and [groups](../group.md).

Every resource is structured the same at the root and contains the following keys:

| Name | Description | Mandatory | Default |
| --- | --- | --- | --- |
| `type` | A string that determines how to parse this resource. Must match one of the supported resource type identifiers. See below for valid values. | yes | |
| `parameters` | A hash of resource-specific parameters. | yes | |
| `requires` | An array of dependencies between resources. See [dependencies](../dependencies.md). | no | `[]` |

Valid values for `type` are:

- `apt::package`
- `apt::preference`
- `cron::job`
- `directory`
- `file`
- `group`
- `host`
- `resolv.conf`
- `symlink`
- `user`

The keys inside `parameters` differ between resources, so the respective resource documentation section should be consulted to see the available resource parameters.

Also note that [variables](../variables.md) can only be used inside the `parameters` hash.

## Example

```yaml
<...>

resources:
  - type: directory
    parameters:
	  ensure: present
	  path: /path/to/directory
	  purge: true

  - type: file
    parameters:
	  ensure: present
	  path: /path/to/directory/somefile

  - type: file
    parameters:
	  ensure: present
	  path: /path/to/directory/otherfile
    requires:
	  - type: file
	    path: /path/to/directory/somefile
```

```yaml
<...>

# All kinds of requirements.
resources:
  - type: directory
    parameters:
	  ensure: present
	  path: /path/to/directory
    requires:
	  - type: apt::package
	    name: some-package

      - type: apt::preference
	    name: some-preference

      - type: cron::job
	    name: some-job

      - type: directory
	    path: /some/directory

      - type: file
	    path: /some/file
	  
	  - type: group
	    name: some-group
	  
	  - type: host
	    ip_address: 172.16.0.1
	  
	  - type: resolv.conf
	  
	  - type: symlink
	    path: /some/symlink
	  
	  - type: user
	    name: some-user
```
